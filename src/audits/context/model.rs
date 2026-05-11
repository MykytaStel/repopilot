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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgrammingParadigm {
    Functional,
    ObjectOriented,
    Procedural,
    DeclarativeUi,
    Reactive,
    DataOriented,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKind {
    Browser,
    Node,
    ReactNative,
    DotNet,
    Unity,
    RustCli,
    RustLibrary,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditContext {
    pub language: LanguageKind,
    pub frameworks: Vec<FrameworkKind>,
    pub roles: Vec<FileRole>,
    pub paradigms: Vec<ProgrammingParadigm>,
    pub runtimes: Vec<RuntimeKind>,
    pub is_test: bool,
}

impl AuditContext {
    pub fn has_framework(&self, framework: FrameworkKind) -> bool {
        self.frameworks.contains(&framework)
    }

    pub fn has_role(&self, role: FileRole) -> bool {
        self.roles.contains(&role)
    }

    pub fn has_paradigm(&self, paradigm: ProgrammingParadigm) -> bool {
        self.paradigms.contains(&paradigm)
    }

    pub fn has_runtime(&self, runtime: RuntimeKind) -> bool {
        self.runtimes.contains(&runtime)
    }

    pub fn is_react_component(&self) -> bool {
        self.has_role(FileRole::ReactComponent)
    }

    pub fn is_react_hook(&self) -> bool {
        self.has_role(FileRole::ReactHook)
    }

    pub fn is_unity_file(&self) -> bool {
        self.has_framework(FrameworkKind::Unity) || self.has_runtime(RuntimeKind::Unity)
    }

    pub fn is_dotnet_file(&self) -> bool {
        self.has_framework(FrameworkKind::DotNet) || self.has_runtime(RuntimeKind::DotNet)
    }

    pub fn is_oop_code(&self) -> bool {
        self.has_paradigm(ProgrammingParadigm::ObjectOriented)
    }

    pub fn is_functional_code(&self) -> bool {
        self.has_paradigm(ProgrammingParadigm::Functional)
    }

    pub fn is_declarative_ui(&self) -> bool {
        self.has_paradigm(ProgrammingParadigm::DeclarativeUi)
    }

    pub fn is_production_code(&self) -> bool {
        !self.is_test && !self.has_role(FileRole::Config)
    }
}
