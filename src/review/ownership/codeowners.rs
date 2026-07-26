use super::model::{Owner, OwnershipDiagnostic};
use globset::{GlobBuilder, GlobMatcher};
use std::fs;
use std::path::{Path, PathBuf};

const LOCATIONS: [&str; 3] = [".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS"];

pub struct OwnershipIndex {
    source: Option<PathBuf>,
    rules: Vec<OwnershipRule>,
}

pub struct OwnershipDiscovery {
    pub index: OwnershipIndex,
    pub diagnostics: Vec<OwnershipDiagnostic>,
}

struct OwnershipRule {
    matcher: CodeownersMatcher,
    owners: Vec<Owner>,
}

enum CodeownersMatcher {
    Path(GlobMatcher),
    Component(GlobMatcher),
}

impl OwnershipIndex {
    pub fn empty() -> Self {
        Self {
            source: None,
            rules: Vec::new(),
        }
    }

    pub fn discover(root: &Path) -> OwnershipDiscovery {
        for relative in LOCATIONS {
            let path = root.join(relative);
            if !path.is_file() {
                continue;
            }
            return match fs::read_to_string(&path) {
                Ok(content) => Self::parse(&content, PathBuf::from(relative)),
                Err(error) => OwnershipDiscovery {
                    index: Self::empty(),
                    diagnostics: vec![OwnershipDiagnostic {
                        message: format!("failed to read {relative}: {error}"),
                        line: None,
                    }],
                },
            };
        }
        OwnershipDiscovery {
            index: Self::empty(),
            diagnostics: Vec::new(),
        }
    }

    pub fn from_codeowners(content: &str, source: PathBuf) -> Result<Self, String> {
        let discovery = Self::parse(content, source);
        if discovery.diagnostics.is_empty() {
            Ok(discovery.index)
        } else {
            Err(discovery
                .diagnostics
                .into_iter()
                .map(|item| item.message)
                .collect::<Vec<_>>()
                .join("; "))
        }
    }

    pub fn source(&self) -> Option<&Path> {
        self.source.as_deref()
    }

    pub fn owners_for(&self, path: &Path) -> Vec<Owner> {
        let path = path.to_string_lossy().replace('\\', "/");
        self.rules
            .iter()
            .rev()
            .find(|rule| rule.matcher.is_match(&path))
            .map(|rule| rule.owners.clone())
            .unwrap_or_default()
    }

    fn parse(content: &str, source: PathBuf) -> OwnershipDiscovery {
        let mut rules = Vec::new();
        let mut diagnostics = Vec::new();
        for (index, line) in content.lines().enumerate() {
            match parse_rule(line) {
                Ok(Some(rule)) => rules.push(rule),
                Ok(None) => {}
                Err(message) => diagnostics.push(OwnershipDiagnostic {
                    message,
                    line: Some(index + 1),
                }),
            }
        }
        OwnershipDiscovery {
            index: Self {
                source: Some(source),
                rules,
            },
            diagnostics,
        }
    }
}

impl CodeownersMatcher {
    fn is_match(&self, path: &str) -> bool {
        match self {
            Self::Path(matcher) => matcher.is_match(path),
            Self::Component(matcher) => path.split('/').any(|part| matcher.is_match(part)),
        }
    }
}

fn parse_rule(line: &str) -> Result<Option<OwnershipRule>, String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(None);
    }
    let tokens = tokenize(line);
    if tokens.len() < 2 {
        return Err("CODEOWNERS rule requires a pattern and at least one owner".to_string());
    }
    let matcher = compile_pattern(&tokens[0])?;
    let mut owners = Vec::new();
    for value in tokens.into_iter().skip(1) {
        if !owners.iter().any(|owner: &Owner| owner.value == value) {
            owners.push(Owner { value });
        }
    }
    Ok(Some(OwnershipRule { matcher, owners }))
}

fn compile_pattern(raw: &str) -> Result<CodeownersMatcher, String> {
    if raw.starts_with('!') {
        return Err("CODEOWNERS does not support negated patterns".to_string());
    }
    let anchored = raw.starts_with('/');
    let mut pattern = raw.trim_start_matches('/').to_string();
    if pattern.ends_with('/') {
        pattern.push_str("**");
    }
    let has_separator = pattern.contains('/');
    let glob = GlobBuilder::new(&pattern)
        .literal_separator(true)
        .build()
        .map_err(|error| format!("invalid CODEOWNERS pattern '{raw}': {error}"))?
        .compile_matcher();
    if anchored || has_separator {
        Ok(CodeownersMatcher::Path(glob))
    } else {
        Ok(CodeownersMatcher::Component(glob))
    }
}

fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for character in line.chars() {
        if escaped {
            current.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }
    if escaped {
        current.push('\\');
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}
