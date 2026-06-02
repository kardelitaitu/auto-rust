# Extract bacon-pipeline as a Portable Standalone Crate

## Context

The bacon pipeline (Observer → Strategist → Coder → Auditor) is a gated LLM automation pipeline currently embedded in the `auto-rust` browser-automation monorepo. The goal is to extract it into a **standalone crate** (`bacon-pipeline`) that can be copy-pasted into any Rust project as a path or git dependency. The user wants **NVIDIA-only** LLM support and **cross-platform validation** (no PowerShell dependency).

After extraction, the host project (`auto-rust`) continues working via thin re-exports.

---

## Task 1: Convert to Cargo workspace + scaffold bacon-pipeline crate

Convert `auto-rust` into a Cargo workspace and create the new crate skeleton.

**Files to modify:**
- `Cargo.toml` — Add `[workspace]` with members `[".", "bacon-pipeline"]`

**Files to create:**
```
bacon-pipeline/
  Cargo.toml
  src/
    lib.rs            (pub mod core; pub mod agent; pub mod llm; pub mod config;)
    config.rs         (ProjectConfig struct — see Task 4)
```

**bacon-pipeline/Cargo.toml:**
```toml
[package]
name = "bacon-pipeline"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
log = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yml = "0.0.12"
toml = "0.8"
regex = "1"
clap = { version = "4", features = ["derive"] }
async-trait = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "time", "sync", "fs", "macros"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }

[dev-dependencies]
tempfile = "3"
```

**Verification:** `cargo check -p bacon-pipeline` compiles (empty crate).

---

## Task 2: Copy bacon_core modules into bacon-pipeline

Copy the 5 `bacon_core` files verbatim, update internal paths.

**Files to create:**
```
bacon-pipeline/src/
  core/
    mod.rs          ← from src/bacon_core/mod.rs (1957 lines)
    agent.rs        ← from src/bacon_core/agent.rs
    git_snapshot.rs ← from src/bacon_core/git_snapshot.rs
    spec_io.rs      ← from src/bacon_core/spec_io.rs
    cli_types.rs    ← from src/bacon_core/cli_types.rs
```

**Changes after copy:**
- `manifest_dir()` (line 1111 of mod.rs): Replace `env!("CARGO_MANIFEST_DIR")` with `ProjectConfig::project_root()` (a `OnceLock`-based global, see Task 4). All 6+ downstream callers (`bacon_config_path`, `collect_source_context`, `read_role_prompt`, `gather_project_context`, etc.) inherit this automatically since they all go through `manifest_dir()`.
- `spec_io::specs_root()`: Same change — use `ProjectConfig` instead of `env!("CARGO_MANIFEST_DIR")`.
- Remove `collect_source_context`'s direct `env!("CARGO_MANIFEST_DIR")` on line 1130 — delegate to `manifest_dir()`.

**Verification:** `cargo check -p bacon-pipeline` compiles. `cargo test -p bacon-pipeline` passes unit tests (non-filesystem tests).

---

## Task 3: Create NVIDIA-only LLM client

Extract only the NVIDIA parts of `src/llm/` into the new crate. Remove Ollama/OpenRouter entirely.

**Files to create:**
```
bacon-pipeline/src/
  llm/
    mod.rs          (Llm struct wrapping NvidiaConfig + reqwest::Client)
    client.rs       (nvidia_chat + retry logic extracted from llm/client.rs, ~200 lines)
    models.rs       (ChatMessage, ChatRequest, ChatResponse, NvidiaConfig — no Ollama/OpenRouter types)
```

**Key simplifications:**
- `Llm` struct holds `NvidiaConfig` directly (no `LlmConfig` wrapper, no `LlmProvider` enum)
- `Llm::from_env_and_config()` reads `NVIDIA_API_KEY`, `NVIDIA_BASE_URL`, `NVIDIA_MODEL` from env + `.bacon/bacon.toml` per-agent overrides
- `pipeline.rs::llm_for_agent()` becomes ~20 lines (no match on provider, just apply overrides to NvidiaConfig)
- `validate_bacon_local_only()` simplified — remove Ollama/OpenRouter provider checks

