#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactSource {
    Filesystem,
    TextHeuristic,
    Ast,
    ImportGraph,
    PackageManifest,
    ConfigFile,
    GitDiff,
    ExternalTool,
    Mixed,
}
