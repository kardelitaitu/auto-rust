use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecMeta {
    pub id: String,
    pub title: String,
    pub status: String,
    pub owner: String,
    pub implementer: String,
    pub priority: String,
}

pub fn specs_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("docs/specs");
    p
}

pub fn active_dir() -> PathBuf {
    let mut p = specs_root();
    p.push("_active");
    p
}

pub fn done_dir() -> PathBuf {
    let mut p = specs_root();
    p.push("_done");
    p
}

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

pub fn read_spec_meta(path: &Path) -> Result<SpecMeta> {
    let yaml_path = path.join("spec.yaml");
    let content = std::fs::read_to_string(&yaml_path)
        .with_context(|| format!("reading {}", yaml_path.display()))?;
    let meta: SpecMeta = serde_yaml::from_str(&content)
        .with_context(|| format!("parsing {}", yaml_path.display()))?;
    Ok(meta)
}

pub fn write_spec_meta(path: &Path, meta: &SpecMeta) -> Result<()> {
    let yaml_path = path.join("spec.yaml");
    let content = serde_yaml::to_string(meta)?;
    std::fs::write(&yaml_path, content)
        .with_context(|| format!("writing {}", yaml_path.display()))?;
    Ok(())
}

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
            // Also try parsing just the leading number
            if let Some(n) = name.split('-').next() {
                if let Ok(n) = n.parse::<u32>() {
                    max_n = max_n.max(n);
                }
            }
        }
    }
    Ok(max_n + 1)
}

pub fn move_to_done(path: &Path) -> Result<()> {
    let done = done_dir();
    std::fs::create_dir_all(&done)?;
    let name = path.file_name().context("spec path has no file name")?;
    let dest = done.join(name);
    std::fs::rename(path, &dest)?;
    Ok(())
}