**Verification:** `cargo check -p bacon-pipeline` compiles. Unit tests for ChatMessage constructors and retry logic pass.

---

## Task 4: Implement ProjectConfig (path abstraction)

Replace all `env!("CARGO_MANIFEST_DIR")` with a configurable `ProjectConfig`.

**File:** `bacon-pipeline/src/config.rs`

```rust
use std::path::PathBuf;
use std::sync::OnceLock;

static PROJECT_CONFIG: OnceLock<ProjectConfig> = OnceLock::new();

pub fn init(config: ProjectConfig) {
    PROJECT_CONFIG.set(config).expect("already initialized");
}

pub fn project_config() -> &'static ProjectConfig {
    PROJECT_CONFIG.get().expect("call bacon_pipeline::init() first")
}

#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub project_root: PathBuf,        // replaces CARGO_MANIFEST_DIR
    pub specs_dir: PathBuf,           // default: root/docs/specs
    pub bacon_dir: PathBuf,           // default: root/.bacon
    pub roles_dir: PathBuf,           // default: root/.bacon/roles
    pub env_file: PathBuf,            // default: root/.env
    pub validation: ValidationCommands,
}

#[derive(Debug, Clone)]
pub struct ValidationCommands {
    pub check_fast: Vec<String>,      // default: ["cargo", "check", "--lib", "--bins"]
    pub check_full: Vec<String>,      // default: ["cargo", "test"]
    pub spec_lint: Vec<String>,       // default: [] (use built-in Rust linter)
}

impl ProjectConfig {
    pub fn with_defaults(root: PathBuf) -> Self { /* fill conventional paths */ }
    pub fn project_root() -> PathBuf { project_config().project_root.clone() }
}
```

**Files to modify in core/ and agent/:**
- Every `env!("CARGO_MANIFEST_DIR")` → `crate::config::project_config().project_root` or `ProjectConfig::project_root()`
- `spec_io::specs_root()` → `project_config().specs_dir`
- `read_role_prompt()` → use `project_config().roles_dir`
- `run_powershell_with_args()` → generalized to `run_validation_command()` using `ValidationCommands`

**Verification:** Set different project roots and verify paths resolve correctly.

---

## Task 5: Copy agent modules into bacon-pipeline

Copy `bacon_agent_nvidia` files, rewire imports, simplify pipeline.

**Files to create:**
```
bacon-pipeline/src/
  agent/
    mod.rs          ← from src/bacon_agent_nvidia/mod.rs
    pipeline.rs     ← simplified NVIDIA-only (remove 90 lines of provider dispatch)
    coder.rs        ← from src/bacon_agent_nvidia/coder.rs
    observer.rs     ← from src/bacon_agent_nvidia/observer.rs
    strategist.rs   ← from src/bacon_agent_nvidia/strategist.rs
    auditor.rs      ← from src/bacon_agent_nvidia/auditor.rs
    committer.rs    ← from src/bacon_agent_nvidia/committer.rs
    cli.rs          ← from src/bacon_agent_nvidia/cli.rs
    spec_io.rs      ← from src/bacon_agent_nvidia/spec_io.rs
    types.rs        ← from src/bacon_agent_nvidia/types.rs
    nvidia_api.rs   ← from src/bacon_agent_nvidia/nvidia_api.rs (optional, for standalone use)
```

**Import rewiring (all agent files):**

| Old import | New import |
|---|---|
| `crate::bacon_core::*` | `crate::core::*` |
| `crate::llm::Llm` | `crate::llm::Llm` (same path, new module) |
| `crate::llm::ChatMessage` | `crate::llm::ChatMessage` |
| `crate::llm::LlmProvider` | **removed** |
| `crate::llm::create_llm_client_from_config` | `crate::llm::Llm::from_env_and_config` |

