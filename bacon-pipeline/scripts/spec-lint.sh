#!/bin/bash
# Spec lint validation for bacon pipeline (bash equivalent of spec-lint.ps1)
set -euo pipefail

SPEC_DIR="${1:-}"

if [ -z "$SPEC_DIR" ]; then
    echo "Usage: $0 <spec-directory>"
    exit 1
fi

if [ ! -d "$SPEC_DIR" ]; then
    echo "ERROR: Spec directory not found: $SPEC_DIR"
    exit 1
fi

errors=0

# Check required files exist
for file in spec.yaml plan.md validation.md; do
    if [ ! -f "$SPEC_DIR/$file" ]; then
        echo "ERROR: Missing required file: $file"
        errors=$((errors + 1))
    fi
done

# Validate YAML structure in spec.yaml
if [ -f "$SPEC_DIR/spec.yaml" ]; then
    if ! python3 -c "import yaml; yaml.safe_load(open('$SPEC_DIR/spec.yaml'))" 2>/dev/null; then
        echo "ERROR: spec.yaml is not valid YAML"
        errors=$((errors + 1))
    else
        # Check status field
        STATUS=$(python3 -c "
import yaml
with open('$SPEC_DIR/spec.yaml') as f:
    data = yaml.safe_load(f)
print(data.get('status', ''))
" 2>/dev/null)
        if [ "$STATUS" = "in-progress" ]; then
            echo "WARNING: spec has status 'in-progress' — may be stale"
        fi
        if [ "$STATUS" = "needs-human-approval" ]; then
            echo "WARNING: spec marked as 'needs-human-approval'"
        fi
    fi
fi

# Check validation.md for failure report
if [ -f "$SPEC_DIR/validation.md" ]; then
    if head -1 "$SPEC_DIR/validation.md" | grep -qi "failure report\|error"; then
        echo "WARNING: validation.md contains failure report"
    fi
fi

# Check plan.md for generic acceptance criteria
if [ -f "$SPEC_DIR/plan.md" ]; then
    if grep -qi "acceptance criteria" "$SPEC_DIR/plan.md" 2>/dev/null; then
        if grep -qi "todo\|tbd\|placeholder" "$SPEC_DIR/plan.md" 2>/dev/null; then
            echo "WARNING: plan.md contains placeholder acceptance criteria"
        fi
    fi
fi

if [ "$errors" -gt 0 ]; then
    echo "FAIL: $errors error(s) found"
    exit 1
fi

echo "PASS: spec is valid"
