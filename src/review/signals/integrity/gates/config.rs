//! Tool configuration: npm scripts, TypeScript, pytest/coverage/mypy/ruff,
//! and Codecov.

use super::flags::relaxed_thresholds;
use super::parse::{flatten_json, flatten_toml, number, parse_ini, parse_jsonc};
use super::{Relaxation, runs_check, swallows_failure};
use serde_json::Value as Json;
use std::collections::BTreeMap;

/// npm scripts that stop checking, and coverage thresholds that drop.
pub(super) fn package_json(before: &str, after: &str) -> Vec<Relaxation> {
    let (Ok(before), Ok(after)) = (
        serde_json::from_str::<Json>(before),
        serde_json::from_str::<Json>(after),
    ) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let empty = serde_json::Map::new();
    let old_scripts = before
        .get("scripts")
        .and_then(Json::as_object)
        .unwrap_or(&empty);
    let new_scripts = after.get("scripts").and_then(Json::as_object);
    for (name, old) in old_scripts {
        let Some(old) = old.as_str() else { continue };
        if !runs_check(name) && !runs_check(old) {
            continue;
        }
        match new_scripts
            .and_then(|scripts| scripts.get(name))
            .and_then(Json::as_str)
        {
            None => found.push(Relaxation::removed(format!(
                "check script `{name}` was removed"
            ))),
            Some(new) => {
                if let Some(marker) = swallows_failure(new)
                    && swallows_failure(old).is_none()
                {
                    found.push(Relaxation::at(
                        format!("\"{name}\""),
                        format!("check script `{name}` now ends with `{marker}`"),
                    ));
                } else if is_noop(new) && !is_noop(old) {
                    found.push(Relaxation::at(
                        format!("\"{name}\""),
                        format!(
                            "check script `{name}` no longer runs a check (`{}`)",
                            new.trim()
                        ),
                    ));
                }
                found.extend(relaxed_thresholds(
                    old,
                    new,
                    &format!("check script `{name}`"),
                ));
            }
        }
    }
    found.extend(lowered_thresholds(&before, &after));
    found
}

/// TypeScript strictness flags turned off or dropped.
pub(super) fn tsconfig(before: &str, after: &str) -> Vec<Relaxation> {
    const STRICT: &[&str] = &[
        "strict",
        "noImplicitAny",
        "strictNullChecks",
        "strictFunctionTypes",
        "noImplicitReturns",
        "noImplicitThis",
        "noUncheckedIndexedAccess",
        "alwaysStrict",
    ];
    let (Some(before), Some(after)) = (parse_jsonc(before), parse_jsonc(after)) else {
        return Vec::new();
    };
    let flag = |doc: &Json, key: &str| {
        doc.pointer(&format!("/compilerOptions/{key}"))
            .and_then(Json::as_bool)
    };
    STRICT
        .iter()
        .filter(|key| flag(&before, key) == Some(true) && flag(&after, key) != Some(true))
        .map(|key| match flag(&after, key) {
            Some(false) => Relaxation::at(
                format!("\"{key}\""),
                format!("`compilerOptions.{key}` turned off"),
            ),
            _ => Relaxation::removed(format!("`compilerOptions.{key}: true` was removed")),
        })
        .collect()
}

/// pytest, coverage, mypy, and ruff settings in `pyproject.toml`.
pub(super) fn pyproject(before: &str, after: &str) -> Vec<Relaxation> {
    let (Ok(before), Ok(after)) = (before.parse::<toml::Table>(), after.parse::<toml::Table>())
    else {
        return Vec::new();
    };
    let flat = |table: &toml::Table| {
        let mut values = BTreeMap::new();
        flatten_toml("", &toml::Value::Table(table.clone()), &mut values);
        values
    };
    compare_settings(&flat(&before), &flat(&after))
}

/// The same settings in INI files: `setup.cfg`, `pytest.ini`, `tox.ini`,
/// `.coveragerc`, `mypy.ini`.
pub(super) fn ini(before: &str, after: &str) -> Vec<Relaxation> {
    compare_settings(&parse_ini(before), &parse_ini(after))
}

