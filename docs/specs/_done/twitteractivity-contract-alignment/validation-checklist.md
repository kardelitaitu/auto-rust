# Validation Checklist

- Missing `duration_ms` resolves from the task config, not a hardcoded `300_000` fallback.
- `scroll_count` changes the runtime scroll budget in a visible way.
- Wrong-type numeric payload values return validation errors.
- Docs and examples for TwitterActivity match the runtime contract.
- Regression tests cover default, explicit, and malformed payload cases.
- `.\check-fast.ps1` still works during iteration.
- `.\check.ps1` still passes before handoff.
- `spec-lint.ps1` stays green for the package.
