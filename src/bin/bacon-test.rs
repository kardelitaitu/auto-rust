// bacon-test.rs - Comprehensive testing framework for Bacon autonomous coding system
// Usage: cargo run --bin bacon-test [OPTIONS]

use clap::Parser;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct TestResult {
    name: String,
    status: TestStatus,
    duration: Duration,
    output: String,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Warning,
}

#[derive(Debug)]
struct TestSuite {
    name: String,
    tests: Vec<TestResult>,
    total_duration: Duration,
}

impl TestSuite {
    fn new(name: String) -> Self {
        Self {
            name,
            tests: Vec::new(),
            total_duration: Duration::ZERO,
        }
    }

    fn add_test(&mut self, result: TestResult) {
        self.total_duration += result.duration;
        self.tests.push(result);
    }

    fn passed_count(&self) -> usize {
        self.tests
            .iter()
            .filter(|t| t.status == TestStatus::Passed)
            .count()
    }

    fn failed_count(&self) -> usize {
        self.tests
            .iter()
            .filter(|t| t.status == TestStatus::Failed)
            .count()
    }

    fn success_rate(&self) -> f64 {
        if self.tests.is_empty() {
            0.0
        } else {
            self.passed_count() as f64 / self.tests.len() as f64 * 100.0
        }
    }
}

struct BaconTestRunner {
    project_root: PathBuf,
    bacon_dir: PathBuf,
    scripts_dir: PathBuf,
}

impl BaconTestRunner {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let current_dir = env::current_dir()?;
        let project_root = current_dir;
        let bacon_dir = project_root.join(".bacon");
        let scripts_dir = bacon_dir.join("scripts");

        if !bacon_dir.exists() {
            return Err("Bacon directory not found. Please run from project root.".into());
        }

