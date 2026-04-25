#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    #[test]
    fn page_manager_is_not_reexported_from_public_surface() {
        let lib = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
            .expect("read lib.rs");
        let runtime =
            fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime/mod.rs"))
                .expect("read runtime/mod.rs");

        assert!(!lib.contains("pub mod page_manager;"));
        assert!(!runtime.contains("pub mod page_manager"));
    }

    #[test]
    fn page_manager_is_not_used_outside_its_own_file() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for path in [
            root.join("src/lib.rs"),
            root.join("src/runtime/mod.rs"),
            root.join("src/main.rs"),
            root.join("src/orchestrator.rs"),
            root.join("src/task/mod.rs"),
        ] {
            if path.exists() {
                let content = fs::read_to_string(&path).expect("read source");
                assert!(
                    !content.contains("PageManager"),
                    "{} still references PageManager",
                    path.display()
                );
            }
        }
    }
}
