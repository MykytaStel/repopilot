use super::helpers::{normalize, path_contains_component};
use crate::audits::context::model::LanguageKind;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextSignals {
    pub path: Vec<ContextSignal>,
    pub language: Vec<ContextSignal>,
    pub content: Vec<ContextSignal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextSignal {
    pub id: &'static str,
    pub confidence: SignalConfidence,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalConfidence {
    Medium,
    High,
}

impl ContextSignals {
    pub fn detect(path: &Path, language: LanguageKind, content: &str) -> Self {
        let mut signals = Self {
            path: Vec::new(),
            language: Vec::new(),
            content: Vec::new(),
        };

        if is_infrastructure_path(path) {
            if is_known_infrastructure_filename(path) {
                signals.path.push(ContextSignal {
                    id: "path.infrastructure-filename",
                    confidence: SignalConfidence::High,
                    reason: "file name is a known infrastructure orchestration file",
                });
            }

            if has_infrastructure_component(path) {
                signals.path.push(ContextSignal {
                    id: "path.infrastructure-component",
                    confidence: SignalConfidence::High,
                    reason: "path contains a known infrastructure directory component",
                });
            }
        }

        if is_declarative_language(language) {
            signals.language.push(ContextSignal {
                id: "language.declarative",
                confidence: SignalConfidence::High,
                reason: "language is primarily declarative or data-description oriented",
            });
        }

        if is_infrastructure_language(language) {
            signals.language.push(ContextSignal {
                id: "language.infrastructure",
                confidence: SignalConfidence::High,
                reason: "language is specific to infrastructure orchestration",
            });
        }

        if is_functional_first_language(language) {
            signals.language.push(ContextSignal {
                id: "language.functional-first",
                confidence: SignalConfidence::High,
                reason: "language is commonly used with functional-first idioms",
            });
        }

        if content.contains("apiVersion:")
            && (content.contains("kind: Deployment")
                || content.contains("kind: Service")
                || content.contains("kind: ConfigMap"))
        {
            signals.content.push(ContextSignal {
                id: "content.kubernetes-manifest",
                confidence: SignalConfidence::Medium,
                reason: "content resembles a Kubernetes manifest",
            });
        }

        signals
    }

    pub fn has(&self, id: &str) -> bool {
        self.path
            .iter()
            .chain(self.language.iter())
            .chain(self.content.iter())
            .any(|signal| signal.id == id)
    }

    pub fn is_declarative_context(&self) -> bool {
        self.has("language.declarative")
    }

    pub fn is_functional_first_language(&self) -> bool {
        self.has("language.functional-first")
    }

    pub fn is_infrastructure_context(&self) -> bool {
        self.has("language.infrastructure")
            || self.has("path.infrastructure-filename")
            || self.has("path.infrastructure-component")
            || self.has("content.kubernetes-manifest")
    }
}

const INFRASTRUCTURE_COMPONENTS: &[&str] = &[
    "infra",
    "infrastructure",
    "terraform",
    "k8s",
    "kubernetes",
    "helm",
    ".github",
    "workflows",
];

const INFRASTRUCTURE_FILENAMES: &[&str] = &[
    "docker-compose.yml",
    "docker-compose.yaml",
    "compose.yml",
    "compose.yaml",
    "kustomization.yml",
    "kustomization.yaml",
    "helmfile.yml",
    "helmfile.yaml",
];

pub fn is_functional_first_language(language: LanguageKind) -> bool {
    matches!(
        language,
        LanguageKind::Elixir
            | LanguageKind::Erlang
            | LanguageKind::Haskell
            | LanguageKind::OCaml
            | LanguageKind::FSharp
            | LanguageKind::Scala
    )
}

pub fn is_declarative_language(language: LanguageKind) -> bool {
    matches!(
        language,
        LanguageKind::Terraform
            | LanguageKind::Dockerfile
            | LanguageKind::Nix
            | LanguageKind::Yaml
            | LanguageKind::Toml
            | LanguageKind::Json
            | LanguageKind::Sql
            | LanguageKind::Html
            | LanguageKind::Css
            | LanguageKind::Scss
            | LanguageKind::Markdown
    )
}

pub fn is_infrastructure_language(language: LanguageKind) -> bool {
    matches!(
        language,
        LanguageKind::Terraform | LanguageKind::Dockerfile | LanguageKind::Nix
    )
}

fn is_infrastructure_path(path: &Path) -> bool {
    is_known_infrastructure_filename(path) || has_infrastructure_component(path)
}

fn is_known_infrastructure_filename(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(normalize)
        .unwrap_or_default();

    INFRASTRUCTURE_FILENAMES
        .iter()
        .any(|candidate| file_name == *candidate)
}

fn has_infrastructure_component(path: &Path) -> bool {
    path_contains_component(path, INFRASTRUCTURE_COMPONENTS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_infrastructure_filenames() {
        assert!(is_infrastructure_path(Path::new("docker-compose.yml")));
        assert!(is_infrastructure_path(Path::new("deploy/helmfile.yaml")));
        assert!(is_infrastructure_path(Path::new(
            ".github/workflows/ci.yml"
        )));
    }

    #[test]
    fn separates_declarative_from_infrastructure_languages() {
        assert!(is_declarative_language(LanguageKind::Json));
        assert!(is_declarative_language(LanguageKind::Yaml));
        assert!(is_declarative_language(LanguageKind::Toml));
        assert!(is_declarative_language(LanguageKind::Markdown));
        assert!(!is_infrastructure_language(LanguageKind::Json));
        assert!(!is_infrastructure_language(LanguageKind::Yaml));
        assert!(!is_infrastructure_language(LanguageKind::Toml));
        assert!(!is_infrastructure_language(LanguageKind::Markdown));
        assert!(is_infrastructure_language(LanguageKind::Terraform));
    }

    #[test]
    fn detects_functional_first_languages() {
        assert!(is_functional_first_language(LanguageKind::Elixir));
        assert!(is_functional_first_language(LanguageKind::Haskell));
        assert!(!is_functional_first_language(LanguageKind::Rust));
    }

    #[test]
    fn context_signals_explain_infrastructure_context() {
        let signals = ContextSignals::detect(
            Path::new("deploy/k8s/app.yml"),
            LanguageKind::Yaml,
            "apiVersion: apps/v1\nkind: Deployment\n",
        );

        assert!(signals.is_declarative_context());
        assert!(signals.is_infrastructure_context());
        assert!(signals.has("path.infrastructure-component"));
        assert!(signals.has("content.kubernetes-manifest"));
    }
}
