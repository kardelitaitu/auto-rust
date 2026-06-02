use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

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

fn spawn_fake_ollama(max_requests: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake ollama");
    let addr = listener.local_addr().expect("fake ollama addr");

    thread::spawn(move || {
        for stream in listener.incoming().take(max_requests) {
            let Ok(mut stream) = stream else {
                break;
            };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
            let _ = read_http_request(&mut stream);

            // NVIDIA Chat Completions API response format
            let body = r##"{"choices":[{"message":{"content":"# Fixture Plan\n\n1. Keep this dry-run fixture deterministic.\n2. Do not write files.\n3. Report success."},"finish_reason":"stop"}]}"##;
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

fn read_http_request(stream: &mut impl Read) -> std::io::Result<Vec<u8>> {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut content_length = None;

    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Ok(request);
        }
        request.extend_from_slice(&buffer[..read]);

        if content_length.is_none() {
            if let Some(header_end) = find_header_end(&request) {
                let headers = String::from_utf8_lossy(&request[..header_end]);
                content_length = headers.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                });

                if content_length.is_none() {
                    return Ok(request);
                }
            }
        }

        if let (Some(header_end), Some(length)) = (find_header_end(&request), content_length) {
            if request.len() >= header_end + 4 + length {
                return Ok(request);
            }
        }
    }
}

fn find_header_end(request: &[u8]) -> Option<usize> {
    request.windows(4).position(|window| window == b"\r\n\r\n")
}

#[test]
fn bacon_full_dry_run_smoke_uses_configured_local_ollama() {
    let bacon = bin_path("bacon");
    let codex = bin_path("codex");
    let worker_dir = codex.parent().expect("codex binary parent");
    let ollama_url = spawn_fake_ollama(4);
    let dir = tempfile::tempdir().expect("temp config dir");
    let bacon_config = dir.path().join("bacon.toml");
    let config = format!(
        r#"
[pipeline]
observer = "nvidia_observer"
strategist = "nvidia_strategist"
coder = "nvidia_coder"
auditor = "nvidia_auditor"
stage_delay_ms = 0
enable_auto_apply = false

[agents.nvidia_observer]
provider = "ollama"
model = "fixture-model"
base_url = "{ollama_url}"
temperature = 0.0
max_tokens = 256
timeout_ms = 5000

[agents.nvidia_strategist]
provider = "ollama"
model = "fixture-model"
base_url = "{ollama_url}"
temperature = 0.0
max_tokens = 256
timeout_ms = 5000

[agents.nvidia_coder]
provider = "ollama"
model = "fixture-model"
base_url = "{ollama_url}"
temperature = 0.0
max_tokens = 256
timeout_ms = 5000

[agents.nvidia_auditor]
provider = "ollama"
model = "fixture-model"
base_url = "{ollama_url}"
temperature = 0.0
max_tokens = 256
timeout_ms = 5000
"#
    );
    std::fs::write(&bacon_config, config).expect("write temp bacon config");

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
        .env("BACON_CONFIG", bacon_config)
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
        combined.contains("Stage 1: Observer (agent: nvidia_observer)"),
        "observer did not use configured worker:\n{}",
        combined
    );
    assert!(
        combined.contains("Agent config: provider=ollama"),
        "pipeline did not use local test LLM config:\n{}",
        combined
    );
}
