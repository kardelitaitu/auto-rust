# Documentation Generation Automation#

Status: `pending`

Owner: `spec-agent`
Implementer: `pending`

## Summary
Rustdoc exists but there's no consolidated API docs or architecture diagrams. This spec proposes adding automated documentation generation and an `ARCHITECTURE.md` for new developer onboarding.

## Scope
- **In scope**: Add `cargo doc --all-features` CI step, create `docs/ARCHITECTURE.md`.
- **Out of scope**: Changing code documentation, rewriting existing docs.

## Next Step
Research documentation generation tools and create initial architecture document.

# Baseline#

## What I Find
The codebase has rustdoc comments but no consolidated API reference or architecture overview.

## What I Claim
Hard to onboard new developers without guided tour of the codebase architecture.

## What Is the Proof
1. No `docs/ARCHITECTURE.md` file exists
2. No CI step for documentation generation
3. Rustdoc is scattered across 27+ twitter modules alone
