//! Spec filesystem operations — canonical shared implementation.
//!
//! Consolidates duplicated code from `bacon_agent_pi/spec_io.rs` and
//! `bacon_agent_nvidia/spec_io.rs` into a single location.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_yml::{Mapping, Value};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Metadata stored in spec.yaml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecMeta {
    pub id: String,
    pub title: String,
    pub status: String,
    pub owner: String,
    pub implementer: String,
    pub priority: String,
}

/// Root specs directory: `docs/specs`.
pub fn specs_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("docs/specs");
    p
}

/// Active specs directory: `docs/specs/_active`.
pub fn active_dir() -> PathBuf {
    let mut p = specs_root();
    p.push("_active");
    p
}

/// Done specs directory: `docs/specs/_done`.
pub fn done_dir() -> PathBuf {
    let mut p = specs_root();
    p.push("_done");
    p
}

/// List all active spec directories sorted by name.
pub fn list_active_specs() -> Result<Vec<PathBuf>> {
    let dir = active_dir();
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut entries: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();
    entries.sort();
    Ok(entries)
}

/// Find the first approved spec in `_active/` (sorted by name, FIFO).
/// Returns `(spec_path, metadata)` if one exists.
pub fn find_approved_spec() -> Result<Option<(PathBuf, SpecMeta)>> {
    for spec_path in list_active_specs()? {
        if let Ok(meta) = read_spec_meta(&spec_path) {
            if meta.status == "approved" {
                return Ok(Some((spec_path, meta)));
            }
        }
    }
    Ok(None)
}

/// Read and parse `spec.yaml` from a spec directory.
pub fn read_spec_meta(path: &Path) -> Result<SpecMeta> {
    let yaml_path = path.join("spec.yaml");
    let content = std::fs::read_to_string(&yaml_path)
        .with_context(|| format!("reading spec.yaml from {}", path.display()))?;
    serde_yml::from_str(&content)
        .with_context(|| format!("parsing spec.yaml from {}", path.display()))
}

