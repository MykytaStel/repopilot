#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageKind {
    Rust,
    TypeScript,
    JavaScript,
    CSharp,
    Python,
    Go,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameworkKind {
    React,
    ReactNative,
    NextJs,
    Unity,
    DotNet,
    NodeJs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileRole {
    ReactComponent,
    ReactHook,
    UnityMonoBehaviour,
    DotNetController,
    DotNetService,
    RustTest,
    Test,
    Config,
    Domain,
    Script,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditContext {
    pub language: LanguageKind,
    pub frameworks: Vec<FrameworkKind>,
    pub roles: Vec<FileRole>,
    pub is_test: bool,
}

impl AuditContext {
    pub fn has_framework(&self, framework: FrameworkKind) -> bool {
        self.frameworks.contains(&framework)
    }

    pub fn has_role(&self, role: FileRole) -> bool {
        self.roles.contains(&role)
    }

    pub fn is_react_component(&self) -> bool {
        self.has_role(FileRole::ReactComponent)
    }

    pub fn is_react_hook(&self) -> bool {
        self.has_role(FileRole::ReactHook)
    }

    pub fn is_unity_file(&self) -> bool {
        self.has_framework(FrameworkKind::Unity)
    }

    pub fn is_dotnet_file(&self) -> bool {
        self.has_framework(FrameworkKind::DotNet)
    }

    pub fn is_production_code(&self) -> bool {
        !self.is_test && !self.has_role(FileRole::Config)
    }
}
