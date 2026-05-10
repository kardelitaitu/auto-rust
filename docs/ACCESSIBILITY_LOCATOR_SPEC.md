# Accessibility Locator Spec (v1)

last audited 08-05-26 by KiloCode

Status: Approved for implementation (2026-04-29), implementation in progress (feature-gated parser + shared resolver with action-path wiring)
Owner: Runtime/Task API maintainers
Depends on: `PROPOSAL_ACCESSIBILITY_LOCATOR.md`

## 1. Purpose

Define the exact runtime contract for accessibility locator support behind existing `TaskContext` selector-based APIs.

This spec is implementation-facing and testable.

## 2. Scope

In scope:
- Parse selector input into either CSS selector or accessibility locator.
- Resolve accessibility locator by role + accessible name (+ optional scope).
- Keep existing `TaskContext` method signatures unchanged.
- Provide deterministic error semantics.

Out of scope (v1):
- New public task-facing methods (`get_by_role`, `get_by_label`, etc.).
- Non-role locator families.
- Heuristic auto-correction of invalid locator strings.

## 3. Existing Baseline (Verified)

Current runtime treats selector inputs as CSS selectors and uses DOM query operations (`document.querySelector(...)`) in shared helpers.

Implication:
