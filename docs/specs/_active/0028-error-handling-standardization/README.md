# Error Handling Standardization

Status: `pending`

Owner: `spec-agent`
Implementer: `pending`

## Summary
Error types are scattered across `anyhow::Result`, custom errors in multiple modules. This spec proposes standardizing on `anyhow` + context for consistent error handling throughout the codebase.

## Scope
- **In scope**: Standardize error handling patterns, add context to errors.
- **Out of scope**: Changing error semantics, breaking API compatibility.

## Next Step
Audit all error types and handling patterns across the codebase.

# Baseline

## What I Find
The codebase uses a mix of `anyhow::Result`, custom error types, and inline error creation.

## What I Claim
Inconsistent error handling makes debugging harder and errors less informative.

## What Is the Proof
1. Custom error types in multiple modules
2. Some errors lack context (file/line information)
3. Mix of `anyhow`, `thiserror`, custom errors
