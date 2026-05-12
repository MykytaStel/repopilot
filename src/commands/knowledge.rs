use crate::cli::{CompareOutputFormatArg, KnowledgeSectionArg};
use repopilot::knowledge::{
    KnowledgeCatalogSection, build_knowledge_catalog_report, render_knowledge_catalog_report,
};
use repopilot::output::OutputFormat;
use repopilot::report::writer::write_report;
use std::path::PathBuf;

pub fn run(
    section: KnowledgeSectionArg,
    format: CompareOutputFormatArg,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = build_knowledge_catalog_report(section.into());
    let report = report?;

    let rendered = render_knowledge_catalog_report(&report, OutputFormat::from(format))?;
    write_report(&rendered, output.as_deref())?;

    Ok(())
}

impl From<KnowledgeSectionArg> for KnowledgeCatalogSection {
    fn from(section: KnowledgeSectionArg) -> Self {
        match section {
            KnowledgeSectionArg::All => KnowledgeCatalogSection::All,
            KnowledgeSectionArg::Languages => KnowledgeCatalogSection::Languages,
            KnowledgeSectionArg::Frameworks => KnowledgeCatalogSection::Frameworks,
            KnowledgeSectionArg::Runtimes => KnowledgeCatalogSection::Runtimes,
            KnowledgeSectionArg::Paradigms => KnowledgeCatalogSection::Paradigms,
            KnowledgeSectionArg::Rules => KnowledgeCatalogSection::Rules,
        }
    }
}
