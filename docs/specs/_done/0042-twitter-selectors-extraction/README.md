# Twitter Selectors JS Extraction

Status: `done`

Owner: `spec-agent`
Implementer: `pending`

## Summary

The `src/utils/twitter/twitteractivity_selectors.rs` file is 783 lines long and consists primarily of Rust functions returning large, multi-line raw JavaScript string literals. Embedding complex JS logic inside Rust strings breaks IDE syntax highlighting, prevents JS linting, and makes DOM manipulation scripts difficult to maintain. This spec extracts these scripts into dedicated `.js` files loaded at compile time via `include_str!()`.

## Scope

- In scope:
  - Creating a `src/utils/twitter/js/` directory to house the extracted scripts.
  - Migrating the raw JS strings from `twitteractivity_selectors.rs` into individual `.js` files.
  - Updating the Rust functions to return `include_str!("js/<script>.js")`.
  - Keeping the existing CSS selector constants (e.g., `HOME_LOGO_SELECTOR`) in the Rust file.
- Out of scope:
  - Modifying the actual JavaScript logic or DOM traversal algorithms.
  - Refactoring the consumers of these selector functions.
  - Adding a JS build step or bundler.

## Files

- `spec.yaml`
- `plan.md`
- `validation.md`
- `notes.md`

## Rules

- Keep the spec short.
- Run `spec-lint.ps1` before handoff.
- Use `.\check-fast.ps1` while iterating.
- Use the archive helper `.\spec-archive.ps1` to move to `_done/`.

## Next Step

Wait for the implementer agent to extract the JavaScript files.
