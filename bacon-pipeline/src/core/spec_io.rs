//! Spec filesystem operations — canonical shared implementation.
//!
//! Consolidates canonical spec I/O operations for the bacon-pipeline crate.

use anyhow::{Context, Result};
use log::warn;
use serde::{Deserialize, Serialize};
use serde_yml::{Mapping, Value};
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

/// Root specs directory — configurable via `ProjectConfig`.
#[must_use]
pub fn specs_root() -> PathBuf {
    crate::config::project_config().specs_dir.clone()
}

/// Active specs directory: `docs/specs/_active`.
#[must_use]
pub fn active_dir() -> PathBuf {
    let mut p = specs_root();
    p.push("_active");
    p
}

/// Done specs directory: `docs/specs/_done`.
#[must_use]
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
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();
    entries.sort();
    Ok(entries)
}

/// List all approved specs in `_active/` (sorted by name).
/// Returns empty vec if none found.
pub fn list_approved_specs() -> Result<Vec<(PathBuf, SpecMeta)>> {
    let mut results = Vec::new();
    for spec_path in list_active_specs()? {
        if let Ok(meta) = read_spec_meta(&spec_path) {
            if meta.status == "approved" {
                results.push((spec_path, meta));
            }
        }
    }
    Ok(results)
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

/// Scan `_done/` and `_active/` for specs whose title overlaps with `description`.
/// Returns a list of matching spec paths with their titles for human-readable reporting.
pub fn find_specs_matching(description: &str) -> Result<Vec<(PathBuf, String)>> {
    let mut results = Vec::new();
    let keywords: Vec<&str> = description
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .collect();

    for dir in [specs_root().join("_done"), specs_root().join("_active")] {
        if !dir.is_dir() {
            continue;
        }
        for entry in
            std::fs::read_dir(&dir).map_err(|e| anyhow::anyhow!("failed to read {dir:?}: {e}"))?
        {
            let entry = entry?;
            let spec_path = entry.path();
            if !spec_path.is_dir() {
                continue;
            }
            if let Ok(meta) = read_spec_meta(&spec_path) {
                let title_lower = meta.title.to_lowercase();
                let matches = keywords
                    .iter()
                    .any(|kw| title_lower.contains(&kw.to_lowercase()));
                if matches {
                    results.push((spec_path, meta.title));
                }
            }
        }
    }
    Ok(results)
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

    let mapping = if let Value::Mapping(mapping) = &mut value {
        mapping
    } else {
        value = Value::Mapping(Mapping::new());
        match &mut value {
            Value::Mapping(mapping) => mapping,
            _ => unreachable!(),
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
    next_spec_number_in_dirs([active_dir(), done_dir()])
}

fn next_spec_number_in_dirs<I, P>(dirs: I) -> Result<u32>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut max_n = 0u32;
    for dir in dirs {
        let dir = dir.as_ref();
        if !dir.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if let Some(n) = parse_spec_number(&name_str) {
                max_n = max_n.max(n);
            }
        }
    }
    Ok(max_n + 1)
}

fn parse_spec_number(name: &str) -> Option<u32> {
    let (prefix, _) = name.split_once('-')?;
    if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    prefix.parse::<u32>().ok()
}

fn sibling_done_dir(active: &Path) -> PathBuf {
    active
        .parent()
        .map_or_else(done_dir, |root| root.join("_done"))
}

fn spec_number_exists(number: u32, dirs: &[PathBuf]) -> Result<bool> {
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if parse_spec_number(&name_str) == Some(number) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Atomically allocate a numbered spec directory under `active` with the given `slug`.
///
/// Uses `std::fs::create_dir` which fails atomically if the directory already
/// exists. The starting number is computed by scanning `_active/` and `_done/`
/// for the largest existing numeric prefix, then adding one. If a concurrent
/// caller creates the same number first, this function increments and retries.
///
/// Returns `(spec_directory_path, allocated_number)`.
pub fn allocate_spec_dir(active: &Path, slug: &str) -> Result<(PathBuf, u32)> {
    std::fs::create_dir_all(active)?;
    let done = sibling_done_dir(active);
    let numbered_dirs = [active.to_path_buf(), done];
    let mut number = next_spec_number_in_dirs(&numbered_dirs)?;
    loop {
        if spec_number_exists(number, &numbered_dirs)? {
            number += 1;
            continue;
        }
        let dir_name = format!("{number:04}-{slug}");
        let spec_dir = active.join(&dir_name);
        match std::fs::create_dir(&spec_dir) {
            Ok(()) => {
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

/// Read a spec file (plan.md, validation.md, etc.) from a spec directory.
/// Returns empty string on failure with a warning.
#[must_use]
pub fn read_spec_file(spec_path: &Path, name: &str) -> String {
    let path = spec_path.join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        warn!("Failed to read spec file {}: {}", path.display(), e);
        String::new()
    })
}

/// Move a spec directory from `_active/` to `_done/`.
///
/// Normalizes metadata to match archive conventions:
/// - Sets `status` to `"done"`.
/// - Prefixes `implementer` with `"archived-"` when it is `"pending"`.
pub fn move_to_done(path: &Path) -> Result<PathBuf> {
    let done = done_dir();
    std::fs::create_dir_all(&done)?;
    let name = path.file_name().context("spec path has no file name")?;
    let dest = done.join(name);

    // Normalize metadata before moving so the _done/ spec is consistent.
    if let Ok(mut meta) = read_spec_meta(path) {
        meta.status = "done".to_string();
        if meta.implementer == "pending" {
            meta.implementer = format!("archived-{}", meta.implementer);
        }
        if let Err(e) = write_spec_meta(path, &meta) {
            warn!("Failed to normalize spec metadata before archive: {e}");
        }
    }

    std::fs::rename(path, &dest)?;
    Ok(dest)
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
    #[ignore = "requires ProjectConfig initialization"]
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
    #[ignore = "requires ProjectConfig initialization"]
    fn test_active_dir_is_child_of_specs_root() {
        let root = specs_root();
        let active = active_dir();
        assert!(active.starts_with(&root));
        assert!(active.to_string_lossy().contains("_active"));
    }

    #[test]
    #[ignore = "requires ProjectConfig initialization"]
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
    #[ignore = "requires ProjectConfig initialization"]
    fn test_list_active_specs_returns_ok() {
        let result = list_active_specs();
        // Should not error (returns empty vec if dir doesn't exist or has no subdirs)
        assert!(result.is_ok());
    }

    #[test]
    fn allocate_spec_dir_uses_max_existing_prefix_plus_one() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let active = temp.path().join("_active");
        let done = temp.path().join("_done");
        std::fs::create_dir_all(active.join("0004-existing-active"))?;
        std::fs::create_dir_all(done.join("0005-existing-done"))?;
        std::fs::write(temp.path().join(".counter"), "99")?;

        let (spec_dir, number) = allocate_spec_dir(&active, "new-slug")?;

        assert_eq!(number, 6);
        assert_eq!(
            spec_dir.file_name().and_then(|name| name.to_str()),
            Some("0006-new-slug")
        );
        assert_eq!(std::fs::read_to_string(temp.path().join(".counter"))?, "99");

        Ok(())
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
