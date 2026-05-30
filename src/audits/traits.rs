use crate::analysis::parse::ParsedFile;
use crate::findings::types::Finding;
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};

pub trait FileAudit: Send + Sync {
    fn audit(&self, file: &FileFacts, config: &ScanConfig) -> Vec<Finding>;

    /// Audits a file with a shared, parse-once [`ParsedFile`] view. The scan
    /// pipeline calls this so AST-based audits can reuse one syntax tree per
    /// file instead of each re-parsing. Audits that need an AST override this;
    /// the default ignores the parsed view and delegates to [`FileAudit::audit`].
    fn audit_parsed(
        &self,
        file: &FileFacts,
        _parsed: &ParsedFile,
        config: &ScanConfig,
    ) -> Vec<Finding> {
        self.audit(file, config)
    }
}

pub trait ProjectAudit {
    fn audit(&self, facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding>;
}
