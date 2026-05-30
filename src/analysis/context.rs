use super::file_role::FileRole;
use super::language_family::LanguageFamily;
use super::module_kind::ModuleKind;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArchitectureContext {
    pub file_role: FileRole,
    pub module_kind: ModuleKind,
    pub language_family: LanguageFamily,
    pub is_entrypoint: bool,
    pub is_public_api: bool,
}
