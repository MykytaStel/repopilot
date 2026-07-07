use super::lifecycle::RuleLifecycle;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum RuleScope {
    File,
    Repository,
    FrameworkProject,
    ChangeSet,
}

impl RuleScope {
    pub fn label(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Repository => "repository",
            Self::FrameworkProject => "framework-project",
            Self::ChangeSet => "change-set",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum FactKind {
    FileMetadata,
    FileContent,
    ParsedSyntax,
    Imports,
    Exports,
    FileContext,
    WorkspaceMetadata,
    DependencyGraph,
    DependencyManifest,
    ConfigFile,
    FrameworkContext,
    GitDiff,
}

impl FactKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::FileMetadata => "file-metadata",
            Self::FileContent => "file-content",
            Self::ParsedSyntax => "parsed-syntax",
            Self::Imports => "imports",
            Self::Exports => "exports",
            Self::FileContext => "file-context",
            Self::WorkspaceMetadata => "workspace-metadata",
            Self::DependencyGraph => "dependency-graph",
            Self::DependencyManifest => "dependency-manifest",
            Self::ConfigFile => "config-file",
            Self::FrameworkContext => "framework-context",
            Self::GitDiff => "git-diff",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum RuleCachePolicy {
    PerFileContent,
    PerWorkspaceRevision,
    PerChangeSet,
    Uncached,
}

impl RuleCachePolicy {
    pub fn label(self) -> &'static str {
        match self {
            Self::PerFileContent => "per-file-content",
            Self::PerWorkspaceRevision => "per-workspace-revision",
            Self::PerChangeSet => "per-change-set",
            Self::Uncached => "uncached",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum RuleOutputKind {
    Finding,
}

impl RuleOutputKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Finding => "finding",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleRequirements {
    pub scope: RuleScope,
    pub fact_kinds: &'static [FactKind],
    pub lifecycle: RuleLifecycle,
    pub cache_policy: RuleCachePolicy,
    pub produces: &'static [RuleOutputKind],
}

const FINDING_OUTPUT: &[RuleOutputKind] = &[RuleOutputKind::Finding];

const FILE_TEXT_FACTS: &[FactKind] = &[FactKind::FileMetadata, FactKind::FileContent];
const FILE_AST_FACTS: &[FactKind] = &[
    FactKind::FileMetadata,
    FactKind::ParsedSyntax,
    FactKind::FileContext,
];
const REPOSITORY_TEXT_FACTS: &[FactKind] = &[
    FactKind::FileMetadata,
    FactKind::FileContent,
    FactKind::WorkspaceMetadata,
];
const REPOSITORY_CONFIG_FACTS: &[FactKind] = &[FactKind::ConfigFile, FactKind::WorkspaceMetadata];
const REPOSITORY_GRAPH_FACTS: &[FactKind] = &[
    FactKind::Imports,
    FactKind::Exports,
    FactKind::FileContext,
    FactKind::DependencyGraph,
    FactKind::WorkspaceMetadata,
];
const REPOSITORY_MIXED_FACTS: &[FactKind] = &[
    FactKind::FileContent,
    FactKind::ParsedSyntax,
    FactKind::FileContext,
    FactKind::WorkspaceMetadata,
];
const FRAMEWORK_AST_FACTS: &[FactKind] = &[
    FactKind::ParsedSyntax,
    FactKind::FileContext,
    FactKind::FrameworkContext,
];
const FRAMEWORK_MANIFEST_FACTS: &[FactKind] = &[
    FactKind::DependencyManifest,
    FactKind::FrameworkContext,
    FactKind::WorkspaceMetadata,
];
const FRAMEWORK_CONFIG_FACTS: &[FactKind] = &[
    FactKind::ConfigFile,
    FactKind::FrameworkContext,
    FactKind::WorkspaceMetadata,
];
const FRAMEWORK_DETECTOR_FACTS: &[FactKind] =
    &[FactKind::FrameworkContext, FactKind::WorkspaceMetadata];
const FRAMEWORK_TEXT_FACTS: &[FactKind] = &[
    FactKind::FileContent,
    FactKind::FileContext,
    FactKind::FrameworkContext,
];
const CHANGE_SET_FACTS: &[FactKind] = &[FactKind::GitDiff, FactKind::WorkspaceMetadata];

impl RuleRequirements {
    pub const UNDECLARED: Self = Self {
        scope: RuleScope::File,
        fact_kinds: &[],
        lifecycle: RuleLifecycle::Preview,
        cache_policy: RuleCachePolicy::Uncached,
        produces: &[],
    };

    const fn declared(
        scope: RuleScope,
        fact_kinds: &'static [FactKind],
        lifecycle: RuleLifecycle,
        cache_policy: RuleCachePolicy,
    ) -> Self {
        Self {
            scope,
            fact_kinds,
            lifecycle,
            cache_policy,
            produces: FINDING_OUTPUT,
        }
    }

    pub const fn file_text(lifecycle: RuleLifecycle) -> Self {
        Self::declared(
            RuleScope::File,
            FILE_TEXT_FACTS,
            lifecycle,
            RuleCachePolicy::PerFileContent,
        )
    }

    pub const fn file_ast(lifecycle: RuleLifecycle) -> Self {
        Self::declared(
            RuleScope::File,
            FILE_AST_FACTS,
            lifecycle,
            RuleCachePolicy::PerFileContent,
        )
    }

    pub const fn repository_text(lifecycle: RuleLifecycle) -> Self {
        Self::declared(
            RuleScope::Repository,
            REPOSITORY_TEXT_FACTS,
            lifecycle,
            RuleCachePolicy::PerWorkspaceRevision,
        )
    }

    pub const fn repository_config(lifecycle: RuleLifecycle) -> Self {
        Self::declared(
            RuleScope::Repository,
            REPOSITORY_CONFIG_FACTS,
            lifecycle,
            RuleCachePolicy::PerWorkspaceRevision,
        )
    }

    pub const fn repository_graph(lifecycle: RuleLifecycle) -> Self {
        Self::declared(
            RuleScope::Repository,
            REPOSITORY_GRAPH_FACTS,
            lifecycle,
            RuleCachePolicy::PerWorkspaceRevision,
        )
    }

    pub const fn repository_mixed(lifecycle: RuleLifecycle) -> Self {
        Self::declared(
            RuleScope::Repository,
            REPOSITORY_MIXED_FACTS,
            lifecycle,
            RuleCachePolicy::PerWorkspaceRevision,
        )
    }

    pub const fn framework_ast(lifecycle: RuleLifecycle) -> Self {
        Self::declared(
            RuleScope::FrameworkProject,
            FRAMEWORK_AST_FACTS,
            lifecycle,
            RuleCachePolicy::PerWorkspaceRevision,
        )
    }

    pub const fn framework_manifest(lifecycle: RuleLifecycle) -> Self {
        Self::declared(
            RuleScope::FrameworkProject,
            FRAMEWORK_MANIFEST_FACTS,
            lifecycle,
            RuleCachePolicy::PerWorkspaceRevision,
        )
    }

    pub const fn framework_config(lifecycle: RuleLifecycle) -> Self {
        Self::declared(
            RuleScope::FrameworkProject,
            FRAMEWORK_CONFIG_FACTS,
            lifecycle,
            RuleCachePolicy::PerWorkspaceRevision,
        )
    }

    pub const fn framework_detector(lifecycle: RuleLifecycle) -> Self {
        Self::declared(
            RuleScope::FrameworkProject,
            FRAMEWORK_DETECTOR_FACTS,
            lifecycle,
            RuleCachePolicy::PerWorkspaceRevision,
        )
    }

    pub const fn framework_text(lifecycle: RuleLifecycle) -> Self {
        Self::declared(
            RuleScope::FrameworkProject,
            FRAMEWORK_TEXT_FACTS,
            lifecycle,
            RuleCachePolicy::PerWorkspaceRevision,
        )
    }

    pub const fn change_set(lifecycle: RuleLifecycle) -> Self {
        Self::declared(
            RuleScope::ChangeSet,
            CHANGE_SET_FACTS,
            lifecycle,
            RuleCachePolicy::PerChangeSet,
        )
    }

    pub fn is_declared(self) -> bool {
        !self.fact_kinds.is_empty()
            && !self.produces.is_empty()
            && self.cache_policy != RuleCachePolicy::Uncached
    }

    pub fn requires_parsed_syntax(self) -> bool {
        self.fact_kinds.contains(&FactKind::ParsedSyntax)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_ast_requires_shared_parse_and_context() {
        let requirements = RuleRequirements::file_ast(RuleLifecycle::Preview);

        assert_eq!(requirements.scope, RuleScope::File);
        assert!(requirements.requires_parsed_syntax());
        assert!(requirements.fact_kinds.contains(&FactKind::FileContext));
        assert_eq!(requirements.cache_policy, RuleCachePolicy::PerFileContent);
        assert!(requirements.is_declared());
    }

    #[test]
    fn graph_rules_are_revision_scoped() {
        let requirements = RuleRequirements::repository_graph(RuleLifecycle::Stable);

        assert_eq!(requirements.scope, RuleScope::Repository);
        assert!(requirements.fact_kinds.contains(&FactKind::DependencyGraph));
        assert_eq!(
            requirements.cache_policy,
            RuleCachePolicy::PerWorkspaceRevision
        );
        assert_eq!(requirements.lifecycle, RuleLifecycle::Stable);
    }

    #[test]
    fn undeclared_requirements_are_detectable() {
        assert!(!RuleRequirements::UNDECLARED.is_declared());
    }
}
