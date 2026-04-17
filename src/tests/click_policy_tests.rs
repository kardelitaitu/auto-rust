#[cfg(test)]
mod click_policy_tests {
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
    fn forbid_direct_page_click_usage() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let scan_roots = [base.join("src"), base.join("task")];
        let forbidden = ["page.click("];
        let allowlist = ["src/tests/click_policy_tests.rs"];

        let mut offenders = Vec::new();

        for root in scan_roots {
            let mut files = Vec::new();
            collect_rs_files(&root, &mut files);

            for file in files {
                let file_str = file.to_string_lossy().replace('\\', "/");
                if allowlist.iter().any(|allowed| file_str.ends_with(allowed)) {
                    continue;
                }

                let Ok(content) = fs::read_to_string(&file) else {
                    continue;
                };

                for needle in &forbidden {
                    if content.contains(needle) {
                        offenders.push(format!("{file_str} contains `{needle}`"));
                    }
                }
            }
        }

        assert!(
            offenders.is_empty(),
            "Direct click usage is forbidden. Use utils::mouse x/y helpers instead.\n{}",
            offenders.join("\n")
        );
    }
}
