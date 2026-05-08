# Navigation and DOM Modularization

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

The `src/utils/navigation.rs` file is currently 1,656 lines long and suffers from a Single Responsibility Principle violation. It mixes two distinct concerns: page routing/lifecycle operations (e.g., `goto`, `go_back`, `wait_for_load`, `page_url`) and DOM querying/element inspection operations (e.g., `selector_exists`, `selector_is_visible`, `selector_text`, `selector_attr`). This spec proposes splitting the DOM inspection logic out into a new, dedicated `src/utils/dom.rs` module, keeping `navigation.rs` strictly focused on browser routing and page transitions.

## Scope

- In scope:
  - Creating `src/utils/dom.rs`.
  - Migrating all DOM query functions (e.g., `selector_exists`, `selector_is_visible`, `selector_text`, `selector_html`, `selector_attr`, `selector_value`, `wait_for_selector`, `wait_for_visible_selector`, and their `css_*` or `ax_*` underlying helpers) from `navigation.rs` to `dom.rs`.
  - Updating `src/utils/mod.rs` to export the new `dom` module.
  - Updating internal and external imports to point to `crate::utils::dom::*` where appropriate (or updating `prelude.rs` if these are re-exported there).
- Out of scope:
  - Modifying the underlying JavaScript evaluation logic or Chromiumoxide CDP wrappers.
  - Introducing new DOM query capabilities.

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

Wait for the implementer agent to extract the DOM functions.
