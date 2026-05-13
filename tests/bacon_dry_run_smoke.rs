use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::thread;

fn bin_path(name: &str) -> PathBuf {
    let var_name = format!("CARGO_BIN_EXE_{}", name);
    if let Some(path) = std::env::var_os(&var_name) {
        return PathBuf::from(path);
    }

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push(if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    });
    path
}

fn spawn_fake_ollama() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake ollama");
    let addr = listener.local_addr().expect("fake ollama addr");

    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buffer = [0_u8; 8192];
            let _ = stream.read(&mut buffer);

            let body = r##"{"message":{"role":"assistant","content":"# Fixture Plan\n\n1. Keep this dry-run fixture deterministic.\n2. Do not write files.\n3. Report success."},"done":true}"##;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    format!("http://{}", addr)
}

#[test]
fn bacon_full_dry_run_smoke_uses_cli_observer_and_local_ollama() {
    let bacon = bin_path("bacon");
    let codex = bin_path("codex");
    let worker_dir = codex.parent().expect("codex binary parent");
    let ollama_url = spawn_fake_ollama();

    let path_sep = if cfg!(windows) { ";" } else { ":" };
    let path = format!(
        "{}{}{}",
        worker_dir.display(),
        path_sep,
        std::env::var("PATH").unwrap_or_default()
    );

    let output = Command::new(bacon)
        .args([
            "--dry-run",
            "--auto",
            "-p",
            "dry-run smoke test: scan for one small improvement",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("PATH", path)
        .env("LLM_PROVIDER", "ollama")
        .env("OLLAMA_URL", ollama_url)
        .env("OLLAMA_MODEL", "fixture-model")
        .env("RUST_LOG", "info")
        .output()
        .expect("run bacon dry-run smoke");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    assert!(
        output.status.success(),
        "bacon dry-run failed\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
    assert!(
        combined.contains("Stage 1: Observer (agent: nvidia)"),
        "observer did not use nvidia worker:\n{}",
        combined
    );
    assert!(
        combined.contains("Pipeline complete"),
        "pipeline did not complete:\n{}",
        combined
    );
}
