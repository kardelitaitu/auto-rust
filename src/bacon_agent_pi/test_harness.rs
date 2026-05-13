use anyhow::Result;
use log::info;
use std::path::PathBuf;

use super::cli::TestArgs;

pub struct Fixture {
    pub name: &'static str,
    pub description: &'static str,
    pub setup: fn() -> Result<TempRepo>,
    pub expected_stages: &'static [ExpectedOutcome],
}

pub enum ExpectedOutcome {
    ObserverFindsIssue,
    StrategistWritesSpec,
    CoderImplements,
    AuditorPasses,
    AuditorRejects,
    PipelineEmpty,
}

pub struct TempRepo {
    pub dir: tempfile::TempDir,
    pub repo_path: PathBuf,
}

impl TempRepo {
    pub fn create_minimal() -> Result<Self> {
        let dir = tempfile::tempdir()?;
        let root = dir.path().to_path_buf();

        std::fs::create_dir_all(root.join("src"))?;
        std::fs::create_dir_all(root.join("docs/specs/_active"))?;
        std::fs::create_dir_all(root.join("docs/specs/_done"))?;

        std::fs::write(
            root.join("Cargo.toml"),
            r#"[package]
name = "test-fixture"
version = "0.1.0"
edition = "2021"
"#,
        )?;

        Ok(Self {
            dir,
            repo_path: root,
        })
    }

    pub fn src_file(&self, name: &str) -> PathBuf {
        self.repo_path.join("src").join(name)
    }

    pub fn active_dir(&self) -> PathBuf {
        self.repo_path.join("docs/specs/_active")
    }

    pub fn done_dir(&self) -> PathBuf {
        self.repo_path.join("docs/specs/_done")
    }

    pub fn write_src(&self, name: &str, content: &str) -> Result<()> {
        std::fs::write(self.src_file(name), content)?;
        Ok(())
    }

    pub fn init_git(&self) -> Result<()> {
        let run = |args: &[&str]| -> Result<()> {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(&self.repo_path)
                .status()?;
            if !status.success() {
                anyhow::bail!("git {:?} failed", args);
            }
            Ok(())
        };
        run(&["init"])?;
        run(&["config", "user.email", "test@test.com"])?;
        run(&["config", "user.name", "Test"])?;
        run(&["add", "."])?;
        run(&["commit", "-m", "initial"])?;
        Ok(())
    }
}

pub static FIXTURES: &[Fixture] = &[
    Fixture {
        name: "trivial-dead-code",
        description: "One unused function — fast path should handle it",
        setup: || {
            let repo = TempRepo::create_minimal()?;
            repo.write_src(
                "lib.rs",
                "pub fn used() -> i32 { 42 }\npub fn unused() -> i32 { 0 }\n",
            )?;
            repo.init_git()?;
            Ok(repo)
        },
        expected_stages: &[
            ExpectedOutcome::ObserverFindsIssue,
            ExpectedOutcome::StrategistWritesSpec,
        ],
    },
    Fixture {
        name: "clippy-lints",
        description: "Multiple clippy warnings",
        setup: || {
            let repo = TempRepo::create_minimal()?;
            repo.write_src(
                "lib.rs",
                "pub fn redundant() -> i32 { let x = 42; return x; }\n\
                 pub fn clone_me(x: &i32) -> i32 { x.clone() }\n",
            )?;
            repo.init_git()?;
            Ok(repo)
        },
        expected_stages: &[
            ExpectedOutcome::ObserverFindsIssue,
            ExpectedOutcome::StrategistWritesSpec,
        ],
    },
    Fixture {
        name: "spec-already-done",
        description: "No pending specs, nothing to do",
        setup: || {
            let repo = TempRepo::create_minimal()?;
            let done_spec = repo.done_dir().join("0001-foo");
            std::fs::create_dir_all(&done_spec)?;
            std::fs::write(
                done_spec.join("spec.yaml"),
                "id: 0001-foo\ntitle: Foo\nstatus: done\n",
            )?;
            repo.init_git()?;
            Ok(repo)
        },
        expected_stages: &[ExpectedOutcome::PipelineEmpty],
    },
];

fn list_fixtures() {
    println!("Available fixtures:");
    for f in FIXTURES {
        println!("  {:25} {}", f.name, f.description);
    }
}

pub async fn run(args: &TestArgs) -> Result<()> {
    if args.list {
        list_fixtures();
        return Ok(());
    }

    let fixtures_to_run: Vec<&Fixture> = if let Some(name) = &args.fixture {
        vec![FIXTURES
            .iter()
            .find(|f| f.name == name.as_str())
            .ok_or_else(|| anyhow::anyhow!("Fixture '{}' not found", name))?]
    } else {
        FIXTURES.iter().collect()
    };

    let mut passed = 0u32;
    let mut failed = 0u32;

    // Build bacon binary once before running fixtures
    info!("Building bacon binary for test harness...");
    let build = std::process::Command::new("cargo")
        .args(["build", "--bin", "bacon", "-q"])
        .current_dir(project_root())
        .status()?;
    if !build.success() {
        anyhow::bail!("failed to build bacon binary");
    }

    for fixture in &fixtures_to_run {
        info!(
            "Running fixture: {} ({})",
            fixture.name, fixture.description
        );
        match run_single(fixture).await {
            Ok(()) => {
                println!("  PASS  {}", fixture.name);
                passed += 1;
            }
            Err(e) => {
                println!("  FAIL  {}: {}", fixture.name, e);
                failed += 1;
            }
        }
    }

    println!("\nResults: {}/{} passed", passed, passed + failed);
    if failed > 0 {
        anyhow::bail!("{} fixture(s) failed", failed);
    }
    Ok(())
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn pi_binary() -> PathBuf {
    let target = project_root().join("target");
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let mut p = target.join(profile);
    if cfg!(windows) {
        p.push("bacon.exe");
    } else {
        p.push("bacon");
    }
    p
}

async fn run_single(fixture: &Fixture) -> Result<()> {
    let repo = (fixture.setup)()?;
    let repo_root = repo.repo_path.to_string_lossy().to_string();
    let pi_bin = pi_binary();

    if !pi_bin.exists() {
        anyhow::bail!(
            "bacon binary not found at {}. Run 'cargo build --bin bacon' first.",
            pi_bin.display()
        );
    }

    let output = std::process::Command::new(&pi_bin)
        .args(["run", "--dry-run", "--auto"])
        .current_dir(&repo_root)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run bacon: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow::anyhow!(
            "bacon exited with error:\nstdout: {}\nstderr: {}",
            stdout,
            stderr
        ));
    }

    let active_specs: Vec<_> = std::fs::read_dir(repo.active_dir())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    let done_specs = std::fs::read_dir(repo.done_dir())
        .map(|d| {
            d.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .count()
        })
        .unwrap_or(0);

    for expected in fixture.expected_stages {
        match expected {
            ExpectedOutcome::ObserverFindsIssue
                if active_specs.is_empty() && done_specs == 0 =>
            {
                anyhow::bail!("Observer should have found an issue");
            }
            ExpectedOutcome::StrategistWritesSpec
                if active_specs.is_empty() =>
            {
                anyhow::bail!("Strategist should have written a spec");
            }
            ExpectedOutcome::PipelineEmpty => {}
            _ => {}
        }
    }

    Ok(())
}
