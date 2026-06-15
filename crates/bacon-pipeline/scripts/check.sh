#!/bin/bash
# Full test suite for bacon pipeline (bash equivalent of check.ps1)
set -euo pipefail

echo "=== check: cargo test ==="
cargo test

echo ""
echo "=== check: PASS ==="
