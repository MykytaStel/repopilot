use crate::findings::types::Finding;
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};

pub trait FileAudit {
    fn audit(&self, file: &FileFacts, config: &ScanConfig) -> Vec<Finding>;
}

pub trait ProjectAudit {
    fn audit(&self, facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding>;
}
