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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_real_filesystem_write_and_read() {
        let dir = std::env::temp_dir().join(format!(
            "bacon-pipeline-test-{}-traits",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();

        let fs_impl = RealFileSystem;
        let path = dir.join("hello.txt");
        fs_impl.write(&path, "hello world").unwrap();

        let content = fs_impl.read_to_string(&path).unwrap();
        assert_eq!(content, "hello world");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_real_filesystem_exists() {
        let fs_impl = RealFileSystem;
        let missing = std::env::temp_dir().join(format!(
            "bacon-pipeline-test-{}-does-not-exist",
            std::process::id()
        ));
        assert!(!fs_impl.exists(&missing));

        let dir = std::env::temp_dir().join(format!(
            "bacon-pipeline-test-{}-exists-check",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        assert!(fs_impl.exists(&dir));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_real_filesystem_rename() {
        let dir = std::env::temp_dir().join(format!(
            "bacon-pipeline-test-{}-rename",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();

        let fs_impl = RealFileSystem;
        let from = dir.join("old.txt");
        let to = dir.join("new.txt");
        fs_impl.write(&from, "data").unwrap();
        fs_impl.rename(&from, &to).unwrap();

        assert!(!fs_impl.exists(&from));
        assert!(fs_impl.exists(&to));
        assert_eq!(fs_impl.read_to_string(&to).unwrap(), "data");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_real_filesystem_copy() {
        let dir = std::env::temp_dir().join(format!(
            "bacon-pipeline-test-{}-copy",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();

        let fs_impl = RealFileSystem;
        let src = dir.join("src.txt");
        let dst = dir.join("dst.txt");
        fs_impl.write(&src, "copy me").unwrap();
        fs_impl.copy(&src, &dst).unwrap();

        assert!(fs_impl.exists(&src));
        assert!(fs_impl.exists(&dst));
        assert_eq!(fs_impl.read_to_string(&dst).unwrap(), "copy me");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_real_filesystem_create_dir_all() {
        let dir = std::env::temp_dir().join(format!(
            "bacon-pipeline-test-{}-mkdir/nested/deep",
            std::process::id()
        ));
        let fs_impl = RealFileSystem;
        fs_impl.create_dir_all(&dir).unwrap();
        assert!(dir.exists());

        let _ = fs::remove_dir_all(
            dir.parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .unwrap(),
        );
    }
}
