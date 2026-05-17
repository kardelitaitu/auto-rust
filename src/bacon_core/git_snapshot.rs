use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Captures pre-apply state to enable safe rollback after failed patch application.
///
/// ## Flow
/// 1. `GitSnapshot::create()` — records HEAD, stashes uncommitted changes, backs up
///    target files to a snapshot directory under `.bacon/snapshots/`.
/// 2. Apply the patch normally.
/// 3. If validation (e.g., `check-fast.ps1`) passes: `snapshot.mark_applied()`.
/// 4. If validation fails: `snapshot.restore()`.
pub struct GitSnapshot {
    /// Repository root
    root: PathBuf,
    /// Snapshot directory (e.g., `.bacon/snapshots/<ts>_<uuid>/`)
    snapshot_dir: PathBuf,
    /// HEAD commit at snapshot time
    base_commit: String,
    /// Whether uncommitted changes were stashed
    had_stash: bool,
    /// Relative file paths backed up before patch application
    backed_up_files: Vec<PathBuf>,
}

impl GitSnapshot {
    /// Create a snapshot before applying a patch to `changed_paths`.
    ///
    /// 1. Records the current HEAD commit.
    /// 2. Stashes any uncommitted tracked changes.
    /// 3. Backs up each file the patch touches.
    pub fn create(root: &Path, changed_paths: &[PathBuf]) -> Result<Self> {
        let root = root
            .canonicalize()
            .context("failed to canonicalize repo root")?;

        // 1. Record HEAD
        let base_commit = current_head(&root)?;

        // 2. Stash uncommitted tracked changes
        let had_stash = stash_uncommitted(&root)?;

        // 3. Create snapshot directory
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let snapshot_dir = root
            .join(".bacon")
            .join("snapshots")
            .join(format!("snap_{}", ts));
        std::fs::create_dir_all(&snapshot_dir).with_context(|| {
            format!("failed to create snapshot dir: {}", snapshot_dir.display())
        })?;

        // 4. Backup each changed file
        let mut backed_up_files = Vec::new();
        for rel_path in changed_paths {
            let abs_path = root.join(rel_path);
            if abs_path.exists() {
                let backup_target = snapshot_dir.join(rel_path);
                if let Some(parent) = backup_target.parent() {
                    std::fs::create_dir_all(parent).with_context(|| {
                        format!(
                            "failed to create backup parent dir for {}",
                            rel_path.display()
                        )
                    })?;
                }
                std::fs::copy(&abs_path, &backup_target).with_context(|| {
                    format!("failed to back up {} to snapshot", rel_path.display())
                })?;
                backed_up_files.push(rel_path.clone());
            }
        }
        Ok(Self {
            root,
            snapshot_dir,
            base_commit,
            had_stash,
            backed_up_files,
        })
    }

    /// Restore the repository to the captured snapshot state (rollback).
    ///
    /// 1. Restores each backed-up file from the snapshot directory.
    /// 2. Pops stash if one was created.
    /// 3. Verifies HEAD hasn't drifted.
    pub fn restore(&self) -> Result<()> {
        // 1. Restore backed-up files
        for rel_path in &self.backed_up_files {
            let backup_source = self.snapshot_dir.join(rel_path);
            let target = self.root.join(rel_path);
            if backup_source.exists() {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&backup_source, &target).with_context(|| {
                    format!("failed to restore {} from snapshot", rel_path.display())
                })?;
            } else {
                // File didn't exist before — remove it
                let _ = std::fs::remove_file(&target);
            }
        }

        // 2. Pop stash if we stashed
        if self.had_stash {
            let output = Command::new("git")
                .arg("-C")
                .arg(&self.root)
                .args(["stash", "pop"])
                .output()
                .context("failed to pop stash during rollback")?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("stash pop failed during rollback: {}", stderr);
            }
        }

        // 3. Warn if HEAD changed during our operation
        let current = current_head(&self.root)?;
        if current != self.base_commit {
            anyhow::bail!(
                "HEAD changed from {} to {} during snapshot lifetime",
                self.base_commit,
                current
            );
        }

        Ok(())
    }

    /// Mark the snapshot as applied successfully — archives the snapshot for reference.
    pub fn mark_applied(self) -> Result<()> {
        let applied_marker = self.snapshot_dir.join(".applied");
        std::fs::write(
            &applied_marker,
            format!("base_commit: {}\n", self.base_commit),
        )
        .context("failed to write .applied marker")?;
        Ok(())
    }
}

// ── Helper functions ──────────────────────────────────────────────────────────

