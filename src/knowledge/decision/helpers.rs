fn override_matches(override_rule: &RuleOverride, context: &RuleMatchContext<'_>) -> bool {
    if override_rule
        .signal
        .as_deref()
        .is_some_and(|signal| Some(signal) != context.signal)
    {
        return false;
    }

    if override_rule
        .language
        .as_deref()
        .is_some_and(|language| !context.languages.contains(&language))
    {
        return false;
    }

    if override_rule
        .framework
        .as_deref()
        .is_some_and(|framework| !context.frameworks.contains(&framework))
    {
        return false;
    }

    if override_rule
        .runtime
        .as_deref()
        .is_some_and(|runtime| !context.runtimes.contains(&runtime))
    {
        return false;
    }

    if override_rule
        .paradigm
        .as_deref()
        .is_some_and(|paradigm| !context.paradigms.contains(&paradigm))
    {
        return false;
    }

    if override_rule
        .role
        .as_deref()
        .is_some_and(|role| !context.roles.contains(&role))
    {
        return false;
    }

    true
}

fn is_test_override(override_rule: &RuleOverride) -> bool {
    matches!(override_rule.role.as_deref(), Some("test" | "rust-test"))
}

fn apply_override(
    override_rule: &RuleOverride,
    current: Severity,
    fallback_risk: Option<crate::knowledge::model::RuleRiskAdjustment>,
) -> RuleDecision {
    let risk_signal = override_rule.risk.clone().or(fallback_risk);
    match override_rule.action {
        RuleDecisionAction::Apply => RuleDecision {
            action: RuleDecisionAction::Apply,
            severity: override_rule.severity.unwrap_or(current),
            reason: override_rule.reason.clone(),
            risk_signal,
        },
        RuleDecisionAction::Suppress => RuleDecision {
            action: RuleDecisionAction::Suppress,
            severity: Severity::Info,
            reason: override_rule.reason.clone(),
            risk_signal: None,
        },
        RuleDecisionAction::Downgrade => {
            let severity = override_rule
                .severity
                .filter(|severity| *severity < current)
                .unwrap_or(current);
            RuleDecision {
                action: RuleDecisionAction::Downgrade,
                severity,
                reason: override_rule.reason.clone(),
                risk_signal,
            }
        }
        RuleDecisionAction::Upgrade => {
            let severity = override_rule
                .severity
                .filter(|severity| *severity > current)
                .unwrap_or(current);
            RuleDecision {
                action: RuleDecisionAction::Upgrade,
                severity,
                reason: override_rule.reason.clone(),
                risk_signal,
            }
        }
    }
}

fn language_support_satisfies(
    rule: &crate::knowledge::model::RuleApplicability,
    context_languages: &[&str],
    minimum_support: crate::knowledge::model::SupportLevel,
) -> bool {
    context_languages.iter().any(|language| {
        (rule.languages.is_empty() || rule.languages.contains(*language))
            && profile_by_id(language).is_some_and(|profile| profile.support >= minimum_support)
    })
}

fn language_ids_for_file(file: &FileFacts, context: &AuditContext) -> Vec<&'static str> {
    let mut languages = Vec::new();
    if let Some(language) = file
        .language
        .as_deref()
        .and_then(language_id_for_name)
        .filter(|language| *language != "unknown")
    {
        languages.push(language);
    }

    let context_language = context.language_id();
    if context_language != "unknown" {
        languages.push(context_language);
    }

    languages
}

fn language_ids_for_project(facts: &ScanFacts) -> Vec<&'static str> {
    let mut languages = facts
        .languages
        .iter()
        .filter_map(|language| language_id_for_name(&language.name))
        .collect::<Vec<_>>();

    if languages.is_empty() {
        languages.extend(
            facts
                .files
                .iter()
                .filter_map(|file| file.language.as_deref())
                .filter_map(language_id_for_name),
        );
    }

    languages
}

fn framework_ids_for_project(facts: &ScanFacts) -> Vec<&'static str> {
    let mut frameworks = facts
        .detected_frameworks
        .iter()
        .filter_map(framework_id)
        .collect::<Vec<_>>();

    for project in &facts.framework_projects {
        frameworks.extend(project.frameworks.iter().filter_map(framework_id));
    }

    dedup_static_ids(&mut frameworks);
    frameworks
}

fn framework_id(framework: &DetectedFramework) -> Option<&'static str> {
    match framework {
        DetectedFramework::ReactNative { .. } => Some("react-native"),
        DetectedFramework::Expo { .. } => Some("expo"),
        DetectedFramework::NextJs { .. } => Some("nextjs"),
        DetectedFramework::React { .. } => Some("react"),
        DetectedFramework::Vue { .. } => Some("vue"),
        DetectedFramework::Angular { .. } => Some("angular"),
        DetectedFramework::Svelte { .. } => Some("svelte"),
        DetectedFramework::NestJs { .. } => Some("nestjs"),
        DetectedFramework::Express { .. } => Some("express"),
        DetectedFramework::Django { .. } => Some("django"),
        DetectedFramework::Flask { .. } => Some("flask"),
        DetectedFramework::FastApi { .. } => Some("fastapi"),
        DetectedFramework::Gin { .. } => Some("gin"),
        DetectedFramework::Echo { .. } => Some("echo"),
        DetectedFramework::Fiber { .. } => Some("fiber"),
    }
}

fn dedup_static_ids(values: &mut Vec<&'static str>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(*value));
}

fn override_criteria(override_rule: &RuleOverride) -> Vec<String> {
    let mut criteria = Vec::new();
    if let Some(value) = &override_rule.signal { criteria.push(format!("signal={value}")); }
    if let Some(value) = &override_rule.language { criteria.push(format!("language={value}")); }
    if let Some(value) = &override_rule.framework { criteria.push(format!("framework={value}")); }
    if let Some(value) = &override_rule.runtime { criteria.push(format!("runtime={value}")); }
    if let Some(value) = &override_rule.paradigm { criteria.push(format!("paradigm={value}")); }
    if let Some(value) = &override_rule.role { criteria.push(format!("role={value}")); }
    if criteria.is_empty() { criteria.push("unconditional".to_string()); }
    criteria
}

fn sorted_values(values: &HashSet<String>) -> String {
    if values.is_empty() { return "none".to_string(); }
    let mut values = values.iter().map(String::as_str).collect::<Vec<_>>();
    values.sort_unstable();
    values.join(",")
}

fn joined_ids(values: &[&str]) -> String {
    if values.is_empty() { "none".to_string() } else { values.join(",") }
}

fn support_level_id(level: SupportLevel) -> &'static str {
    match level {
        SupportLevel::DetectOnly => "detect-only",
        SupportLevel::ImportAware => "import-aware",
        SupportLevel::ContextAware => "context-aware",
        SupportLevel::RuleAware => "rule-aware",
    }
}
