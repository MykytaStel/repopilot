//! CI pipelines: GitHub Actions workflows and GitLab CI.

use super::{Relaxation, runs_check, swallows_failure};
use serde_yaml::Value;

/// GitHub Actions: steps or jobs that stop failing the run, or that run a
/// check and were removed.
pub(super) fn github_actions(before: &str, after: &str) -> Vec<Relaxation> {
    let (Some(before), Some(after)) = (parse(before), parse(after)) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let (Some(old_jobs), Some(new_jobs)) = (mapping(&before, "jobs"), mapping(&after, "jobs"))
    else {
        return found;
    };
    for (job_id, old_job) in old_jobs {
        let name = key_text(job_id);
        let Some(new_job) = new_jobs.get(job_id) else {
            if job_runs_check(old_job, &name) {
                found.push(Relaxation::removed(format!(
                    "job `{name}` that ran checks was removed"
                )));
            }
            continue;
        };
        if is_true(new_job.get("continue-on-error")) && !is_true(old_job.get("continue-on-error")) {
            found.push(Relaxation::at(
                "continue-on-error",
                format!("job `{name}` now has `continue-on-error: true`; its failures no longer fail the run"),
            ));
        }
        if is_disabled(new_job.get("if")) && !is_disabled(old_job.get("if")) {
            found.push(Relaxation::at(
                "if:",
                format!("job `{name}` is now disabled with `if: false`"),
            ));
        }
        found.extend(steps(&name, old_job, new_job));
    }
    found
}

fn steps(job: &str, old_job: &Value, new_job: &Value) -> Vec<Relaxation> {
    let old_steps = sequence(old_job, "steps");
    let new_steps = sequence(new_job, "steps");
    let mut found = Vec::new();
    for old_step in old_steps {
        let label = step_label(old_step);
        if !runs_check(&label) {
            continue;
        }
        let Some(new_step) = new_steps.iter().find(|step| same_step(old_step, step)) else {
            found.push(Relaxation::removed(format!(
                "check step `{label}` in job `{job}` was removed"
            )));
            continue;
        };
        if is_true(new_step.get("continue-on-error")) && !is_true(old_step.get("continue-on-error"))
        {
            found.push(Relaxation::at(
                "continue-on-error",
                format!("check step `{label}` in job `{job}` now has `continue-on-error: true`"),
            ));
        }
        if is_disabled(new_step.get("if")) && !is_disabled(old_step.get("if")) {
            found.push(Relaxation::at(
                "if:",
                format!("check step `{label}` in job `{job}` is now disabled with `if: false`"),
            ));
        }
        if let (Some(old_run), Some(new_run)) =
            (text(old_step.get("run")), text(new_step.get("run")))
            && let Some(marker) = swallows_failure(&new_run)
            && swallows_failure(&old_run).is_none()
        {
            found.push(Relaxation::at(
                marker,
                format!("check step `{label}` in job `{job}` now ends with `{marker}`"),
            ));
        }
    }
    found
}

/// GitLab CI: jobs that may now fail, run only manually, or were removed.
pub(super) fn gitlab(before: &str, after: &str) -> Vec<Relaxation> {
    const RESERVED: &[&str] = &[
        "stages",
        "variables",
        "default",
        "include",
        "workflow",
        "image",
        "services",
        "before_script",
        "after_script",
        "cache",
    ];
    let (Some(before), Some(after)) = (parse(before), parse(after)) else {
        return Vec::new();
    };
    let (Some(old_jobs), Some(new_jobs)) = (before.as_mapping(), after.as_mapping()) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for (job_id, old_job) in old_jobs {
        let name = key_text(job_id);
        if RESERVED.contains(&name.as_str()) || name.starts_with('.') || !old_job.is_mapping() {
            continue;
        }
        let script = scripts(old_job);
        if !runs_check(&format!("{name} {script}")) {
            continue;
        }
        let Some(new_job) = new_jobs.get(job_id) else {
            found.push(Relaxation::removed(format!(
                "job `{name}` that ran checks was removed"
            )));
            continue;
        };
        if is_true(new_job.get("allow_failure")) && !is_true(old_job.get("allow_failure")) {
            found.push(Relaxation::at(
                "allow_failure",
                format!("job `{name}` now has `allow_failure: true`"),
            ));
        }
        if text(new_job.get("when")).as_deref() == Some("manual")
            && text(old_job.get("when")).as_deref() != Some("manual")
        {
            found.push(Relaxation::at(
                "when: manual",
                format!("job `{name}` now runs only manually"),
            ));
        }
        if let Some(marker) = swallows_failure(&scripts(new_job))
            && swallows_failure(&script).is_none()
        {
            found.push(Relaxation::at(
                marker,
                format!("job `{name}` script now ends with `{marker}`"),
            ));
        }
    }
    found
}

fn parse(content: &str) -> Option<Value> {
    serde_yaml::from_str(content).ok()
}

fn mapping<'a>(value: &'a Value, key: &str) -> Option<&'a serde_yaml::Mapping> {
    value.get(key)?.as_mapping()
}

fn sequence<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_sequence)
        .map_or(&[], |steps| steps.as_slice())
}

fn key_text(key: &Value) -> String {
    text(Some(key)).unwrap_or_default()
}

fn text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.clone()),
        Value::Bool(flag) => Some(flag.to_string()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn is_true(value: Option<&Value>) -> bool {
    matches!(text(value).as_deref(), Some("true"))
}

fn is_disabled(value: Option<&Value>) -> bool {
    let compact: String = text(value).unwrap_or_default().split_whitespace().collect();
    matches!(compact.as_str(), "false" | "${{false}}")
}

fn step_label(step: &Value) -> String {
    ["name", "run", "uses"]
        .iter()
        .find_map(|key| text(step.get(*key)))
        .map(|label| label.lines().next().unwrap_or_default().trim().to_string())
        .unwrap_or_default()
}

fn same_step(left: &Value, right: &Value) -> bool {
    ["name", "uses", "run"].iter().any(|key| {
        let (a, b) = (text(left.get(*key)), text(right.get(*key)));
        a.is_some() && a == b
    }) || (text(left.get("name")).is_some() && text(left.get("name")) == text(right.get("name")))
}

fn job_runs_check(job: &Value, name: &str) -> bool {
    runs_check(name)
        || sequence(job, "steps")
            .iter()
            .any(|step| runs_check(&step_label(step)))
}

fn scripts(job: &Value) -> String {
    ["before_script", "script"]
        .iter()
        .filter_map(|key| job.get(*key))
        .flat_map(|value| match value {
            Value::Sequence(lines) => lines.iter().filter_map(|line| text(Some(line))).collect(),
            other => text(Some(other)).into_iter().collect::<Vec<_>>(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}