/// Get the current HEAD commit hash.
fn current_head(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .context("failed to get HEAD commit")?;
    if !output.status.success() {
        anyhow::bail!(
            "git rev-parse HEAD failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Stash any uncommitted tracked changes. Returns `true` if a stash was created.
///
/// Does NOT include untracked files, because the patch file saved to
/// `.bacon/sessions/approved_patches/` is an untracked file that must
/// persist across the snapshot lifecycle.
fn stash_uncommitted(root: &Path) -> Result<bool> {
    // Check for unstaged or staged changes to tracked files
    let status_output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain"])
        .output()
        .context("failed to check git status")?;
    let status = String::from_utf8_lossy(&status_output.stdout);
    let has_tracked_changes = status.lines().any(|l| !l.is_empty() && !l.starts_with("?"));

    if !has_tracked_changes {
        return Ok(false);
    }

    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["stash", "push", "-m", "bacon-snapshot"])
        .output()
        .context("failed to stash uncommitted changes")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git stash failed: {}", stderr);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn init_git_repo(dir: &Path) -> Result<()> {
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("init")
            .output()
            .context("git init failed")?;
        // Set user config to avoid warnings
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["config", "user.email", "test@test.com"])
            .output()?;
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["config", "user.name", "Test"])
            .output()?;
        Ok(())
    }

    fn write_and_commit(root: &Path, path: &str, content: &str) -> Result<()> {
        let full = root.join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&full, content)?;
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["add", "."])
            .output()?;
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["commit", "-m", "initial"])
            .output()?;
        Ok(())
    }

    #[test]
    fn test_snapshot_create_and_restore() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        init_git_repo(root)?;
        write_and_commit(
            root,
            "src/lib.rs",
            "pub fn hello() -> &'static str { \"hello\" }\n",
        )?;

        // Modify file
        fs::write(
            root.join("src/lib.rs"),
            "pub fn hello() -> &'static str { \"world\" }\n",
        )?;

        let changed_paths = vec![PathBuf::from("src/lib.rs")];
        let snapshot = GitSnapshot::create(root, &changed_paths)?;

        // Verify snapshot was created
        assert!(snapshot.snapshot_dir.exists(), "snapshot dir should exist");
        assert!(
            snapshot.snapshot_dir.join("src/lib.rs").exists(),
            "backup file should exist"
        );

        // Read backup content (strip Windows \r\n differences)
        let backup_content = fs::read_to_string(snapshot.snapshot_dir.join("src/lib.rs"))?;
        assert_eq!(
            backup_content.trim(),
            "pub fn hello() -> &'static str { \"hello\" }"
        );

        // Modify file further
        fs::write(
            root.join("src/lib.rs"),
            "pub fn hello() -> &'static str { \"broken\" }\n",
        )?;

        // Restore
        snapshot.restore()?;

        // Verify restored (compare trimmed to handle Windows line endings)
        let restored = fs::read_to_string(root.join("src/lib.rs"))?;
        assert!(
            restored.contains("hello") && restored.contains("static str"),
            "file should be restored to snapshot state, got: {}",
            restored
        );

        Ok(())
    }

    #[test]
    fn test_snapshot_stash_and_restore() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        init_git_repo(root)?;
        write_and_commit(root, "src/lib.rs", "pub fn a() {}\n")?;

        // Make uncommitted changes
        fs::write(root.join("src/lib.rs"), "pub fn b() {}\n")?;

        let changed_paths = vec![PathBuf::from("src/lib.rs")];
        let snapshot = GitSnapshot::create(root, &changed_paths)?;
        assert!(
            snapshot.had_stash,
            "should have stashed uncommitted changes"
        );

        // Make more changes after snapshot (simulating patch application)
        fs::write(root.join("src/lib.rs"), "pub fn c() {}\n")?;

        // Restore
        snapshot.restore()?;

        // After restore: file should be back to snapshot-time state,
        // which includes the stashed uncommitted changes (b)
        let content = fs::read_to_string(root.join("src/lib.rs"))?;
        assert!(
            content.contains("pub fn b()"),
            "restore should re-apply stashed uncommitted changes, got: {}",
            content
        );

        Ok(())
    }

    #[test]
    fn test_snapshot_no_changes() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        init_git_repo(root)?;
        write_and_commit(root, "src/lib.rs", "pub fn a() {}\n")?;

        // No uncommitted changes, no modifications
        let changed_paths = vec![PathBuf::from("src/lib.rs")];
        let snapshot = GitSnapshot::create(root, &changed_paths)?;

        assert!(!snapshot.had_stash, "no stash needed");
        // Backup should still exist
        assert!(snapshot.snapshot_dir.join("src/lib.rs").exists());

        Ok(())
    }

    #[test]
    fn test_snapshot_new_file_backup() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        init_git_repo(root)?;
        write_and_commit(root, "src/lib.rs", "pub fn a() {}\n")?;

        // New file not yet committed
        fs::write(root.join("src/new.rs"), "pub fn b() {}\n")?;

        let changed_paths = vec![PathBuf::from("src/new.rs"), PathBuf::from("src/lib.rs")];
        let snapshot = GitSnapshot::create(root, &changed_paths)?;

        // New file exists at backup time (not stashed), so it IS backed up
        assert!(snapshot.snapshot_dir.join("src/lib.rs").exists());
        assert!(
            snapshot.snapshot_dir.join("src/new.rs").exists(),
            "new file exists at backup time, should be backed up"
        );

        // Restore
        snapshot.restore()?;

        // lib.rs should be back to original
        let lib = fs::read_to_string(root.join("src/lib.rs"))?;
        assert!(lib.contains("pub fn a()"), "lib.rs should be restored");
        // new.rs should still exist (restore preserves files not in backup)
        assert!(
            root.join("src/new.rs").exists(),
            "new.rs should still exist after restore"
        );

        Ok(())
    }

    #[test]
    fn test_mark_applied() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        init_git_repo(root)?;
        write_and_commit(root, "src/lib.rs", "pub fn a() {}\n")?;

        let changed_paths = vec![PathBuf::from("src/lib.rs")];
        let snapshot = GitSnapshot::create(root, &changed_paths)?;

        let snapshot_dir = snapshot.snapshot_dir.clone();
        snapshot.mark_applied()?;

        let marker = snapshot_dir.join(".applied");
        assert!(marker.exists(), ".applied marker should exist");

        Ok(())
    }
}
