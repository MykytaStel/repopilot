use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FileRole {
    Production,
    Test,
    Generated,
    Config,
    Documentation,
    Fixture,
}
