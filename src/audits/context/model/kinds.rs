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
    /// A Rust test-support module (`testutil.rs`, `test_utils.rs`, …): a
    /// production file whose `panic!`/`unwrap` calls are test assertion plumbing.
    /// Carried *alongside* the file's production role so only opted-in rules
    /// (currently `rust.panic-risk`) treat it specially.
    TestSupport,
    /// A CLI command handler: a file in a `commands/` directory whose package
    /// declares an executable entrypoint (npm `package.json#bin`, Cargo
    /// `[[bin]]`/`src/bin`). Such a command owns its own exit code, so
    /// host-termination calls there are an intended boundary — unlike a reusable
    /// module elsewhere in the same package, which is not exempted.
    CliExecutable,
    /// A build-tooling module: a Gradle convention plugin or build script under
    /// `build-logic/` / `buildSrc/`. It configures the build, never ships in the
    /// app, so a `throw`/`TODO()` there fails the build by design rather than at
    /// runtime. Carried alongside the production role so only opted-in rules
    /// treat it specially.
    BuildTooling,
    Infrastructure,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgrammingParadigm {
    Functional,
    ObjectOriented,
    Procedural,
    Declarative,
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
    Infrastructure,
    Unknown,
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
            FileRole::TestSupport => "test-support",
            FileRole::CliExecutable => "cli-executable",
            FileRole::BuildTooling => "build-tooling",
            FileRole::Infrastructure => "infrastructure",
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
            ProgrammingParadigm::Declarative => "declarative",
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
            RuntimeKind::Infrastructure => "infrastructure",
            RuntimeKind::Unknown => "unknown",
        }
    }
}
