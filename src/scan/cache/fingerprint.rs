pub fn config_fingerprint(config: &ScanConfig) -> String {
    let input = CacheFingerprintInput {
        cache_schema_version: CACHE_SCHEMA_VERSION,
        repopilot_version: env!("CARGO_PKG_VERSION"),
        scan_config: config,
        file_audits: registered_file_audits(config)
            .into_iter()
            .map(|registration| AuditFingerprint {
                audit_id: registration.metadata.audit_id,
                kind: registration.metadata.kind.label(),
                category: registration.metadata.category,
                rule_ids: registration.metadata.rule_ids.to_vec(),
            })
            .collect(),
        project_audits: registered_project_audits(config)
            .into_iter()
            .map(|registration| AuditFingerprint {
                audit_id: registration.metadata.audit_id,
                kind: registration.metadata.kind.label(),
                category: registration.metadata.category,
                rule_ids: registration.metadata.rule_ids.to_vec(),
            })
            .collect(),
        rules: all_rule_metadata()
            .map(|rule| RuleFingerprint {
                rule_id: rule.rule_id,
                title: rule.title,
                category: rule.category.clone(),
                default_severity: rule.default_severity,
                docs_url: rule.docs_url,
                description: rule.description,
                recommendation: rule.recommendation,
            })
            .collect(),
    };

    match serde_json::to_vec(&input) {
        Ok(bytes) => stable_hash_hex(&bytes),
        Err(_) => stable_hash_hex(format!("{config:?}:{}", CACHE_SCHEMA_VERSION).as_bytes()),
    }
}

pub fn stable_hash_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Serialize)]
struct CacheFingerprintInput<'a> {
    cache_schema_version: u32,
    repopilot_version: &'static str,
    scan_config: &'a ScanConfig,
    file_audits: Vec<AuditFingerprint>,
    project_audits: Vec<AuditFingerprint>,
    rules: Vec<RuleFingerprint>,
}

#[derive(Serialize)]
struct AuditFingerprint {
    audit_id: &'static str,
    kind: &'static str,
    category: FindingCategory,
    rule_ids: Vec<&'static str>,
}

#[derive(Serialize)]
struct RuleFingerprint {
    rule_id: &'static str,
    title: &'static str,
    category: FindingCategory,
    default_severity: Severity,
    docs_url: Option<&'static str>,
    description: &'static str,
    recommendation: Option<&'static str>,
}
