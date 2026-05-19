use clap::ValueEnum;
use repopilot::baseline::gate::FailOn;
use repopilot::findings::types::{Confidence, Severity};
use repopilot::output::OutputFormat;
use repopilot::risk::RiskPriority;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OutputFormatArg {
    Console,
    Html,
    Json,
    Markdown,
    Sarif,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum CompareOutputFormatArg {
    Console,
    Json,
    Markdown,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum SeverityArg {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ConfidenceArg {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum PriorityArg {
    P0,
    P1,
    P2,
    P3,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub enum ScanProfileArg {
    Default,
    Strict,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum KnowledgeSectionArg {
    All,
    Languages,
    Frameworks,
    Runtimes,
    Paradigms,
    Rules,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum FailOnArg {
    NewLow,
    NewMedium,
    NewHigh,
    NewCritical,
    Low,
    Medium,
    High,
    Critical,
}

impl From<CompareOutputFormatArg> for OutputFormat {
    fn from(format: CompareOutputFormatArg) -> Self {
        match format {
            CompareOutputFormatArg::Console => OutputFormat::Console,
            CompareOutputFormatArg::Json => OutputFormat::Json,
            CompareOutputFormatArg::Markdown => OutputFormat::Markdown,
        }
    }
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(format: OutputFormatArg) -> Self {
        match format {
            OutputFormatArg::Console => OutputFormat::Console,
            OutputFormatArg::Html => OutputFormat::Html,
            OutputFormatArg::Json => OutputFormat::Json,
            OutputFormatArg::Markdown => OutputFormat::Markdown,
            OutputFormatArg::Sarif => OutputFormat::Sarif,
        }
    }
}

impl From<ConfidenceArg> for Confidence {
    fn from(value: ConfidenceArg) -> Self {
        match value {
            ConfidenceArg::Low => Confidence::Low,
            ConfidenceArg::Medium => Confidence::Medium,
            ConfidenceArg::High => Confidence::High,
        }
    }
}

impl From<PriorityArg> for RiskPriority {
    fn from(value: PriorityArg) -> Self {
        match value {
            PriorityArg::P0 => RiskPriority::P0,
            PriorityArg::P1 => RiskPriority::P1,
            PriorityArg::P2 => RiskPriority::P2,
            PriorityArg::P3 => RiskPriority::P3,
        }
    }
}

impl From<FailOnArg> for FailOn {
    fn from(value: FailOnArg) -> Self {
        match value {
            FailOnArg::NewLow => FailOn::New(Severity::Low),
            FailOnArg::NewMedium => FailOn::New(Severity::Medium),
            FailOnArg::NewHigh => FailOn::New(Severity::High),
            FailOnArg::NewCritical => FailOn::New(Severity::Critical),
            FailOnArg::Low => FailOn::Any(Severity::Low),
            FailOnArg::Medium => FailOn::Any(Severity::Medium),
            FailOnArg::High => FailOn::Any(Severity::High),
            FailOnArg::Critical => FailOn::Any(Severity::Critical),
        }
    }
}

pub fn parse_vibe_budget(value: &str) -> Result<usize, String> {
    let tokens = match value {
        "2k" => 2048,
        "4k" => 4096,
        "8k" => 8192,
        "16k" => 16384,
        other => other
            .parse::<usize>()
            .map_err(|_| "expected 2k, 4k, 8k, 16k, or a positive token count".to_string())?,
    };

    if tokens == 0 {
        return Err("budget must be greater than zero".to_string());
    }

    Ok(tokens)
}

pub fn parse_byte_size(value: &str) -> Result<u64, String> {
    let s = value.to_lowercase();
    if let Some(n) = s.strip_suffix("gb") {
        return n
            .trim()
            .parse::<u64>()
            .map(|n| n << 30)
            .map_err(|_| format!("invalid: {value}"));
    }
    if let Some(n) = s.strip_suffix("mb") {
        return n
            .trim()
            .parse::<u64>()
            .map(|n| n << 20)
            .map_err(|_| format!("invalid: {value}"));
    }
    if let Some(n) = s.strip_suffix("kb") {
        return n
            .trim()
            .parse::<u64>()
            .map(|n| n << 10)
            .map_err(|_| format!("invalid: {value}"));
    }
    value
        .parse::<u64>()
        .map_err(|_| "expected bytes, e.g. 512, 1mb, 2gb".to_string())
}
