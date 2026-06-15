#!/bin/bash
# Fast compilation check for bacon pipeline (bash equivalent of check-fast.ps1)
set -euo pipefail

echo "=== check-fast: cargo check ==="
cargo check --lib --bins

echo ""
echo "=== check-fast: PASS ==="
