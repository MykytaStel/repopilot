"""Agent adapters for the agent eval: Claude Code and Codex, each run once,
non-interactively, in a throwaway repository, with or without the RepoPilot
plugin.

The plugin under test is a copy of `integrations/claude-code/repopilot` whose
two hook scripts are wrapped, so every hook call leaves a file next to the
repository (`hooks/NNN-guard-exit2.txt`) with the feedback it gave. The
wrapped scripts still run the shipped ones unchanged.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import threading
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PLUGIN = ROOT / "integrations" / "claude-code" / "repopilot"
MARKETPLACE = "repopilot-eval"
SETUP = threading.Lock()  # parallel runs share the plugin copy and Codex homes

WRAPPER = """#!/usr/bin/env sh
# Agent eval: run the shipped NAME hook unchanged and keep what it decided.
here=$(cd "$(dirname "$0")" && pwd)
input=$(cat)
err=$(mktemp)
printf '%s' "$input" | sh "$here/NAME.real.sh" 2>"$err"
code=$?
root=$(git rev-parse --show-toplevel 2>/dev/null)
if [ -n "$root" ]; then
  log="$root/../hooks"
  mkdir -p "$log"
  n=$(ls "$log" | wc -l | tr -d ' ')
  cp "$err" "$log/$(printf '%03d' "$n")-NAME-exit$code.txt"
fi
cat "$err" >&2
rm -f "$err"
exit $code
"""


def prepare_plugin(out: Path) -> Path:
    """A local marketplace holding the wrapped plugin; returns its root."""
    market = out / "plugin-market"
    with SETUP:
        if not market.exists():
            _copy_plugin(market)
    return market


def _copy_plugin(market: Path) -> None:
    plugin = market / "repopilot"
    shutil.copytree(PLUGIN, plugin)
    for name in ("snapshot", "guard"):
        script = plugin / "scripts" / f"{name}.sh"
        script.rename(plugin / "scripts" / f"{name}.real.sh")
        script.write_text(WRAPPER.replace("NAME", name))
        script.chmod(0o755)
    manifest = {
        "name": MARKETPLACE,
        "owner": {"name": "RepoPilot agent eval"},
        "plugins": [{"name": "repopilot", "source": "./repopilot", "description": "RepoPilot (eval copy)"}],
    }
    (market / ".claude-plugin").mkdir()
    (market / ".claude-plugin" / "marketplace.json").write_text(json.dumps(manifest, indent=2))


def _codex_home(out: Path, market: Path, with_plugin: bool, env: dict) -> Path:
    """A private CODEX_HOME per condition, sharing only the user's login."""
    home = out / "homes" / ("codex-plugin" if with_plugin else "codex-plain")
    with SETUP:
        if not home.exists():
            _make_codex_home(home, market, with_plugin, env)
    return home


def _make_codex_home(home: Path, market: Path, with_plugin: bool, env: dict) -> None:
    home.mkdir(parents=True)
    (home / "auth.json").symlink_to(Path.home() / ".codex" / "auth.json")
    if with_plugin:
        codex_env = {**env, "CODEX_HOME": str(home)}
        for args in (["marketplace", "add", str(market)], ["add", f"repopilot@{MARKETPLACE}"]):
            subprocess.run(["codex", "plugin", *args], env=codex_env, check=True, capture_output=True)


def _codex_model(home: Path, thread_id: str | None) -> str:
    """The model Codex actually used, from the session log it wrote."""
    if thread_id:
        for log in (home / "sessions").rglob(f"*{thread_id}.jsonl"):
            for line in log.read_text().splitlines():
                if '"model"' in line:
                    found = json.loads(line).get("payload", {}).get("model")
                    if found:
                        return found
    return "codex default"


def run_codex(prompt: str, repo: Path, env: dict, out: Path, with_plugin: bool, timeout: int, model: str | None) -> dict:
    market = prepare_plugin(out)
    home = _codex_home(out, market, with_plugin, env)
    cmd = ["codex", "exec", "--json", "-s", "workspace-write", "--skip-git-repo-check", "-C", str(repo)]
    # The account's curated plugins (skills such as a brainstorming flow that
    # waits for approval) sync into any CODEX_HOME at startup, racily. Keep
    # them out so only RepoPilot differs between conditions.
    cmd += ["-c", "features.remote_plugin=false", "-c", "features.apps=false"]
    if with_plugin:
        cmd.append("--dangerously-bypass-hook-trust")
    if model:
        cmd += ["-m", model]
    cmd.append(prompt)
    proc, timed_out = _run(cmd, repo, {**env, "CODEX_HOME": str(home)}, timeout)
    messages, usage, errors, thread_id = [], {}, [], None
    for line in proc.stdout.splitlines() if proc else []:
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        item = event.get("item", {})
        if event.get("type") == "thread.started":
            thread_id = event.get("thread_id")
        elif event.get("type") == "item.completed" and item.get("type") == "agent_message":
            messages.append(item.get("text", ""))
        elif event.get("type") == "turn.completed":
            for key, value in event.get("usage", {}).items():
                usage[key] = usage.get(key, 0) + value
        elif event.get("type") in ("turn.failed", "error"):
            errors.append(json.dumps(event)[:500])
    return {
        "model": model or _codex_model(home, thread_id),
        "final_message": messages[-1] if messages else "",
        "usage": usage,
        "cost_usd": None,
        "errors": errors,
        "timed_out": timed_out,
        "exit_code": proc.returncode if proc else None,
    }


def run_claude(prompt: str, repo: Path, env: dict, out: Path, with_plugin: bool, timeout: int, model: str | None) -> dict:
    market = prepare_plugin(out)
    sandbox = {"sandbox": {"enabled": True, "autoAllowBashIfSandboxed": True}}
    servers = json.loads((PLUGIN / ".mcp.json").read_text()) if with_plugin else {"mcpServers": {}}
    cmd = [
        "claude", "-p", prompt,
        "--output-format", "stream-json", "--verbose",
        "--setting-sources", "project,local",
        "--settings", json.dumps(sandbox),
        "--strict-mcp-config", "--mcp-config", json.dumps(servers),
        "--permission-mode", "acceptEdits",
        "--no-session-persistence",
    ]  # fmt: skip
    if with_plugin:
        cmd += ["--plugin-dir", str(market / "repopilot")]
    if model:
        cmd += ["--model", model]
    proc, timed_out = _run(cmd, repo, env, timeout)
    init, result = {}, {}
    for line in proc.stdout.splitlines() if proc else []:
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if event.get("type") == "system" and event.get("subtype") == "init":
            init = event
        elif event.get("type") == "result":
            result = event
    return {
        "model": init.get("model", model or "claude default"),
        "plugins": [plugin.get("name") for plugin in init.get("plugins", [])],
        "final_message": result.get("result", ""),
        "usage": result.get("usage", {}),
        "cost_usd": result.get("total_cost_usd"),
        "turns": result.get("num_turns"),
        "permission_denials": len(result.get("permission_denials", [])),
        "errors": [result.get("subtype")] if result.get("is_error") else [],
        "timed_out": timed_out,
        "exit_code": proc.returncode if proc else None,
    }


def _run(cmd: list[str], cwd: Path, env: dict, timeout: int):
    """Run the agent; keep its raw event stream next to the repository."""
    try:
        proc = subprocess.run(
            cmd, cwd=cwd, env=env, stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=timeout
        )
    except subprocess.TimeoutExpired:
        return None, True
    (cwd.parent / "agent.jsonl").write_text(proc.stdout)
    (cwd.parent / "agent.stderr").write_text(proc.stderr)
    return proc, False
