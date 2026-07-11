use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};

const MAX_STORED_ANALYSES: usize = 8;
const DEFAULT_PAGE_LIMIT: usize = 100;
const MAX_PAGE_LIMIT: usize = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisKind {
    Scan,
    Review,
}

impl AnalysisKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Scan => "scan",
            Self::Review => "review",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnalysisRecord {
    pub kind: AnalysisKind,
    pub report: String,
    pub workspace_revision: String,
}

#[derive(Default)]
pub struct AnalysisStore {
    records: HashMap<String, AnalysisRecord>,
    order: VecDeque<String>,
}

impl AnalysisStore {
    pub fn insert(
        &mut self,
        kind: AnalysisKind,
        report: String,
        workspace_revision: &str,
    ) -> String {
        let handle = analysis_handle(kind, &report, workspace_revision);
        if !self.records.contains_key(&handle) {
            self.order.push_back(handle.clone());
        }
        self.records.insert(
            handle.clone(),
            AnalysisRecord {
                kind,
                report,
                workspace_revision: workspace_revision.to_string(),
            },
        );
        while self.order.len() > MAX_STORED_ANALYSES {
            if let Some(expired) = self.order.pop_front() {
                self.records.remove(&expired);
            }
        }
        handle
    }

    pub fn get(&self, handle: &str) -> Option<&AnalysisRecord> {
        self.records.get(handle)
    }
}

fn analysis_handle(kind: AnalysisKind, report: &str, workspace_revision: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"mcp-analysis-handle-v1\n");
    hasher.update(kind.label().as_bytes());
    hasher.update(b"\n");
    hasher.update(workspace_revision.as_bytes());
    hasher.update(b"\n");
    hasher.update(report.as_bytes());
    let digest = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}-{}", kind.label(), &digest[..24])
}

pub struct PaginatedReport {
    pub text: String,
    pub metadata: Option<Value>,
}

pub fn paginate_findings(report: &str, arguments: &Value) -> Result<PaginatedReport, String> {
    let offset = parse_usize(arguments, "offset")?;
    let limit = parse_usize(arguments, "limit")?;
    if offset.is_none() && limit.is_none() {
        return Ok(PaginatedReport {
            text: report.to_string(),
            metadata: None,
        });
    }

    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);
    if limit == 0 || limit > MAX_PAGE_LIMIT {
        return Err(format!("`limit` must be between 1 and {MAX_PAGE_LIMIT}"));
    }

    let mut value: Value =
        serde_json::from_str(report).map_err(|error| format!("paginate failed: {error}"))?;
    let findings = value
        .get_mut("findings")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "analysis report has no findings array".to_string())?;
    let total = findings.len();
    let start = offset.min(total);
    let end = start.saturating_add(limit).min(total);
    let page = findings[start..end].to_vec();
    *findings = page;
    let returned = end - start;
    let next_offset = (end < total).then_some(end);
    let text = serde_json::to_string_pretty(&value)
        .map_err(|error| format!("paginate failed: {error}"))?;

    Ok(PaginatedReport {
        text,
        metadata: Some(json!({
            "offset": offset,
            "limit": limit,
            "total": total,
            "returned": returned,
            "next_offset": next_offset
        })),
    })
}

fn parse_usize(arguments: &Value, key: &str) -> Result<Option<usize>, String> {
    let Some(value) = arguments.get(key) else {
        return Ok(None);
    };
    let raw = value
        .as_u64()
        .ok_or_else(|| format!("`{key}` must be a non-negative integer"))?;
    usize::try_from(raw)
        .map(Some)
        .map_err(|_| format!("`{key}` is too large"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_preserves_total_and_returns_next_offset() {
        let report = json!({ "findings": [1, 2, 3], "other": true }).to_string();
        let page =
            paginate_findings(&report, &json!({ "offset": 1, "limit": 1 })).expect("paginate");
        let value: Value = serde_json::from_str(&page.text).expect("report JSON");
        assert_eq!(value["findings"], json!([2]));
        assert_eq!(page.metadata.as_ref().expect("metadata")["total"], 3);
        assert_eq!(page.metadata.as_ref().expect("metadata")["next_offset"], 2);
    }

    #[test]
    fn store_evicts_old_handles() {
        let mut store = AnalysisStore::default();
        let first = store.insert(AnalysisKind::Scan, "first".into(), "r1");
        for index in 0..MAX_STORED_ANALYSES {
            store.insert(AnalysisKind::Scan, format!("report-{index}"), "r1");
        }
        assert!(store.get(&first).is_none());
    }
}
