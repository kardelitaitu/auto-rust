# Dead Code Cleanup Round 2#

Status: `pending`

Owner: `spec-agent`
Implementer: `pending`

## Summary
After specs 0017/0023/0024/0025 cleanup, there might be more dead code: unused imports, unreachable code, deprecated functions. This spec proposes a systematic cleanup pass using `cargo clippy --fix` and manual review.

## Scope
- **In scope**: Remove dead code identified by clippy, unused imports, unreachable code paths.
- **Out of scope**: Changing functionality, removing public API.

## Next Step
Run `cargo clippy --fix` and review all warnings.

# Baseline#

## What I Find
Potentially dead code after recent refactorings (spec 0017 DSL, 0018 mouse, 0021 twitter).

## What I Claim
Dead code increases maintenance burden and confuses new developers.

## What Is the Proof
1. Recent large refactorings (DSL 2,362->modular, mouse refactoring, twitter helpers)
2. Clippy warnings may exist for unused code
3. Some functions may be replaced but not removed
