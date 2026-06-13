// A flat constants module: many lines, almost no branching, so the branch
// density stays well under the threshold and the file is not flagged.
const STATUS = {
  CREATED: "created",
  READ: "read",
  UPDATED: "updated",
  DELETED: "deleted",
  LISTED: "listed",
  COUNTED: "counted",
  SEARCHED: "searched",
  EXPORTED: "exported",
  ARCHIVED: "archived",
  RESTORED: "restored",
};

function label(key) {
  return STATUS[key] || "unknown";
}

module.exports = { STATUS, label };
