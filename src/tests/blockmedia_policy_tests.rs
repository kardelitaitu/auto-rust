#[cfg(test)]
mod blockmedia_policy {
    use std::fs;
    use std::path::{Path, PathBuf};

    fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(root) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rs_files(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }

    #[test]
    fn forbid_blockmedia_usage_outside_cookiebot_task() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let task_root = base.join("task");
        let allowlist = [base.join("task/cookiebot.rs")];
        let forbidden = ["block_heavy_resources_for_cookiebot("];

        let mut offenders = Vec::new();
        let mut files = Vec::new();
        collect_rs_files(&task_root, &mut files);

        for file in files {
            if allowlist.contains(&file) {
                continue;
            }

            let Ok(content) = fs::read_to_string(&file) else {
                continue;
            };

            for needle in &forbidden {
                if content.contains(needle) {
                    offenders.push(format!(
                        "{} contains `{}`",
                        file.to_string_lossy().replace('\\', "/"),
                        needle
                    ));
                }
            }
        }

        assert!(
            offenders.is_empty(),
            "blockmedia usage is restricted to task/cookiebot.rs only.\n{}",
            offenders.join("\n")
        );
    }
}
