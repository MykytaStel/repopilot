use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ModuleKind {
    Feature,
    Shared,
    Infrastructure,
    Domain,
    Ui,
    Cli,
    Unknown,
}
