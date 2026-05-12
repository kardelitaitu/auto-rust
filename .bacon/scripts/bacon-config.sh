#!/bin/bash
# .bacon/scripts/bacon-config.sh - Configuration management for Bacon system
# Handles environment variable overrides and default values

# Environment variables override bacon.toml. Defaults are applied after TOML loading.

# Load configuration from bacon.toml if available
load_toml_config() {
    local config_file="${SCRIPT_DIR}/../bacon.toml"

    if [[ -f "$config_file" ]]; then
        # Extract scalar configuration values without generated parser binaries.
        local config_values=(
            "log_level:BACON_LOG_LEVEL"
            "enable_metrics:BACON_ENABLE_METRICS"
            "gemini_model:BACON_GEMINI_MODEL"
            "codex_model:BACON_CODEX_MODEL"
            "audit_model:BACON_AUDIT_MODEL"
            "request_timeout_seconds:BACON_REQUEST_TIMEOUT"
            "max_shadow_age_hours:BACON_MAX_SHADOW_AGE"
            "rollback_depth:BACON_ROLLBACK_DEPTH"
            "enable_rollback:BACON_ENABLE_ROLLBACK"
            "enable_auto_apply:BACON_AUTO_APPLY"
            "require_full_check_for_auto_apply:BACON_REQUIRE_FULL_CHECK"
        )

        for config_pair in "${config_values[@]}"; do
            local key="${config_pair%:*}"
            local var="${config_pair#*:}"

            if grep -Eq "^[[:space:]]*${key}[[:space:]]*=" "$config_file" 2>/dev/null; then
                local value
                value=$(grep -E "^[[:space:]]*${key}[[:space:]]*=" "$config_file" | head -1 | sed 's/^[^=]*= *//' | sed 's/[[:space:]]*$//' | tr -d '"')
                if [[ -n "$value" && -z "${!var+x}" ]]; then
                    export "$var"="$value"
                fi
            fi
        done
    fi
}

# Apply defaults after TOML loading so explicit environment overrides win.
apply_default_config() {
    export BACON_CYCLE_INTERVAL=${BACON_CYCLE_INTERVAL:-10}
    export BACON_MAX_CYCLES=${BACON_MAX_CYCLES:-0}
    export BACON_LOG_LEVEL=${BACON_LOG_LEVEL:-info}
    export BACON_ENABLE_METRICS=${BACON_ENABLE_METRICS:-true}
    export BACON_SHADOW_CLEANUP=${BACON_SHADOW_CLEANUP:-true}
    export BACON_GEMINI_MODEL=${BACON_GEMINI_MODEL:-gemini-pro}
    export BACON_CODEX_MODEL=${BACON_CODEX_MODEL:-codex-5.5}
    export BACON_AUDIT_MODEL=${BACON_AUDIT_MODEL:-codex-5.4mini}
    export BACON_REQUEST_TIMEOUT=${BACON_REQUEST_TIMEOUT:-30}
    export BACON_MAX_SHADOW_AGE=${BACON_MAX_SHADOW_AGE:-24}
    export BACON_ROLLBACK_DEPTH=${BACON_ROLLBACK_DEPTH:-10}
    export BACON_ENABLE_ROLLBACK=${BACON_ENABLE_ROLLBACK:-true}
    export BACON_AUTO_APPLY=${BACON_AUTO_APPLY:-false}
    export BACON_REQUIRE_FULL_CHECK=${BACON_REQUIRE_FULL_CHECK:-true}
    export BACON_GEMINI_CMD=${BACON_GEMINI_CMD:-gemini-cli}
    export BACON_CODEX_CMD=${BACON_CODEX_CMD:-codex-cli}
    export BACON_JQ_CMD=${BACON_JQ_CMD:-jq}
}

# Validate configuration
validate_config() {
    local errors=0

    # Check numeric values
    if ! [[ "$BACON_CYCLE_INTERVAL" =~ ^[0-9]+$ ]]; then
        echo "ERROR: BACON_CYCLE_INTERVAL must be a positive integer" >&2
        errors=$((errors + 1))
    fi

    if ! [[ "$BACON_MAX_CYCLES" =~ ^[0-9]+$ ]]; then
        echo "ERROR: BACON_MAX_CYCLES must be a non-negative integer" >&2
        errors=$((errors + 1))
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
apply_default_config
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
