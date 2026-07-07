use crate::findings::decision::{DecisionRecord, build_decision_record};
use crate::findings::occurrence::occurrence_key;
use crate::findings::types::Finding;
use serde::Serialize;

/// The one shared wrapper every JSON output surface (scan, baseline, review)
/// uses to expose a finding's occurrence key and canonical decision record
/// alongside its existing fields. SARIF and the AI-context JSON DTO are
/// bespoke (not `Finding`-flattening) and populate the same two values
/// directly via `occurrence_key`/`build_decision_record`.
#[derive(Debug, Serialize)]
pub struct FindingRecord<'a> {
    #[serde(flatten)]
    pub finding: &'a Finding,
    pub occurrence_key: String,
    pub decision: DecisionRecord,
}

impl<'a> FindingRecord<'a> {
    pub fn new(finding: &'a Finding) -> Self {
        Self {
            finding,
            occurrence_key: occurrence_key(finding),
            decision: build_decision_record(finding),
        }
    }
}
