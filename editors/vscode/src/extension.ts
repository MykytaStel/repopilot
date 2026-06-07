import * as vscode from "vscode";
import { spawn } from "node:child_process";
import * as path from "node:path";

type Evidence = {
  path: string;
  line_start: number;
  line_end?: number;
};

type Finding = {
  rule_id: string;
  title: string;
  description: string;
  severity: string;
  evidence: Evidence[];
  in_diff?: boolean;
};

type ReviewSignal = {
  kind: string;
  headline: string;
  detail?: string;
  path: string;
  line_start?: number;
  line_end?: number;
  tier: string;
  suppressed: boolean;
};

type RepoPilotReport = {
  findings?: Finding[];
  tiered_signals?: {
    definitely: ReviewSignal[];
    maybe: ReviewSignal[];
    noise: ReviewSignal[];
  };
  review?: {
    in_diff_findings: number;
    tiered_signals: { total: number };
  };
};

let diagnostics: vscode.DiagnosticCollection;
let status: vscode.StatusBarItem;
let lastReport = "";
const DIAGNOSTIC_DETAIL_LIMIT = 20;

export function activate(context: vscode.ExtensionContext): void {
  diagnostics = vscode.languages.createDiagnosticCollection("repopilot");
  status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 50);
  status.command = "repopilot.openReport";
  status.text = "$(shield) RepoPilot";
  status.show();

  context.subscriptions.push(
    diagnostics,
    status,
    vscode.commands.registerCommand("repopilot.scanWorkspace", () => runScan(context)),
    vscode.commands.registerCommand("repopilot.reviewChanges", () => runReview(context)),
    vscode.commands.registerCommand("repopilot.refreshDiagnostics", () => runReview(context)),
    vscode.commands.registerCommand("repopilot.explainCurrentFile", () => explainCurrentFile(context)),
    vscode.commands.registerCommand("repopilot.openReport", () => openReport(context)),
  );

  registerMcpProvider(context);
}

export function deactivate(): void {
  diagnostics?.dispose();
  status?.dispose();
}

async function runScan(context: vscode.ExtensionContext): Promise<void> {
  const report = await runJson(context, ["scan", ".", "--format", "json", "--no-progress"]);
  publishDiagnostics(report);
  lastReport = await runText(context, ["scan", ".", "--format", "markdown", "--no-progress"]);
  status.text = `$(shield) RepoPilot: ${report.findings?.length ?? 0} finding(s)`;
}

async function runReview(context: vscode.ExtensionContext): Promise<void> {
  const args = ["review", ".", "--format", "json", "--no-progress"];
  const base = vscode.workspace.getConfiguration("repopilot").get<string>("reviewBase", "").trim();
  if (base) {
    args.push("--base", base);
  }
  const report = await runJson(context, args);
  publishDiagnostics(report);
  lastReport = await runText(
    context,
    args.map((arg) => (arg === "json" ? "markdown" : arg)),
  );
  const findings = report.review?.in_diff_findings ?? report.findings?.length ?? 0;
  const signals = report.review?.tiered_signals.total ?? 0;
  status.text = `$(shield) RepoPilot: ${findings} finding(s), ${signals} signal(s)`;
}

async function explainCurrentFile(context: vscode.ExtensionContext): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  const root = workspaceRoot();
  if (!editor || !root) {
    throw new Error("Open a workspace file first.");
  }
  const relative = path.relative(root.fsPath, editor.document.uri.fsPath);
  const explanation = await runText(context, [
    "inspect",
    "explain",
    relative,
    "--format",
    "markdown",
  ]);
  await showMarkdown(explanation);
}

async function openReport(context: vscode.ExtensionContext): Promise<void> {
  if (!lastReport) {
    await runReview(context);
  }
  await showMarkdown(lastReport);
}

async function showMarkdown(content: string): Promise<void> {
  const document = await vscode.workspace.openTextDocument({
    language: "markdown",
    content,
  });
  await vscode.window.showTextDocument(document, { preview: true });
}

