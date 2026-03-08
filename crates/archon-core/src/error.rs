use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArchonError {
    #[error("hologram.repo.yaml not found (run `archon init`)")]
    NoRepoMeta,

    #[error("invalid standards version: {0}")]
    InvalidVersion(String),

    #[error("architecture repo not found: {0}")]
    ArchRepoNotFound(String),

    #[error("policy error: {0}")]
    Policy(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
}