/// Write updated metadata into `spec.yaml`, preserving unknown fields.
/// Uses atomic temp-file + rename to prevent partial writes.
pub fn write_spec_meta(path: &Path, meta: &SpecMeta) -> Result<()> {
    let yaml_path = path.join("spec.yaml");
    let tmp_path = path.join("spec.yaml.tmp");
    let mut value = std::fs::read_to_string(&yaml_path)
        .ok()
        .and_then(|content| serde_yml::from_str::<Value>(&content).ok())
        .unwrap_or_else(|| Value::Mapping(Mapping::new()));

    let mapping = match &mut value {
        Value::Mapping(mapping) => mapping,
        _ => {
            value = Value::Mapping(Mapping::new());
            match &mut value {
                Value::Mapping(mapping) => mapping,
                _ => unreachable!(),
            }
        }
    };

    set_yaml_string(mapping, "id", &meta.id);
    set_yaml_string(mapping, "title", &meta.title);
    set_yaml_string(mapping, "status", &meta.status);
    set_yaml_string(mapping, "owner", &meta.owner);
    set_yaml_string(mapping, "implementer", &meta.implementer);
    set_yaml_string(mapping, "priority", &meta.priority);

    let content = serde_yml::to_string(&value)?;
    // Write to temp file first, then atomically rename — prevents partial writes
    std::fs::write(&tmp_path, &content)
        .with_context(|| format!("writing {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &yaml_path)
        .with_context(|| format!("renaming {} to {}", tmp_path.display(), yaml_path.display()))?;
    Ok(())
}

fn set_yaml_string(mapping: &mut Mapping, key: &str, value: &str) {
    mapping.insert(
        Value::String(key.to_string()),
        Value::String(value.to_string()),
    );
}

/// Compute the next available spec number by scanning both `_active/` and `_done/`.
/// Used only as a hint for `allocate_spec_dir` — not race-safe on its own.
pub fn next_spec_number() -> Result<u32> {
    let mut max_n = 0u32;
    for dir in [active_dir(), done_dir()] {
        if !dir.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(rest) = name.strip_prefix(|c: char| c.is_ascii_digit()) {
                if let Some(rest) = rest.strip_prefix('-') {
                    if let Some(n) = rest.split('-').next() {
                        if let Ok(n) = n.parse::<u32>() {
                            max_n = max_n.max(n);
                        }
                    }
                }
            }
            if let Some(n) = name.split('-').next() {
                if let Ok(n) = n.parse::<u32>() {
                    max_n = max_n.max(n);
                }
            }
        }
    }
    Ok(max_n + 1)
}

/// Path to the counter hint file used by `allocate_spec_dir`.
fn counter_path() -> PathBuf {
    specs_root().join(".counter")
}

/// Read the current counter hint. Returns 0 if file is missing or corrupt.
fn load_counter() -> u32 {
    let path = counter_path();
    let mut buf = String::new();
    std::fs::File::open(&path)
        .and_then(|mut f| f.read_to_string(&mut buf))
        .ok()
        .and_then(|_| buf.trim().parse::<u32>().ok())
        .unwrap_or(0)
}

/// Write a counter hint (best-effort, non-atomic — used only as a starting hint).
fn save_counter(value: u32) {
    let path = counter_path();
    if let Ok(mut f) = std::fs::File::create(&path) {
        let _ = write!(f, "{}", value);
    }
}

/// Atomically allocate a numbered spec directory under `active` with the given `slug`.
///
/// Uses `std::fs::create_dir` which fails atomically if the directory already
/// exists. If a collision occurs, increments the number and retries. The
/// counter file at `docs/specs/.counter` provides a starting hint to keep
/// retries O(1) amortized but is NOT used for atomicity — concurrent callers
/// that read the same hint simply retry until they find a free slot.
///
/// Returns `(spec_directory_path, allocated_number)`.
pub fn allocate_spec_dir(active: &Path, slug: &str) -> Result<(PathBuf, u32)> {
    let hint = load_counter().max(1);
    let mut number = hint;
    loop {
        let dir_name = format!("{:04}-{}", number, slug);
        let spec_dir = active.join(&dir_name);
        match std::fs::create_dir(&spec_dir) {
            Ok(()) => {
                if number > hint {
                    save_counter(number);
                }
                return Ok((spec_dir, number));
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                number += 1;
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    }
}

/// Move a spec directory from `_active/` to `_done/`.
pub fn move_to_done(path: &Path) -> Result<()> {
    let done = done_dir();
    std::fs::create_dir_all(&done)?;
    let name = path.file_name().context("spec path has no file name")?;
    let dest = done.join(name);
    std::fs::rename(path, &dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_spec_meta_preserves_schema_fields() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let spec = temp.path().join("0001-preserve-schema");
        std::fs::create_dir_all(&spec)?;
        std::fs::write(
            spec.join("spec.yaml"),
            "version: 1\n\
             id: 0001-preserve-schema\n\
             title: Preserve schema\n\
             status: approved\n\
             owner: pipeline\n\
             implementer: pipeline\n\
             priority: P2\n\
             area:\n\
               - bacon\n\
             acceptance:\n\
               - Keep extra fields\n",
        )?;

        let mut meta = read_spec_meta(&spec)?;
        meta.status = "implemented".to_string();
        write_spec_meta(&spec, &meta)?;

        let updated: Value =
            serde_yml::from_str(&std::fs::read_to_string(spec.join("spec.yaml"))?)?;
        assert_eq!(updated["status"], Value::String("implemented".to_string()));
        assert_eq!(updated["area"][0], Value::String("bacon".to_string()));
        assert_eq!(
            updated["acceptance"][0],
            Value::String("Keep extra fields".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_specs_root_ends_with_docs_specs() {
        let root = specs_root();
        let path_str = root.to_string_lossy();
        assert!(
            path_str.contains("docs"),
            "specs_root should contain 'docs', got: {}",
            path_str
        );
    }

    #[test]
    fn test_active_dir_is_child_of_specs_root() {
        let root = specs_root();
        let active = active_dir();
        assert!(active.starts_with(&root));
        assert!(active.to_string_lossy().contains("_active"));
    }

    #[test]
    fn test_done_dir_is_child_of_specs_root() {
        let root = specs_root();
        let done = done_dir();
        assert!(done.starts_with(&root));
        assert!(done.to_string_lossy().contains("_done"));
    }

    #[test]
    fn test_spec_meta_serialize_roundtrip() {
        let meta = SpecMeta {
            id: "test-001".to_string(),
            title: "Test Spec".to_string(),
            status: "active".to_string(),
            owner: "tester".to_string(),
            implementer: "coder".to_string(),
            priority: "high".to_string(),
        };
        let yaml = serde_yml::to_string(&meta).expect("serialize");
        let restored: SpecMeta = serde_yml::from_str(&yaml).expect("deserialize");
        assert_eq!(restored.id, meta.id);
        assert_eq!(restored.title, meta.title);
        assert_eq!(restored.status, meta.status);
        assert_eq!(restored.owner, meta.owner);
        assert_eq!(restored.implementer, meta.implementer);
        assert_eq!(restored.priority, meta.priority);
    }

    #[test]
    fn test_list_active_specs_returns_ok() {
        let result = list_active_specs();
        // Should not error (returns empty vec if dir doesn't exist or has no subdirs)
        assert!(result.is_ok());
    }

    #[test]
    fn test_spec_meta_debug() {
        let meta = SpecMeta {
            id: "id".to_string(),
            title: "t".to_string(),
            status: "s".to_string(),
            owner: "o".to_string(),
            implementer: "i".to_string(),
            priority: "p".to_string(),
        };
        let debug = format!("{:?}", meta);
        assert!(debug.contains("SpecMeta"));
    }
}