function publishDiagnostics(report: RepoPilotReport): void {
  diagnostics.clear();
  const grouped = new Map<string, vscode.Diagnostic[]>();
  let remaining = DIAGNOSTIC_DETAIL_LIMIT;

  for (const finding of report.findings ?? []) {
    if (remaining === 0) break;
    const evidence = finding.evidence?.[0];
    if (!evidence) continue;
    addDiagnostic(grouped, evidence.path, evidence.line_start, evidence.line_end, {
      message: `${finding.title}: ${finding.description}`,
      source: "RepoPilot",
      code: finding.rule_id,
      severity: findingSeverity(finding.severity),
    });
    remaining -= 1;
  }

  const signals = report.tiered_signals;
  for (const signal of [...(signals?.definitely ?? []), ...(signals?.maybe ?? [])]) {
    if (remaining === 0) break;
    if (signal.suppressed || !signal.path) continue;
    addDiagnostic(grouped, signal.path, signal.line_start ?? 1, signal.line_end, {
      message: signal.detail ? `${signal.headline}: ${signal.detail}` : signal.headline,
      source: "RepoPilot Review",
      code: signal.kind,
      severity:
        signal.tier === "definitely-sensitive"
          ? vscode.DiagnosticSeverity.Warning
          : vscode.DiagnosticSeverity.Information,
    });
    remaining -= 1;
  }

  const root = workspaceRoot();
  if (!root) return;
  for (const [relative, entries] of grouped) {
    diagnostics.set(vscode.Uri.joinPath(root, relative), entries);
  }
}

function addDiagnostic(
  grouped: Map<string, vscode.Diagnostic[]>,
  file: string,
  startLine: number,
  endLine: number | undefined,
  input: {
    message: string;
    source: string;
    code: string;
    severity: vscode.DiagnosticSeverity;
  },
): void {
  const start = Math.max(0, startLine - 1);
  const end = Math.max(start, (endLine ?? startLine) - 1);
  const diagnostic = new vscode.Diagnostic(
    new vscode.Range(start, 0, end, Number.MAX_SAFE_INTEGER),
    input.message,
    input.severity,
  );
  diagnostic.source = input.source;
  diagnostic.code = input.code;
  const normalized = file.replace(/\\/g, "/").replace(/^\.\//, "");
  grouped.set(normalized, [...(grouped.get(normalized) ?? []), diagnostic]);
}

function findingSeverity(severity: string): vscode.DiagnosticSeverity {
  switch (severity) {
    case "CRITICAL":
    case "HIGH":
      return vscode.DiagnosticSeverity.Error;
    case "MEDIUM":
      return vscode.DiagnosticSeverity.Warning;
    default:
      return vscode.DiagnosticSeverity.Information;
  }
}

async function runJson(
  context: vscode.ExtensionContext,
  args: string[],
): Promise<RepoPilotReport> {
  const text = await runText(context, args);
  return JSON.parse(text) as RepoPilotReport;
}

async function runText(context: vscode.ExtensionContext, args: string[]): Promise<string> {
  if (!vscode.workspace.isTrusted) {
    throw new Error("RepoPilot is disabled in untrusted workspaces.");
  }
  const root = workspaceRoot();
  if (!root) {
    throw new Error("Open a workspace folder first.");
  }
  const binary = binaryPath(context);
  return vscode.window.withProgress(
    { location: vscode.ProgressLocation.Window, title: "RepoPilot" },
    () =>
      new Promise<string>((resolve, reject) => {
        const child = spawn(binary, args, { cwd: root.fsPath, windowsHide: true });
        let stdout = "";
        let stderr = "";
        child.stdout.setEncoding("utf8").on("data", (chunk) => (stdout += chunk));
        child.stderr.setEncoding("utf8").on("data", (chunk) => (stderr += chunk));
        child.on("error", reject);
        child.on("close", (code) => {
          if (code === 0) resolve(stdout);
          else reject(new Error(stderr.trim() || `RepoPilot exited with code ${code}`));
        });
      }),
  );
}

function workspaceRoot(): vscode.Uri | undefined {
  return vscode.workspace.workspaceFolders?.[0]?.uri;
}

function binaryPath(context: vscode.ExtensionContext): string {
  const configured = vscode.workspace
    .getConfiguration("repopilot")
    .get<string>("binaryPath", "")
    .trim();
  if (configured) return configured;
  return context.asAbsolutePath(path.join("bin", process.platform === "win32" ? "repopilot.exe" : "repopilot"));
}

function registerMcpProvider(context: vscode.ExtensionContext): void {
  const api = vscode.lm as unknown as {
    registerMcpServerDefinitionProvider?: (
      id: string,
      provider: { provideMcpServerDefinitions(): unknown[] },
    ) => vscode.Disposable;
  };
  const StdioDefinition = (vscode as unknown as Record<string, unknown>)
    .McpStdioServerDefinition as
    | (new (label: string, command: string, args: string[]) => unknown)
    | undefined;
  if (!api.registerMcpServerDefinitionProvider || !StdioDefinition) return;
  const root = workspaceRoot();
  if (!root) return;
  context.subscriptions.push(
    api.registerMcpServerDefinitionProvider("repopilot", {
      provideMcpServerDefinitions: () => [
        new StdioDefinition("RepoPilot", binaryPath(context), ["mcp", "--root", root.fsPath]),
      ],
    }),
  );
}
