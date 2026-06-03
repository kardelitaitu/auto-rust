//! Traits for abstracting side-effecting operations to enable unit testing.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;
use std::process::Command;

/// Abstraction for filesystem operations.
#[async_trait]
pub trait FileSystem: Send + Sync {
    fn read_to_string(&self, path: &Path) -> Result<String>;
    fn write(&self, path: &Path, content: &str) -> Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn create_dir_all(&self, path: &Path) -> Result<()>;
    fn rename(&self, from: &Path, to: &Path) -> Result<()>;
    fn copy(&self, from: &Path, to: &Path) -> Result<()>;
}

pub struct RealFileSystem;

#[async_trait]
impl FileSystem for RealFileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String> {
        std::fs::read_to_string(path).context("FileSystem::read_to_string")
    }
    fn write(&self, path: &Path, content: &str) -> Result<()> {
        std::fs::write(path, content).context("FileSystem::write")
    }
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
    fn create_dir_all(&self, path: &Path) -> Result<()> {
        std::fs::create_dir_all(path).context("FileSystem::create_dir_all")
    }
    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        std::fs::rename(from, to).context("FileSystem::rename")
    }
    fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        std::fs::copy(from, to)
            .context("FileSystem::copy")
            .map(|_| ())
    }
}

/// Abstraction for external command execution (git, validation scripts).
#[async_trait]
pub trait CommandRunner: Send + Sync {
    fn run(&self, command: &str, args: &[String], dir: Option<&Path>) -> Result<(bool, String)>;
}

pub struct RealCommandRunner;

#[async_trait]
impl CommandRunner for RealCommandRunner {
    fn run(&self, command: &str, args: &[String], dir: Option<&Path>) -> Result<(bool, String)> {
        let mut cmd = Command::new(command);
        cmd.args(args);
        if let Some(d) = dir {
            cmd.current_dir(d);
        }
        let output = cmd.output().context("CommandRunner::run")?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = if stderr.is_empty() {
            stdout
        } else {
            format!("{stdout}\n{stderr}")
        };
        Ok((output.status.success(), combined))
    }
}

/// Abstraction for LLM API interactions.
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, messages: Vec<crate::llm::models::ChatMessage>) -> Result<String>;
}

pub struct RealLlmClient {
    client: crate::llm::LlmClient,
}

impl RealLlmClient {
    pub fn new(config: crate::llm::models::NvidiaConfig) -> Self {
        Self {
            client: crate::llm::LlmClient::new(config),
        }
    }
}

#[async_trait]
impl LlmClient for RealLlmClient {
    async fn chat(&self, messages: Vec<crate::llm::models::ChatMessage>) -> Result<String> {
        self.client.chat(messages).await
    }
}
