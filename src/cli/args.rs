use clap::ValueEnum;
use repopilot::baseline::gate::FailOn;
use repopilot::findings::types::Severity;
use repopilot::output::OutputFormat;

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
