#!/bin/bash
# .bacon/scripts/bacon-config.sh - Configuration management for Bacon system
# Handles environment variable overrides and default values

# Default configuration values
export BACON_CYCLE_INTERVAL=${BACON_CYCLE_INTERVAL:-10}
export BACON_MAX_CYCLES=${BACON_MAX_CYCLES:-0}
export BACON_LOG_LEVEL=${BACON_LOG_LEVEL:-info}
export BACON_ENABLE_METRICS=${BACON_ENABLE_METRICS:-true}
export BACON_SHADOW_CLEANUP=${BACON_SHADOW_CLEANUP:-true}

# AI provider configuration
export BACON_GEMINI_MODEL=${BACON_GEMINI_MODEL:-gemini-pro}
export BACON_CODEX_MODEL=${BACON_CODEX_MODEL:-codex-5.5}
export BACON_AUDIT_MODEL=${BACON_AUDIT_MODEL:-codex-5.4mini}
export BACON_REQUEST_TIMEOUT=${BACON_REQUEST_TIMEOUT:-30}

# Safety configuration
export BACON_MAX_SHADOW_AGE=${BACON_MAX_SHADOW_AGE:-24}
export BACON_ROLLBACK_DEPTH=${BACON_ROLLBACK_DEPTH:-10}
export BACON_ENABLE_ROLLBACK=${BACON_ENABLE_ROLLBACK:-true}

# External command configuration
export BACON_GEMINI_CMD=${BACON_GEMINI_CMD:-gemini-cli}
export BACON_CODEX_CMD=${BACON_CODEX_CMD:-codex-cli}
export BACON_JQ_CMD=${BACON_JQ_CMD:-jq}

# Load configuration from bacon.toml if available
load_toml_config() {
    local config_file="${SCRIPT_DIR}/../bacon.toml"
    local parser_script="${SCRIPT_DIR}/bacon-config-parser"
    local simple_parser="${SCRIPT_DIR}/bacon-config-parser-simple"
    
    if [[ -f "$config_file" ]]; then
        # Try simple parser first (more reliable)
        if [[ -f "$simple_parser" ]]; then
            if eval "$("$simple_parser" "$config_file")" 2>/dev/null; then
                return 0
            else
                echo "WARNING: Simple TOML parser failed, trying fallback parsing" >&2
            fi
        fi
        
        # Try complex parser if available
        if [[ -f "$parser_script" ]]; then
            if eval "$("$parser_script" "$config_file")" 2>/dev/null; then
                return 0
            else
                echo "WARNING: Complex TOML parser failed, using simple parsing" >&2
            fi
        fi
        
        # Fallback to simple grep-based parsing
        echo "INFO: Using simple TOML parsing as final fallback" >&2
        
        # Extract configuration values with simple parsing
        local config_values=(
            "cycle_interval:BACON_CYCLE_INTERVAL"
            "max_cycles:BACON_MAX_CYCLES"
            "log_level:BACON_LOG_LEVEL"
            "enable_metrics:BACON_ENABLE_METRICS"
            "shadow_cleanup:BACON_SHADOW_CLEANUP"
            "gemini_model:BACON_GEMINI_MODEL"
            "codex_model:BACON_CODEX_MODEL"
            "audit_model:BACON_AUDIT_MODEL"
            "request_timeout:BACON_REQUEST_TIMEOUT"
            "max_shadow_age:BACON_MAX_SHADOW_AGE"
            "rollback_depth:BACON_ROLLBACK_DEPTH"
            "enable_rollback:BACON_ENABLE_ROLLBACK"
        )
        
        for config_pair in "${config_values[@]}"; do
            local key="${config_pair%:*}"
            local var="${config_pair#*:}"
            
            if grep -q "$key" "$config_file" 2>/dev/null; then
                local value
                value=$(grep "$key" "$config_file" | head -1 | sed 's/.*= *//' | tr -d ' "')
                if [[ -n "$value" ]]; then
                    export "$var"="$value"
                fi
            fi
        done
    fi
}

# Validate configuration
validate_config() {
    local errors=0
    
    # Check numeric values
    if ! [[ "$BACON_CYCLE_INTERVAL" =~ ^[0-9]+$ ]]; then
        echo "ERROR: BACON_CYCLE_INTERVAL must be a positive integer" >&2
        ((errors++))
    fi
    
    if ! [[ "$BACON_MAX_CYCLES" =~ ^[0-9]+$ ]]; then
        echo "ERROR: BACON_MAX_CYCLES must be a non-negative integer" >&2
        ((errors++))
    fi
    
    # Check command availability
    for cmd_var in BACON_GEMINI_CMD BACON_CODEX_CMD BACON_JQ_CMD; do
        local cmd_value="${!cmd_var}"
        if [[ -n "$cmd_value" ]] && ! command -v "$cmd_value" >/dev/null 2>&1; then
            echo "WARNING: Command not found: $cmd_value (${cmd_var})" >&2
        fi
    done
    
    return $errors
}

# Load and validate configuration
load_toml_config
validate_config

# Log configuration (debug mode only)
if [[ "${BACON_LOG_LEVEL:-info}" == "debug" ]]; then
    echo "Bacon Configuration:" >&2
    echo "  Cycle Interval: ${BACON_CYCLE_INTERVAL}s" >&2
    echo "  Max Cycles: ${BACON_MAX_CYCLES}" >&2
    echo "  Log Level: ${BACON_LOG_LEVEL}" >&2
    echo "  Enable Metrics: ${BACON_ENABLE_METRICS}" >&2
    echo "  Gemini Model: ${BACON_GEMINI_MODEL}" >&2
    echo "  Codex Model: ${BACON_CODEX_MODEL}" >&2
fi