/// Codecov targets lowered or statuses made informational.
pub(super) fn codecov(before: &str, after: &str) -> Vec<Relaxation> {
    let (Ok(before), Ok(after)) = (
        serde_yaml::from_str::<Json>(before),
        serde_yaml::from_str::<Json>(after),
    ) else {
        return Vec::new();
    };
    let mut found = lowered_thresholds(&before, &after);
    let (mut old, mut new) = (BTreeMap::new(), BTreeMap::new());
    flatten_json("", &before, &mut old);
    flatten_json("", &after, &mut new);
    for (key, value) in &new {
        if key.ends_with("informational")
            && value == "true"
            && old.get(key).map(String::as_str) != Some("true")
        {
            found.push(Relaxation::at(
                "informational",
                format!("`{key}` is now true; coverage no longer fails the check"),
            ));
        }
    }
    found
}

/// Settings compared as flattened `section.key` paths (TOML or INI).
fn compare_settings(
    old: &BTreeMap<String, String>,
    new: &BTreeMap<String, String>,
) -> Vec<Relaxation> {
    let mut found = Vec::new();
    for (key, value) in new {
        let previous = old.get(key);
        let leaf = key.rsplit('.').next().unwrap_or(key);
        if let Some(previous) = previous {
            found.extend(relaxed_thresholds(previous, value, &format!("`{key}`")));
        }
        if leaf == "addopts" {
            // Quotes vary (`-m "not slow"`); compare the bare option text.
            let unquoted = |text: &str| text.replace(['"', '\''], "");
            let (value, previous) = (unquoted(value), previous.map(|old| unquoted(old)));
            for flag in ["--deselect", "--ignore", "-k ", "-m not", "--lf"] {
                if value.contains(flag) && !previous.as_ref().is_some_and(|old| old.contains(flag))
                {
                    found.push(Relaxation::at(
                        flag.trim(),
                        format!(
                            "`{key}` now passes `{}`; some tests no longer run",
                            flag.trim()
                        ),
                    ));
                }
            }
        }
        if (leaf == "fail_under" || leaf == "cov-fail-under")
            && let (Some(old), Some(new)) = (previous.and_then(|v| number(v)), number(value))
            && new < old
        {
            found.push(Relaxation::at(
                leaf,
                format!("`{key}` lowered from {old} to {new}"),
            ));
        }
        if matches!(leaf, "strict" | "disallow_untyped_defs" | "strict_optional")
            && value == "false"
            && previous.map(String::as_str) == Some("true")
        {
            found.push(Relaxation::at(leaf, format!("`{key}` turned off")));
        }
        if leaf == "ignore_errors"
            && value == "true"
            && previous.map(String::as_str) != Some("true")
        {
            found.push(Relaxation::at(
                leaf,
                format!("`{key}` is now true; type errors are not reported"),
            ));
        }
        if matches!(leaf, "ignore" | "extend-ignore") && key.contains("ruff") {
            let old_codes: Vec<&str> = previous.map_or(Vec::new(), |old| old.split(',').collect());
            let added: Vec<&str> = value
                .split(',')
                .filter(|code| !code.is_empty() && !old_codes.contains(code))
                .collect();
            if !added.is_empty() {
                found.push(Relaxation::at(
                    leaf,
                    format!("`{key}` now ignores {}", added.join(", ")),
                ));
            }
        }
    }
    for key in old.keys() {
        if key.ends_with("fail_under") && !new.contains_key(key) {
            found.push(Relaxation::removed(format!(
                "`{key}` was removed; coverage no longer has a floor"
            )));
        }
    }
    found
}

/// Numbers under a coverage threshold path that went down.
fn lowered_thresholds(before: &Json, after: &Json) -> Vec<Relaxation> {
    let (mut old, mut new) = (BTreeMap::new(), BTreeMap::new());
    flatten_json("", before, &mut old);
    flatten_json("", after, &mut new);
    new.iter()
        .filter(|(key, _)| {
            let lower = key.to_ascii_lowercase();
            lower.contains("threshold") || lower.ends_with(".target")
        })
        .filter_map(|(key, value)| {
            let (old, new) = (number(old.get(key)?)?, number(value)?);
            (new < old).then(|| {
                let leaf = key.rsplit('.').next().unwrap_or(key);
                Relaxation::at(leaf, format!("`{key}` lowered from {old} to {new}"))
            })
        })
        .collect()
}

fn is_noop(script: &str) -> bool {
    let script = script.trim();
    script.is_empty() || script == "true" || script == "exit 0" || script.starts_with("echo ")
}
