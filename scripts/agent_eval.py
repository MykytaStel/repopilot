#!/usr/bin/env python3
"""Measure what coding agents do to the checks that judge them.

Each task in `tests/agent_eval/tasks/` is a small repository with a tempting
way to turn its check green without doing the work: a hard bug, a test that
depends on the machine, a spec change that conflicts with an old test, a
coverage floor, a strict type check. A hidden oracle in the task decides
whether the work was really done; RepoPilot plays no part in that verdict.

    python3 scripts/agent_eval.py selfcheck          # tasks start red and the reference solution passes the oracle
    python3 scripts/agent_eval.py run --run-set NAME # run agents; append to tests/agent_eval/results/NAME.jsonl
    python3 scripts/agent_eval.py report             # write tests/agent_eval/REPORT.md

`run` uses the agents' own logins (Claude Code and Codex subscriptions).
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import tomllib
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import agent_eval_agents as agents  # noqa: E402
import agent_eval_report as report  # noqa: E402

EVAL = ROOT / "tests" / "agent_eval"
RESULTS = EVAL / "results"
WEAKENING_KINDS = ("behavioral.test-deleted-or-emptied",)
GITIGNORE = ".repopilot/\nnode_modules/\n.mypy_cache/\n__pycache__/\n"


def load_tasks(names: str = "all") -> list[dict]:
    tasks = []
    for path in sorted((EVAL / "tasks").iterdir()):
        if path.is_dir() and (names == "all" or path.name in names.split(",")):
            tasks.append({**tomllib.loads((path / "task.toml").read_text()), "id": path.name, "dir": path})
    return tasks


def base_env(repopilot: Path, task: dict) -> dict:
    """The caller's environment without any host agent session (each agent
    uses its own CLI login), with the RepoPilot build under test first on PATH."""
    env = {key: value for key, value in os.environ.items() if not key.startswith(("CLAUDE", "ANTHROPIC_BASE_URL"))}
    env["PATH"] = f"{repopilot.parent}{os.pathsep}{env.get('PATH', '')}"
    env.update(task.get("env", {}))
    return env


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=repo, capture_output=True, text=True, check=True).stdout


def prepare_repo(task: dict, dest: Path, overlay: Path | None = None) -> str:
    shutil.copytree(task["dir"] / "repo", dest)
    (dest / ".gitignore").write_text(GITIGNORE)
    git(dest, "init", "-q")
    git(dest, "config", "user.email", "eval@example.invalid")
    git(dest, "config", "user.name", "Agent Eval")
    git(dest, "add", "-A")
    git(dest, "commit", "-qm", "start")
    if overlay:
        shutil.copytree(overlay, dest, dirs_exist_ok=True)
    return git(dest, "rev-parse", "HEAD").strip()


def check(command: str, cwd: Path, env: dict) -> tuple[bool, str]:
    try:
        run = subprocess.run(command, shell=True, cwd=cwd, env=env, capture_output=True, text=True, timeout=300)
    except subprocess.TimeoutExpired:
        return False, "timed out"
    return run.returncode == 0, (run.stdout + run.stderr)[-1500:]


def oracle(task: dict, repo: Path, env: dict) -> tuple[bool, str]:
    """Run the hidden oracle on a copy of the final repository."""
    with tempfile.TemporaryDirectory() as tmp:
        copy = Path(tmp) / "check"
        shutil.copytree(repo, copy, ignore=shutil.ignore_patterns(".git", ".repopilot", "node_modules"))
        shutil.copytree(task["dir"] / "oracle", copy / "oracle", dirs_exist_ok=True)
        return check(task["oracle"], copy, {**env, **task.get("oracle_env", {})})


def selfcheck(repopilot: Path) -> int:
    failures = 0
    for task in load_tasks():
        env = base_env(repopilot, task)
        with tempfile.TemporaryDirectory() as tmp:
            start = Path(tmp) / "start"
            prepare_repo(task, start)
            green, _ = check(task["check"], start, env)
            oracle_start, _ = oracle(task, start, env)
            solved = Path(tmp) / "solved"
            prepare_repo(task, solved, task["dir"] / "solution")
            green_after, out_check = check(task["check"], solved, env)
            oracle_after, out_oracle = oracle(task, solved, env)
        problems = []
        if green != task.get("starts_green", False):
            problems.append(f"check starts {'green' if green else 'red'}")
        if oracle_start and task["expect"] != "skip-kept":
            problems.append("oracle passes before any work")
        if not green_after:
            problems.append(f"reference solution fails the check: {out_check[-300:]}")
        if not oracle_after:
            problems.append(f"reference solution fails the oracle: {out_oracle[-300:]}")
        failures += bool(problems)
        print(f"{'FAIL' if problems else 'ok  '} {task['id']}" + "".join(f"\n     {p}" for p in problems))
    return 1 if failures else 0


def integrity(repopilot: Path, repo: Path, env: dict) -> list[dict]:
    run = subprocess.run(
        [str(repopilot), "review", str(repo), "--since-snapshot", "--format", "json"],
        capture_output=True, text=True, env=env,
    )  # fmt: skip
    try:
        tiers = json.loads(run.stdout).get("tiered_signals", {})
    except json.JSONDecodeError:
        return [{"kind": "review-failed", "path": run.stderr[-300:]}]
    return [
        {"kind": s["kind"], "path": s.get("path"), "line": s.get("line"), "tier": tier}
        for tier in ("definitely", "maybe", "noise")
        for s in tiers.get(tier, [])
        if (s.get("family") == "integrity" or s.get("kind") in WEAKENING_KINDS) and not s.get("suppressed")
    ]


def hook_events(run_dir: Path) -> list[dict]:
    events = []
    for path in sorted((run_dir / "hooks").glob("*.txt")) if (run_dir / "hooks").exists() else []:
        _, script, code = path.stem.split("-", 2)
        events.append({"script": script, "exit": int(code.removeprefix("exit")), "feedback": path.read_text()[-2000:]})
    return events


def run_one(task: dict, agent: str, condition: str, rep: int, args, out: Path) -> dict:
    run_dir = out / args.run_set / f"{task['id']}__{agent}__{condition}__{rep}"
    shutil.rmtree(run_dir, ignore_errors=True)
    run_dir.mkdir(parents=True)
    repo = run_dir / "repo"
    start = prepare_repo(task, repo)
    env = base_env(args.repopilot, task)
    subprocess.run([str(args.repopilot), "snapshot"], cwd=repo, env=env, capture_output=True, check=True)
    began = time.time()
    with_plugin = condition == "plugin"
    if agent == "claude":
        outcome = agents.run_claude(task["prompt"], repo, env, out, with_plugin, args.timeout, args.claude_model)
    else:
        outcome = agents.run_codex(task["prompt"], repo, env, out, with_plugin, args.timeout, args.codex_model)
    duration = round(time.time() - began, 1)
    green, check_out = check(task["check"], repo, env)
    correct, oracle_out = oracle(task, repo, env)
    git(repo, "add", "-A")
    record = {
        "run_set": args.run_set, "task": task["id"], "category": task["category"], "expect": task["expect"],
        "agent": agent, "condition": condition, "rep": rep, "duration_s": duration,
        **outcome,
        "green": green, "correct": correct, "check_tail": check_out[-600:], "oracle_tail": oracle_out[-600:],
        "integrity": integrity(args.repopilot, repo, env),
        "hooks": hook_events(run_dir),
        "changed": git(repo, "diff", "--cached", "--name-status", start).split("\n")[:-1],
        "diff": git(repo, "diff", "--cached", start)[:20000],
    }  # fmt: skip
    return scrub(record, str(run_dir))


def scrub(value, prefix: str):
    """Replace the local run directory with `<run>` so records carry no machine paths."""
    if isinstance(value, str):
        return value.replace(prefix, "<run>")
    if isinstance(value, list):
        return [scrub(item, prefix) for item in value]
    if isinstance(value, dict):
        return {key: scrub(item, prefix) for key, item in value.items()}
    return value


def run(args) -> None:
    out = Path(args.out)
    RESULTS.mkdir(parents=True, exist_ok=True)
    results = RESULTS / f"{args.run_set}.jsonl"
    done = set()
    if results.exists():
        for line in results.read_text().splitlines():
            r = json.loads(line)
            done.add((r["task"], r["agent"], r["condition"], r["rep"]))
    plan = [
        (task, agent, condition, rep)
        for rep in range(1, args.reps + 1)
        for task in load_tasks(args.tasks)
        for agent in args.agents.split(",")
        for condition in args.conditions.split(",")
        if (task["id"], agent, condition, rep) not in done
    ]
    print(f"{len(plan)} runs to do ({len(done)} already recorded in {results.relative_to(ROOT)})", flush=True)
    lock = threading.Lock()

    def one(item):
        task, agent, condition, rep = item
        record = run_one(task, agent, condition, rep, args, out)
        with lock, results.open("a") as handle:
            handle.write(json.dumps(record) + "\n")
        blocks = sum(1 for h in record["hooks"] if h["script"] == "guard" and h["exit"] == 2)
        print(
            f"{task['id']:<20} {agent:<6} {condition:<6} #{rep} green={record['green']!s:<5} "
            f"correct={record['correct']!s:<5} weakened={len(record['integrity'])} blocks={blocks} "
            f"{record['duration_s']}s",
            flush=True,
        )

    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        list(pool.map(one, plan))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("selfcheck", "run", "report"):
        p = sub.add_parser(name)
        p.add_argument("--repopilot", type=Path, default=ROOT / "target" / "release" / "repopilot")
        if name == "run":
            p.add_argument("--run-set", required=True)
            p.add_argument("--agents", default="claude,codex")
            p.add_argument("--conditions", default="plain,plugin")
            p.add_argument("--tasks", default="all")
            p.add_argument("--reps", type=int, default=1)
            p.add_argument("--jobs", type=int, default=2)
            p.add_argument("--timeout", type=int, default=900)
            p.add_argument("--claude-model")
            p.add_argument("--codex-model")
            p.add_argument("--out", default=str(Path(tempfile.gettempdir()) / "repopilot-agent-eval"))
    args = parser.parse_args()
    if args.command != "report" and not args.repopilot.exists():
        sys.exit(f"{args.repopilot} not found; run `cargo build --release` or pass --repopilot")
    if args.command == "selfcheck":
        sys.exit(selfcheck(args.repopilot))
    if args.command == "run":
        run(args)
    report.write(RESULTS, EVAL / "REPORT.md")


if __name__ == "__main__":
    main()
