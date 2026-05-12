// bacon-config-parser-simple.rs - Simple TOML parser for bacon configuration
// Compile with: rustc --edition 2021 bacon-config-parser-simple.rs -o bacon-config-parser-simple

use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <toml_file>", args[0]);
        process::exit(1);
    }
    
    let config_file = &args[1];
    
    // Read TOML file
    let toml_content = match fs::read_to_string(config_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading {}: {}", config_file, e);
            process::exit(1);
        }
    };
    
    // Parse and export configuration as environment variables
    let mut cycle_interval = 10;
    let mut max_cycles = 0;
    let mut log_level = "info";
    let mut enable_metrics = true;
    let mut shadow_cleanup = true;
    let mut gemini_model = "gemini-pro";
    let mut codex_model = "codex-5.5";
    let mut audit_model = "codex-5.4mini";
    let mut request_timeout = 30;
    let mut max_shadow_age = 24;
    let mut rollback_depth = 10;
    let mut enable_rollback = true;
    
    // Simple TOML parsing
    for line in toml_content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            
            match key {
                "cycle_interval" => {
                    if let Ok(val) = value.parse::<u32>() {
                        cycle_interval = val;
                    }
                }
                "max_cycles" => {
                    if let Ok(val) = value.parse::<u32>() {
                        max_cycles = val;
                    }
                }
                "log_level" => log_level = value,
                "enable_metrics" => {
                    if let Ok(val) = value.parse::<bool>() {
                        enable_metrics = val;
                    }
                }
                "shadow_cleanup" => {
                    if let Ok(val) = value.parse::<bool>() {
                        shadow_cleanup = val;
                    }
                }
                "gemini_model" => gemini_model = value,
                "codex_model" => codex_model = value,
                "audit_model" => audit_model = value,
                "request_timeout" => {
                    if let Ok(val) = value.parse::<u32>() {
                        request_timeout = val;
                    }
                }
                "max_shadow_age" => {
                    if let Ok(val) = value.parse::<u32>() {
                        max_shadow_age = val;
                    }
                }
                "rollback_depth" => {
                    if let Ok(val) = value.parse::<u32>() {
                        rollback_depth = val;
                    }
                }
                "enable_rollback" => {
                    if let Ok(val) = value.parse::<bool>() {
                        enable_rollback = val;
                    }
                }
                _ => {} // Ignore unknown keys
            }
        }
    }
    
    // Export configuration as environment variables
    println!("export BACON_CYCLE_INTERVAL={}", cycle_interval);
    println!("export BACON_MAX_CYCLES={}", max_cycles);
    println!("export BACON_LOG_LEVEL=\"{}\"", log_level);
    println!("export BACON_ENABLE_METRICS={}", enable_metrics);
    println!("export BACON_SHADOW_CLEANUP={}", shadow_cleanup);
    println!("export BACON_GEMINI_MODEL=\"{}\"", gemini_model);
    println!("export BACON_CODEX_MODEL=\"{}\"", codex_model);
    println!("export BACON_AUDIT_MODEL=\"{}\"", audit_model);
    println!("export BACON_REQUEST_TIMEOUT={}", request_timeout);
    println!("export BACON_MAX_SHADOW_AGE={}", max_shadow_age);
    println!("export BACON_ROLLBACK_DEPTH={}", rollback_depth);
    println!("export BACON_ENABLE_ROLLBACK={}", enable_rollback);
}