        Ok(Self {
            project_root,
            bacon_dir,
            scripts_dir,
        })
    }

    fn run_bash_test_suite(&self) -> TestSuite {
        let mut suite = TestSuite::new("Bash Test Suite".to_string());
        let test_script = self.scripts_dir.join("test-bacon-system.sh");

        println!("🔧 Running bash test suite...");

        if !test_script.exists() {
            let result = TestResult {
                name: "Bash Test Script".to_string(),
                status: TestStatus::Failed,
                duration: Duration::ZERO,
                output: String::new(),
                error: Some("test-bacon-system.sh not found".to_string()),
            };
            suite.add_test(result);
            return suite;
        }

        let start = Instant::now();

        // Use relative path from project root for better bash compatibility
        let relative_script_path = PathBuf::from(".bacon/scripts/test-bacon-system.sh");

        let output = Command::new("bash")
            .arg(&relative_script_path)
            .current_dir(&self.project_root)
            .output();

        let duration = start.elapsed();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);
                let combined_output = format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);

                let test_result = TestResult {
                    name: "Bash Test Suite".to_string(),
                    status: if result.status.success() {
                        TestStatus::Passed
                    } else {
                        TestStatus::Failed
                    },
                    duration,
                    output: combined_output.clone(),
                    error: if !result.status.success() {
                        Some(combined_output)
                    } else {
                        None
                    },
                };

                suite.add_test(test_result);

                // Parse individual test results from bash output
                self.parse_bash_test_results(&stdout, &mut suite, duration);
            }
            Err(e) => {
                let test_result = TestResult {
                    name: "Bash Test Suite".to_string(),
                    status: TestStatus::Failed,
                    duration,
                    output: String::new(),
                    error: Some(format!("Failed to execute bash test: {}", e)),
                };
                suite.add_test(test_result);
            }
        }

        suite
    }

    #[allow(clippy::collapsible_if)]
    fn parse_bash_test_results(
        &self,
        output: &str,
        suite: &mut TestSuite,
        total_duration: Duration,
    ) {
        let lines: Vec<&str> = output.lines().collect();
        let mut current_test = String::new();
        let mut test_count = 0;

        for line in &lines {
            if line.contains("[TEST] Starting test:") {
                if !current_test.is_empty() && test_count > 0 {
                    // Add previous test result
                    let status = if line.contains("PASSED") {
                        TestStatus::Passed
                    } else {
                        TestStatus::Failed
                    };
                    let result = TestResult {
                        name: current_test.clone(),
                        status,
                        duration: Duration::from_millis(100), // Approximate
                        output: String::new(),
                        error: None,
                    };
                    suite.add_test(result);
                }

                // Extract test name
                if let Some(start) = line.find("[TEST] Starting test: ") {
                    current_test = line[start + 24..].to_string();
                    test_count += 1;
                }
            } else if line.contains("[PASS]") || line.contains("[FAIL]") {
                if !current_test.is_empty() {
                    let status = if line.contains("[PASS]") {
                        TestStatus::Passed
                    } else {
                        TestStatus::Failed
                    };
                    let result = TestResult {
                        name: current_test.clone(),
                        status,
                        duration: Duration::from_millis(100), // Approximate
                        output: line.to_string(),
                        error: if line.contains("[FAIL]") {
                            Some(line.to_string())
                        } else {
                            None
                        },
                    };
                    suite.add_test(result);
                    current_test.clear();
                }
            }
        }

        // Add final test summary
        if let Some(summary_line) = lines
            .iter()
            .find(|line| line.contains("BACON SYSTEM TEST RESULTS"))
        {
            let result = TestResult {
                name: "Test Summary".to_string(),
                status: TestStatus::Passed,
                duration: total_duration,
                output: summary_line.to_string(),
                error: None,
            };
            suite.add_test(result);
        }
    }

    fn run_rust_logic_tests(&self) -> TestSuite {
        let mut suite = TestSuite::new("Rust Logic Tests".to_string());

        println!("🦀 Running Rust logic tests...");

        // Test 1: Configuration parsing
        let test_result = self.test_config_parsing();
        suite.add_test(test_result);

        // Test 2: JSON schema validation
        let test_result = self.test_json_validation();
        suite.add_test(test_result);

        // Test 3: TOML parsing
        let test_result = self.test_toml_parsing();
        suite.add_test(test_result);

        // Test 4: File system operations
        let test_result = self.test_file_operations();
        suite.add_test(test_result);

        // Test 5: Command execution
        let test_result = self.test_command_execution();
        suite.add_test(test_result);

        suite
    }

    fn test_config_parsing(&self) -> TestResult {
        let start = Instant::now();
        let config_file = self.bacon_dir.join("bacon.toml");

        if !config_file.exists() {
            return TestResult {
                name: "Configuration File Exists".to_string(),
                status: TestStatus::Failed,
                duration: start.elapsed(),
                output: String::new(),
                error: Some("bacon.toml not found".to_string()),
            };
        }

        // Try to read and parse basic configuration
        match fs::read_to_string(&config_file) {
            Ok(content) => {
                let has_cycle_interval = content.contains("cycle_interval");
                let has_log_level = content.contains("log_level");
                let has_enable_metrics = content.contains("enable_metrics");

                TestResult {
                    name: "Configuration Parsing".to_string(),
                    status: if has_cycle_interval && has_log_level && has_enable_metrics {
                        TestStatus::Passed
                    } else {
                        TestStatus::Warning
                    },
                    duration: start.elapsed(),
                    output: format!(
                        "Config file size: {} bytes, Contains cycle_interval: {}, log_level: {}, enable_metrics: {}",
                        content.len(), has_cycle_interval, has_log_level, has_enable_metrics
                    ),
                    error: None,
                }
            }
            Err(e) => TestResult {
                name: "Configuration Parsing".to_string(),
                status: TestStatus::Failed,
                duration: start.elapsed(),
                output: String::new(),
                error: Some(format!("Failed to read config: {}", e)),
            },
        }
    }

    fn test_json_validation(&self) -> TestResult {
        let start = Instant::now();
        let validator_script = self.scripts_dir.join("json-validator.sh");

        if !validator_script.exists() {
            return TestResult {
                name: "JSON Validator Exists".to_string(),
                status: TestStatus::Failed,
                duration: start.elapsed(),
                output: String::new(),
                error: Some("json-validator.sh not found".to_string()),
            };
        }

        TestResult {
            name: "JSON Validation System".to_string(),
            status: TestStatus::Passed,
            duration: start.elapsed(),
            output: "JSON validator script exists and is accessible".to_string(),
            error: None,
        }
    }

    fn test_toml_parsing(&self) -> TestResult {
        let start = Instant::now();
        let parser_script = self.scripts_dir.join("bacon-config-parser");

        let has_parser = parser_script.exists();
        let simple_parser = self.scripts_dir.join("bacon-config-parser-simple");
        let has_simple_parser = simple_parser.exists();

        TestResult {
            name: "TOML Parser Availability".to_string(),
            status: if has_parser || has_simple_parser {
                TestStatus::Passed
            } else {
                TestStatus::Failed
            },
            duration: start.elapsed(),
            output: format!(
                "Complex parser: {}, Simple parser: {}",
                has_parser, has_simple_parser
            ),
            error: None,
        }
    }

    fn test_file_operations(&self) -> TestResult {
        let start = Instant::now();

        // Test creating a temporary file in sessions directory
        let sessions_dir = self.bacon_dir.join("sessions");
        let test_file = sessions_dir.join("bacon_test_temp.txt");

        let result = match fs::write(&test_file, "test content") {
            Ok(_) => {
                let read_result = fs::read_to_string(&test_file);
                match read_result {
                    Ok(content) => {
                        let success = content == "test content";
                        // Cleanup
                        let _ = fs::remove_file(&test_file);

                        TestResult {
                            name: "File Operations".to_string(),
                            status: if success {
                                TestStatus::Passed
                            } else {
                                TestStatus::Failed
                            },
                            duration: start.elapsed(),
                            output: format!(
                                "Successfully wrote and read file content: {}",
                                success
                            ),
                            error: None,
                        }
                    }
                    Err(e) => TestResult {
                        name: "File Operations".to_string(),
                        status: TestStatus::Failed,
                        duration: start.elapsed(),
                        output: String::new(),
                        error: Some(format!("Failed to read test file: {}", e)),
                    },
                }
            }
            Err(e) => TestResult {
                name: "File Operations".to_string(),
                status: TestStatus::Failed,
                duration: start.elapsed(),
                output: String::new(),
                error: Some(format!("Failed to write test file: {}", e)),
            },
        };

        result
    }

    fn test_command_execution(&self) -> TestResult {
        let start = Instant::now();

        // Test if we can execute basic commands
        let result = match Command::new("cargo").arg("--version").output() {
            Ok(output) => {
                let version = String::from_utf8_lossy(&output.stdout);
                TestResult {
                    name: "Command Execution".to_string(),
                    status: TestStatus::Passed,
                    duration: start.elapsed(),
                    output: format!("Cargo version: {}", version.trim()),
                    error: None,
                }
            }
            Err(e) => TestResult {
                name: "Command Execution".to_string(),
                status: TestStatus::Failed,
                duration: start.elapsed(),
                output: String::new(),
                error: Some(format!("Failed to execute cargo: {}", e)),
            },
        };

        result
    }

    fn run_integration_tests(&self) -> TestSuite {
        let mut suite = TestSuite::new("Integration Tests".to_string());

        println!("🔗 Running integration tests...");

        // Test 1: PowerShell manager availability
        let test_result = self.test_powershell_manager();
        suite.add_test(test_result);

        // Test 2: Bacon system prerequisites
        let test_result = self.test_system_prerequisites();
        suite.add_test(test_result);

        // Test 3: Script availability
        let test_result = self.test_script_availability();
        suite.add_test(test_result);

        suite
    }

    fn test_powershell_manager(&self) -> TestResult {
        let start = Instant::now();
        let manager_script = self.scripts_dir.join("bacon-manager.ps1");

        if !manager_script.exists() {
            return TestResult {
                name: "PowerShell Manager Exists".to_string(),
                status: TestStatus::Failed,
                duration: start.elapsed(),
                output: String::new(),
                error: Some("bacon-manager.ps1 not found".to_string()),
            };
        }

        TestResult {
            name: "PowerShell Manager".to_string(),
            status: TestStatus::Passed,
            duration: start.elapsed(),
            output: "PowerShell manager script exists and is accessible".to_string(),
            error: None,
        }
    }

    fn test_system_prerequisites(&self) -> TestResult {
        let start = Instant::now();

        // Check for essential commands
        let commands = vec!["cargo", "rustc"];
        let mut available_commands = 0;
        let mut command_results = Vec::new();

        for cmd in &commands {
            match Command::new(cmd).arg("--version").output() {
                Ok(output) => {
                    if output.status.success() {
                        available_commands += 1;
                        let version = String::from_utf8_lossy(&output.stdout);
                        command_results.push(format!("{}: {}", cmd, version.trim()));
                    } else {
                        command_results.push(format!("{}: Command failed", cmd));
                    }
                }
                Err(_) => {
                    command_results.push(format!("{}: Not found", cmd));
                }
            }
        }

        TestResult {
            name: "System Prerequisites".to_string(),
            status: if available_commands == commands.len() {
                TestStatus::Passed
            } else {
                TestStatus::Warning
            },
            duration: start.elapsed(),
            output: format!(
                "Available commands: {}/{}\n{}",
                available_commands,
                commands.len(),
                command_results.join("\n")
            ),
            error: None,
        }
    }

    fn test_script_availability(&self) -> TestResult {
        let start = Instant::now();

        let required_scripts = vec![
            "bacon-orchestrate.sh",
            "bacon-observer.sh",
            "bacon-strategist.sh",
            "bacon-sentinel.sh",
            "bacon-config.sh",
            "test-bacon-system.sh",
            "json-validator.sh",
        ];

        let mut available_scripts = 0;
        let mut script_results = Vec::new();

        for script in &required_scripts {
            let script_path = self.scripts_dir.join(script);
            let exists = script_path.exists();
            if exists {
                available_scripts += 1;
            }
            script_results.push(format!("{}: {}", script, if exists { "✓" } else { "✗" }));
        }

        TestResult {
            name: "Script Availability".to_string(),
            status: if available_scripts == required_scripts.len() {
                TestStatus::Passed
            } else {
                TestStatus::Warning
            },
            duration: start.elapsed(),
            output: format!(
                "Available scripts: {}/{}\n{}",
                available_scripts,
                required_scripts.len(),
                script_results.join("\n")
            ),
            error: None,
        }
    }

    fn print_suite_results(&self, suite: &TestSuite) {
        println!("\n📊 {} Results:", suite.name);
        println!("   Total Tests: {}", suite.tests.len());
        println!(
            "   Passed: {} ({})",
            suite.passed_count(),
            if suite.passed_count() == suite.tests.len() {
                "✓"
            } else {
                "✗"
            }
        );
        println!("   Failed: {}", suite.failed_count());
        println!("   Success Rate: {:.1}%", suite.success_rate());
        println!("   Duration: {:?}", suite.total_duration);

        for test in &suite.tests {
            let status_icon = match test.status {
                TestStatus::Passed => "✅",
                TestStatus::Failed => "❌",
                TestStatus::Warning => "⚠️",
                TestStatus::Skipped => "⏭️",
            };

            println!("   {} {} ({:?})", status_icon, test.name, test.duration);

            if let Some(error) = &test.error {
                println!("     Error: {}", error);
            }

            if !test.output.is_empty() && test.status == TestStatus::Failed {
                // Show first line of output for failed tests
                if let Some(first_line) = test.output.lines().next() {
                    println!("     Output: {}", first_line);
                }
            }
        }
    }

    #[allow(clippy::vec_init_then_push)]
    fn run_all_tests(&self) -> Vec<TestSuite> {
        let mut suites = Vec::new();
        suites.push(self.run_bash_test_suite());
        suites.push(self.run_rust_logic_tests());
        suites.push(self.run_integration_tests());
        suites
    }
}

