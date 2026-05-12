#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageKind {
    Rust,
    TypeScript,
    JavaScript,
    CSharp,
    Python,
    Go,
    Java,
    Kotlin,
    Swift,
    C,
    Cpp,
    CHeader,
    Php,
    Ruby,
    Dart,
    Scala,
    Shell,
    PowerShell,
    Sql,
    Html,
    Css,
    Scss,
    Elixir,
    Erlang,
    Haskell,
    OCaml,
    FSharp,
    R,
    Julia,
    Lua,
    Perl,
    Zig,
    Solidity,
    ObjectiveC,
    Terraform,
    Dockerfile,
    Nix,
    Json,
    Toml,
    Yaml,
    Markdown,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameworkKind {
    React,
    ReactNative,
    NextJs,
    Expo,
    Vue,
    Angular,
    Svelte,
    NestJs,
    Express,
    Unity,
    DotNet,
    NodeJs,
    Django,
    Flask,
    FastApi,
    Gin,
    Echo,
    Fiber,
    Spring,
    Android,
    Flutter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileRole {
    ReactComponent,
    ReactHook,
    AppEntrypoint,
    FrameworkComponent,
    FrameworkHook,
    FrameworkService,
    FrameworkController,
    UnityMonoBehaviour,
    DotNetController,
    DotNetService,
    RustTest,
    Test,
    Config,
    Generated,
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
    Python,
    Go,
    Jvm,
    Android,
    Ios,
    Shell,
    Native,
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
        !self.is_test && !self.has_role(FileRole::Config) && !self.has_role(FileRole::Generated)
    }

    pub fn language_id(&self) -> &'static str {
        self.language.as_id()
    }

    pub fn framework_ids(&self) -> Vec<&'static str> {
        self.frameworks
            .iter()
            .map(|framework| framework.as_id())
            .collect()
    }

    pub fn role_ids(&self) -> Vec<&'static str> {
        self.roles.iter().map(|role| role.as_id()).collect()
    }

    pub fn paradigm_ids(&self) -> Vec<&'static str> {
        self.paradigms
            .iter()
            .map(|paradigm| paradigm.as_id())
            .collect()
    }

    pub fn runtime_ids(&self) -> Vec<&'static str> {
        self.runtimes
            .iter()
            .map(|runtime| runtime.as_id())
            .collect()
    }
}

impl LanguageKind {
    pub fn as_id(self) -> &'static str {
        match self {
            LanguageKind::Rust => "rust",
            LanguageKind::TypeScript => "typescript",
            LanguageKind::JavaScript => "javascript",
            LanguageKind::CSharp => "csharp",
            LanguageKind::Python => "python",
            LanguageKind::Go => "go",
            LanguageKind::Java => "java",
            LanguageKind::Kotlin => "kotlin",
            LanguageKind::Swift => "swift",
            LanguageKind::C => "c",
            LanguageKind::Cpp => "cpp",
            LanguageKind::CHeader => "c-header",
            LanguageKind::Php => "php",
            LanguageKind::Ruby => "ruby",
            LanguageKind::Dart => "dart",
            LanguageKind::Scala => "scala",
            LanguageKind::Shell => "shell",
            LanguageKind::PowerShell => "powershell",
            LanguageKind::Sql => "sql",
            LanguageKind::Html => "html",
            LanguageKind::Css => "css",
            LanguageKind::Scss => "scss",
            LanguageKind::Elixir => "elixir",
            LanguageKind::Erlang => "erlang",
            LanguageKind::Haskell => "haskell",
            LanguageKind::OCaml => "ocaml",
            LanguageKind::FSharp => "fsharp",
            LanguageKind::R => "r",
            LanguageKind::Julia => "julia",
            LanguageKind::Lua => "lua",
            LanguageKind::Perl => "perl",
            LanguageKind::Zig => "zig",
            LanguageKind::Solidity => "solidity",
            LanguageKind::ObjectiveC => "objective-c",
            LanguageKind::Terraform => "terraform",
            LanguageKind::Dockerfile => "dockerfile",
            LanguageKind::Nix => "nix",
            LanguageKind::Json => "json",
            LanguageKind::Toml => "toml",
            LanguageKind::Yaml => "yaml",
            LanguageKind::Markdown => "markdown",
            LanguageKind::Unknown => "unknown",
        }
    }
}

