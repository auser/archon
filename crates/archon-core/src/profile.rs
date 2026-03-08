use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Repository profile determines which base docs get generated during init.
#[derive(Debug, Clone, Copy, ValueEnum, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    /// Base Rust workspace — minimal docs
    RustWorkspace,
    /// Runtime system — adds runtime, performance, security docs
    RuntimeSystem,
    /// Compiler/AI system — adds IR, pipeline, backend docs
    CompilerAi,
    /// CLI tool — adds CLI-specific docs
    CliTool,
    /// Service application — adds API, deployment, ops docs
    ServiceApp,
}

impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RustWorkspace => write!(f, "rust-workspace"),
            Self::RuntimeSystem => write!(f, "runtime-system"),
            Self::CompilerAi => write!(f, "compiler-ai"),
            Self::CliTool => write!(f, "cli-tool"),
            Self::ServiceApp => write!(f, "service-app"),
        }
    }
}