#[derive(Parser)]
#[command(name = "bacon-test")]
#[command(version = "1.0.0")]
#[command(about = "Comprehensive testing framework for Bacon autonomous coding system")]
struct Cli {
    /// Run only bash test suite
    #[arg(long)]
    bash_only: bool,

    /// Run only Rust logic tests
    #[arg(long)]
    rust_only: bool,

    /// Run only integration tests
    #[arg(long)]
    integration_only: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let runner = BaconTestRunner::new()?;

    println!("🥓 Bacon Test Framework v1.0.0");
    println!("📍 Project: {}", runner.project_root.display());
    println!("🔧 Starting comprehensive test suite...\n");

    let mut suites = Vec::new();
    let total_start = Instant::now();

    if cli.bash_only {
        suites.push(runner.run_bash_test_suite());
    } else if cli.rust_only {
        suites.push(runner.run_rust_logic_tests());
    } else if cli.integration_only {
        suites.push(runner.run_integration_tests());
    } else {
        suites = runner.run_all_tests();
    }

    let total_duration = total_start.elapsed();

    // Print results for each suite
    for suite in &suites {
        runner.print_suite_results(suite);
    }

    // Print overall summary
    let total_tests: usize = suites.iter().map(|s| s.tests.len()).sum();
    let total_passed: usize = suites.iter().map(|s| s.passed_count()).sum();
    let total_failed: usize = suites.iter().map(|s| s.failed_count()).sum();
    let overall_success_rate = if total_tests > 0 {
        total_passed as f64 / total_tests as f64 * 100.0
    } else {
        0.0
    };

    println!("\n🎯 Overall Test Summary:");
    println!("   Total Tests: {}", total_tests);
    println!(
        "   Passed: {} ({})",
        total_passed,
        if total_passed == total_tests {
            "✓"
        } else {
            "✗"
        }
    );
    println!("   Failed: {}", total_failed);
    println!("   Success Rate: {:.1}%", overall_success_rate);
    println!("   Total Duration: {:?}", total_duration);

    // Exit with appropriate code
    if total_failed > 0 {
        println!("\n❌ Some tests failed. Check the output above for details.");
        std::process::exit(1);
    } else {
        println!("\n✅ All tests passed! Bacon system is ready for production.");
        std::process::exit(0);
    }
}