impl FrameworkKind {
    pub fn as_id(self) -> &'static str {
        match self {
            FrameworkKind::React => "react",
            FrameworkKind::ReactNative => "react-native",
            FrameworkKind::NextJs => "nextjs",
            FrameworkKind::Expo => "expo",
            FrameworkKind::Vue => "vue",
            FrameworkKind::Angular => "angular",
            FrameworkKind::Svelte => "svelte",
            FrameworkKind::NestJs => "nestjs",
            FrameworkKind::Express => "express",
            FrameworkKind::Unity => "unity",
            FrameworkKind::DotNet => "dotnet",
            FrameworkKind::NodeJs => "nodejs",
            FrameworkKind::Django => "django",
            FrameworkKind::Flask => "flask",
            FrameworkKind::FastApi => "fastapi",
            FrameworkKind::Gin => "gin",
            FrameworkKind::Echo => "echo",
            FrameworkKind::Fiber => "fiber",
            FrameworkKind::Spring => "spring",
            FrameworkKind::Android => "android",
            FrameworkKind::Flutter => "flutter",
        }
    }
}

impl FileRole {
    pub fn as_id(self) -> &'static str {
        match self {
            FileRole::ReactComponent => "react-component",
            FileRole::ReactHook => "react-hook",
            FileRole::AppEntrypoint => "app-entrypoint",
            FileRole::FrameworkComponent => "framework-component",
            FileRole::FrameworkHook => "framework-hook",
            FileRole::FrameworkService => "framework-service",
            FileRole::FrameworkController => "framework-controller",
            FileRole::UnityMonoBehaviour => "unity-monobehaviour",
            FileRole::DotNetController => "dotnet-controller",
            FileRole::DotNetService => "dotnet-service",
            FileRole::RustTest => "rust-test",
            FileRole::Test => "test",
            FileRole::Config => "config",
            FileRole::Generated => "generated",
            FileRole::Domain => "domain",
            FileRole::Script => "script",
            FileRole::Unknown => "unknown",
        }
    }
}

impl ProgrammingParadigm {
    pub fn as_id(self) -> &'static str {
        match self {
            ProgrammingParadigm::Functional => "functional",
            ProgrammingParadigm::ObjectOriented => "object-oriented",
            ProgrammingParadigm::Procedural => "procedural",
            ProgrammingParadigm::DeclarativeUi => "declarative-ui",
            ProgrammingParadigm::Reactive => "reactive",
            ProgrammingParadigm::DataOriented => "data-oriented",
            ProgrammingParadigm::Mixed => "mixed",
            ProgrammingParadigm::Unknown => "unknown",
        }
    }
}

impl RuntimeKind {
    pub fn as_id(self) -> &'static str {
        match self {
            RuntimeKind::Browser => "browser",
            RuntimeKind::Node => "node",
            RuntimeKind::ReactNative => "react-native",
            RuntimeKind::DotNet => "dotnet",
            RuntimeKind::Unity => "unity",
            RuntimeKind::RustCli => "rust-cli",
            RuntimeKind::RustLibrary => "rust-library",
            RuntimeKind::Python => "python",
            RuntimeKind::Go => "go",
            RuntimeKind::Jvm => "jvm",
            RuntimeKind::Android => "android",
            RuntimeKind::Ios => "ios",
            RuntimeKind::Shell => "shell",
            RuntimeKind::Native => "native",
            RuntimeKind::Unknown => "unknown",
        }
    }
}
