// A wide but flat dispatcher: many arms, no nesting. Cognitive score is ~1
// (the switch itself), so it is not flagged — even though a file-level branch
// count (`code-quality.complex-file`) would call this "complex".
export function dispatch(kind: string): string {
  switch (kind) {
    case "create":
      return "created";
    case "read":
      return "read";
    case "update":
      return "updated";
    case "delete":
      return "deleted";
    case "list":
      return "listed";
    case "count":
      return "counted";
    case "search":
      return "searched";
    case "export":
      return "exported";
    case "import":
      return "imported";
    case "archive":
      return "archived";
    case "restore":
      return "restored";
    default:
      return "unknown";
  }
}
