// bacon-config-parser.rs - Rust utility for parsing bacon.toml configuration
// Compile with: rustc --edition 2021 bacon-config-parser.rs -o bacon-config-parser

use std::collections::HashMap;
use std::env;
use std::fs;
use std::process;

#[derive(Debug, Default)]
struct BaconConfig {
    cycle_interval: u32,
    max_cycles: u32,
    log_level: String,
    enable_metrics: bool,
    shadow_cleanup: bool,
    gemini_model: String,
    codex_model: String,
    audit_model: String,
    request_timeout: u32,
    max_shadow_age: u32,
    rollback_depth: u32,
    enable_rollback: bool,
}

impl BaconConfig {
    fn from_toml_string(toml_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = BaconConfig::default();
        
        // Simple TOML parsing - looking for key = value pairs
        for line in toml_str.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }
            
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                
                match key {
                    "cycle_interval" => config.cycle_interval = value.parse().unwrap_or(10),
                    "max_cycles" => config.max_cycles = value.parse().unwrap_or(0),
                    "log_level" => config.log_level = value.to_string(),
                    "enable_metrics" => config.enable_metrics = value.parse().unwrap_or(true),
                    "shadow_cleanup" => config.shadow_cleanup = value.parse().unwrap_or(true),
                    "gemini_model" => config.gemini_model = value.to_string(),
                    "codex_model" => config.codex_model = value.to_string(),
                    "audit_model" => config.audit_model = value.to_string(),
                    "request_timeout" => config.request_timeout = value.parse().unwrap_or(30),
                    "max_shadow_age" => config.max_shadow_age = value.parse().unwrap_or(24),
                    "rollback_depth" => config.rollback_depth = value.parse().unwrap_or(10),
                    "enable_rollback" => config.enable_rollback = value.parse().unwrap_or(true),
                    _ => {} // Ignore unknown keys
                }
            }
        }
        
        Ok(config)
    }
    
    fn export_env_vars(&self) {
        // Export configuration as environment variables
        println!("export BACON_CYCLE_INTERVAL={}", self.cycle_interval);
        println!("export BACON_MAX_CYCLES={}", self.max_cycles);
        println!("export BACON_LOG_LEVEL=\"{}\"", self.log_level);
        println!("export BACON_ENABLE_METRICS={}", self.enable_metrics);
        println!("export BACON_SHADOW_CLEANUP={}", self.shadow_cleanup);
        println!("export BACON_GEMINI_MODEL=\"{}\"", self.gemini_model);
        println!("export BACON_CODEX_MODEL=\"{}\"", self.codex_model);
        println!("export BACON_AUDIT_MODEL=\"{}\"", self.audit_model);
        println!("export BACON_REQUEST_TIMEOUT={}", self.request_timeout);
        println!("export BACON_MAX_SHADOW_AGE={}", self.max_shadow_age);
        println!("export BACON_ROLLBACK_DEPTH={}", self.rollback_depth);
        println!("export BACON_ENABLE_ROLLBACK={}", self.enable_rollback);
    }
}

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
    
    // Parse configuration
    let config = match BaconConfig::from_toml_string(&toml_content) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error parsing TOML: {}", e);
            process::exit(1);
        }
    };
    
    // Export environment variables
    config.export_env_vars();
}
