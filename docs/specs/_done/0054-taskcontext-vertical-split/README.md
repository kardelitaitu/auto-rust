# Split task_context.rs into domain-focused submodule files

Status: `done`

Owner: `spec-agent`
Implementer: `pending`

## Summary

`src/runtime/task_context.rs` is 5,607 lines — 7.5% of all Rust source in the project. It already has 5 submodules extracted (`click_learning`, `interaction`, `interaction_pipeline`, `query`, `types`), but the bulk of methods remain. This spec extracts 7 more domain-focused submodules (cookies, clipboard, data_files, http, navigation, session_io, style), reducing the file to ~3,500 lines. All extractions are pure moves — no behavioral changes, no public API changes.

## Scope

- In scope:
  - Extract cookies.rs (5 methods, ~600 lines)
  - Extract clipboard.rs (5 methods, ~200 lines)
  - Extract data_files.rs (9 methods, ~400 lines)
  - Extract http.rs (3 methods, ~200 lines)
  - Extract navigation.rs (5 methods, ~330 lines)
  - Extract session_io.rs (6 methods, ~550 lines)
  - Extract style.rs (5 methods, ~200 lines)
  - Register all new submodules in task_context.rs
- Out of scope:
  - Splitting click/hover/wait/scroll/pause/queries (deferred)
  - Changing method signatures or behavior
  - Adding new functionality

## Files

- spec.yaml
- baseline.md
- plan.md
- validation-checklist.md
- ci-commands.md
- decisions.md
- quality-rules.md

## Next Step

Implementer: create each submodule file, move method blocks, register modules, verify.
