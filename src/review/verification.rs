use crate::config::model::VerificationCheckConfig;
use crate::verification::VerificationRole;
use globset::Glob;

/// The configured verification capability and the checks selected for a review.
///
/// This is internal review state rather than an output contract. Keeping it on
/// the report lets proof, readiness, and renderers reason from the same policy
/// without inferring availability from returned outcomes alone.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VerificationPolicy {
    pub configured: Vec<VerificationPolicyCheck>,
    pub selected: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationPolicyCheck {
    pub id: String,
    pub role: VerificationRole,
    pub paths: Vec<String>,
}

impl VerificationPolicyCheck {
    pub fn matches_path(&self, path: &str) -> bool {
        if self.paths.is_empty() {
            return true;
        }
        let normalized = path.replace('\\', "/");
        self.paths.iter().any(|pattern| {
            Glob::new(pattern)
                .map(|glob| glob.compile_matcher().is_match(&normalized))
                .unwrap_or(false)
        })
    }
}

impl VerificationPolicy {
    pub fn from_configs(configured: &[VerificationCheckConfig]) -> Self {
        Self {
            configured: configured
                .iter()
                .map(|check| {
                    let mut paths = check
                        .paths
                        .iter()
                        .map(|path| path.replace('\\', "/"))
                        .collect::<Vec<_>>();
                    paths.sort();
                    paths.dedup();
                    VerificationPolicyCheck {
                        id: check.id.clone(),
                        role: check.role,
                        paths,
                    }
                })
                .collect(),
            selected: Vec::new(),
        }
    }

    pub fn set_selected(&mut self, selected: &[String]) {
        self.selected = selected.to_vec();
        self.selected.sort();
        self.selected.dedup();
    }
}
