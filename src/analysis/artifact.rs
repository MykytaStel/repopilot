use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ArtifactOrigin {
    #[default]
    Source,
    ParsedCacheV2,
    LegacyCache,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SyntaxSummary {
    pub parsed: bool,
    pub root_kind: Option<String>,
    pub has_errors: bool,
    pub named_child_count: usize,
}

impl SyntaxSummary {
    pub fn unavailable() -> Self {
        Self::default()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RoleEvidenceFact {
    pub role: String,
    pub source: String,
    pub reason: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FileContextFacts {
    pub roles: Vec<String>,
    pub role_evidence: Vec<RoleEvidenceFact>,
    pub frameworks: Vec<String>,
    pub runtimes: Vec<String>,
    pub paradigms: Vec<String>,
    pub is_test: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ParsedArtifact {
    pub path: PathBuf,
    pub language: Option<String>,
    pub imports: Vec<String>,
    pub deferred_imports: Vec<String>,
    pub exports: Vec<String>,
    pub context: FileContextFacts,
    pub syntax: SyntaxSummary,
    pub origin: ArtifactOrigin,
}

impl ParsedArtifact {
    pub fn from_source(
        path: PathBuf,
        language: Option<String>,
        imports: Vec<String>,
        deferred_imports: Vec<String>,
        exports: Vec<String>,
        context: FileContextFacts,
        syntax: SyntaxSummary,
    ) -> Self {
        Self {
            path,
            language,
            imports,
            deferred_imports,
            exports,
            context,
            syntax,
            origin: ArtifactOrigin::Source,
        }
    }

    pub fn from_legacy_cache(
        path: PathBuf,
        language: Option<String>,
        imports: Vec<String>,
        deferred_imports: Vec<String>,
        context: FileContextFacts,
    ) -> Self {
        Self {
            path,
            language,
            imports,
            deferred_imports,
            exports: Vec::new(),
            context,
            syntax: SyntaxSummary::unavailable(),
            origin: ArtifactOrigin::LegacyCache,
        }
    }

    pub fn from_parsed_cache_v2(
        path: PathBuf,
        language: Option<String>,
        imports: Vec<String>,
        deferred_imports: Vec<String>,
        exports: Vec<String>,
        context: FileContextFacts,
        syntax: SyntaxSummary,
    ) -> Self {
        Self {
            path,
            language,
            imports,
            deferred_imports,
            exports,
            context,
            syntax,
            origin: ArtifactOrigin::ParsedCacheV2,
        }
    }

    pub fn rebase_path(&mut self, path: PathBuf) {
        self.path = path;
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_source_origin(&self) -> bool {
        self.origin == ArtifactOrigin::Source
    }

    pub fn has_complete_parsed_facts(&self) -> bool {
        matches!(
            self.origin,
            ArtifactOrigin::Source | ArtifactOrigin::ParsedCacheV2
        )
    }

    pub fn is_complete_source_artifact(&self) -> bool {
        self.is_source_origin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_cache_artifact_is_explicitly_incomplete() {
        let artifact = ParsedArtifact::from_legacy_cache(
            PathBuf::from("src/lib.rs"),
            Some("Rust".to_string()),
            vec!["crate::facts".to_string()],
            Vec::new(),
            FileContextFacts::default(),
        );

        assert_eq!(artifact.origin, ArtifactOrigin::LegacyCache);
        assert!(!artifact.is_source_origin());
        assert!(!artifact.is_complete_source_artifact());
        assert!(!artifact.has_complete_parsed_facts());
        assert!(!artifact.syntax.parsed);
        assert!(artifact.exports.is_empty());
    }

    #[test]
    fn parsed_cache_v2_artifact_preserves_complete_facts_without_source_origin() {
        let artifact = ParsedArtifact::from_parsed_cache_v2(
            PathBuf::from("src/lib.rs"),
            Some("Rust".to_string()),
            vec!["crate::facts".to_string()],
            Vec::new(),
            vec!["run".to_string()],
            FileContextFacts::default(),
            SyntaxSummary {
                parsed: true,
                root_kind: Some("source_file".to_string()),
                has_errors: false,
                named_child_count: 2,
            },
        );

        assert_eq!(artifact.origin, ArtifactOrigin::ParsedCacheV2);
        assert!(!artifact.is_source_origin());
        assert!(!artifact.is_complete_source_artifact());
        assert!(artifact.has_complete_parsed_facts());
        assert_eq!(artifact.exports, vec!["run"]);
        assert!(artifact.syntax.parsed);
    }

    #[test]
    fn source_artifact_has_source_origin_and_complete_facts() {
        let artifact = ParsedArtifact::from_source(
            PathBuf::from("src/lib.rs"),
            Some("Rust".to_string()),
            vec!["crate::facts".to_string()],
            Vec::new(),
            vec!["run".to_string()],
            FileContextFacts::default(),
            SyntaxSummary {
                parsed: true,
                root_kind: Some("source_file".to_string()),
                has_errors: false,
                named_child_count: 2,
            },
        );

        assert_eq!(artifact.origin, ArtifactOrigin::Source);
        assert!(artifact.is_source_origin());
        assert!(artifact.is_complete_source_artifact());
        assert!(artifact.has_complete_parsed_facts());
    }
}