**Validation script changes:**
- `coder.rs::run_check_fast()` — read command from `project_config().validation.check_fast` instead of hardcoded PowerShell
- `strategist.rs` / `auditor.rs` — read spec_lint command from config, fallback to built-in
- `committer.rs` — read check_full command from config

**Verification:** `cargo check -p bacon-pipeline` compiles clean. `cargo test -p bacon-pipeline` passes.

---

## Task 6: Bundle cross-platform validation scripts

Provide bash equivalents alongside PowerShell scripts, and implement spec-lint in Rust.

**Files to create:**
```
bacon-pipeline/
  scripts/
    check-fast.sh     (bash equivalent of check-fast.ps1)
    check.sh          (bash equivalent of check.ps1)
    spec-lint.sh      (bash equivalent of spec-lint.ps1)
```

**Built-in spec-lint:** Implement the spec-lint validation rules as a Rust function `run_builtin_spec_lint()` in the agent module. This eliminates the external script dependency for the most common use case. Rules to port:
- Check file existence (spec.yaml, plan.md, validation.md)
- Validate YAML structure
- Check status transitions
- Detect failure reports in validation.md
- Detect generic acceptance criteria

**Verification:** Run `run_builtin_spec_lint()` against existing specs and compare output with `spec-lint.ps1`.

---

## Task 7: Host project integration (auto-rust uses bacon-pipeline)

Wire auto-rust to use the extracted crate while maintaining backward compatibility.

**Files to modify:**

**`Cargo.toml`:**
```toml
[workspace]
members = [".", "bacon-pipeline"]

[dependencies]
bacon-pipeline = { path = "bacon-pipeline" }
```

**`src/lib.rs`:**
```rust
// Replace:
// pub mod bacon_core;
// pub mod bacon_agent_nvidia;
// With:
pub mod bacon_core {
    pub use bacon_pipeline::core::*;
}
pub mod bacon_agent_nvidia {
    pub use bacon_pipeline::agent::*;
}
```

**`src/bin/bacon.rs`:**
```rust
use bacon_pipeline::agent::pipeline::Pipeline;
use bacon_pipeline::core::cli_types::{Cli, Command, RunArgs};
use bacon_pipeline::config::{init, ProjectConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    init(ProjectConfig::with_defaults(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    ));
    // ... existing CLI logic
}
```

**Other binaries** (`bacon-spec.rs`, `bacon-coder.rs`, `bacon-review.rs`, `nvidia.rs`): Add `bacon_pipeline::config::init()` call at startup, update imports.

**Keep `src/llm/`** in auto-rust — it's still used by Twitter/task modules. The bacon-pipeline crate has its own copy.

**Verification:**
- `cargo build` — compiles (workspace + both crates)
- `cargo test -p auto-rust` — no regressions
- `cargo run --bin bacon -- --dry-run` — pipeline initializes correctly
- `cargo run --bin nvidia` — standalone binary still works

---

## Task 8: Final verification and cleanup

**Checks:**
1. `cargo check -p bacon-pipeline` — clean
2. `cargo test -p bacon-pipeline` — all tests pass
3. `cargo build -p auto-rust` — compiles with re-exports
4. `cargo test -p auto-rust` — no regressions
5. `cargo run --bin bacon -- --dry-run` — pipeline works from new crate
6. Grep bacon-pipeline/ for `auto-rust`, `crate::adaptive`, `crate::browser`, `crate::task` — zero matches
7. Verify no `env!("CARGO_MANIFEST_DIR")` in bacon-pipeline that resolves to auto-rust paths
8. Remove old `src/bacon_core/` and `src/bacon_agent_nvidia/` from auto-rust (replaced by re-exports)

**Optional: Portability smoke test**
Create a minimal test project that depends on `bacon-pipeline` with its own `.bacon/bacon.toml` and run `--dry-run`.
