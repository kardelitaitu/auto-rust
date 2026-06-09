//! Data file operations for `TaskContext`.

use anyhow::Result;
use serde::Serialize;

use crate::runtime::task_context::{FileMetadata, TaskContext};

impl TaskContext {
    pub fn read_data_file(&self, relative_path: &str) -> Result<String> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_read_data {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_read_data' permission",
                self.session_id
            ));
        }
        let path = crate::task::security::validate_data_path(relative_path)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        std::fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("Failed to read file: {e}"))
    }

    pub fn write_data_file(&self, relative_path: &str, content: &[u8]) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_write_data {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_write_data' permission",
                self.session_id
            ));
        }
        if !crate::task::security::is_safe_path(relative_path) {
            return Err(anyhow::anyhow!(
                "Invalid path: Path contains unsafe components"
            ));
        }
        let path = std::path::Path::new("config").join(relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("Failed to create directory: {e}"))?;
        }
        std::fs::write(&path, content).map_err(|e| anyhow::anyhow!("Failed to write file: {e}"))
    }

    pub fn list_data_files(&self, subdir: Option<&str>) -> Result<Vec<String>> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_read_data {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_read_data' permission",
                self.session_id
            ));
        }
        let base_path = if let Some(s) = subdir {
            if !crate::task::security::is_safe_path(s) {
                return Err(anyhow::anyhow!("Invalid subdir: Unsafe path components"));
            }
            std::path::Path::new("config").join(s)
        } else {
            std::path::Path::new("config").to_path_buf()
        };
        let mut files = Vec::new();
        if base_path.exists() {
            for entry in std::fs::read_dir(&base_path)
                .map_err(|e| anyhow::anyhow!("Failed to read directory: {e}"))?
            {
                let entry = entry.map_err(|e| anyhow::anyhow!("Directory entry error: {e}"))?;
                if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    if let Some(name) = entry.file_name().to_str() {
                        files.push(name.to_string());
                    }
                }
            }
        }
        Ok(files)
    }

    pub fn data_file_exists(&self, relative_path: &str) -> Result<bool> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_read_data {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_read_data' permission",
                self.session_id
            ));
        }
        let path = crate::task::security::validate_data_path(relative_path)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(path.exists())
    }

    pub fn delete_data_file(&self, relative_path: &str) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_write_data {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_write_data' permission",
                self.session_id
            ));
        }
        let path = crate::task::security::validate_data_path(relative_path)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        if !path.exists() {
            return Err(anyhow::anyhow!("File not found: {relative_path}"));
        }
        std::fs::remove_file(&path).map_err(|e| anyhow::anyhow!("Failed to delete file: {e}"))
    }

    pub fn append_data_file(&self, relative_path: &str, content: &[u8]) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_write_data {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_write_data' permission",
                self.session_id
            ));
        }
        if !crate::task::security::is_safe_path(relative_path) {
            return Err(anyhow::anyhow!(
                "Invalid path: Path contains unsafe components"
            ));
        }
        let path = std::path::Path::new("config").join(relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("Failed to create directory: {e}"))?;
        }
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| anyhow::anyhow!("Failed to open file for append: {e}"))?;
        file.write_all(content)
            .map_err(|e| anyhow::anyhow!("Failed to append to file: {e}"))
    }

    pub fn read_json_data<T: serde::de::DeserializeOwned>(&self, relative_path: &str) -> Result<T> {
        let content = self.read_data_file(relative_path)?;
        serde_json::from_str(&content).map_err(|e| anyhow::anyhow!("Failed to parse JSON: {e}"))
    }

    pub fn write_json_data<T: Serialize>(&self, relative_path: &str, data: &T) -> Result<()> {
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| anyhow::anyhow!("Failed to serialize to JSON: {e}"))?;
        self.write_data_file(relative_path, json.as_bytes())
    }

    pub fn data_file_metadata(&self, relative_path: &str) -> Result<FileMetadata> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_read_data {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_read_data' permission",
                self.session_id
            ));
        }
        let path = crate::task::security::validate_data_path(relative_path)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let metadata = std::fs::metadata(&path)
            .map_err(|e| anyhow::anyhow!("Failed to read metadata: {e}"))?;
        let modified = metadata
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let created = metadata
            .created()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        Ok(FileMetadata {
            size: metadata.len(),
            modified,
            created,
        })
    }
}
