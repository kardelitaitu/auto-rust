# Task Migration Guide: Accessibility Locator

last audited 08-05-26 by Kilo

## Purpose

This guide explains how to migrate task code from CSS-first selectors to accessibility locator-first selectors using the current runtime behavior.

Goals:
- reliable
- scalable
- easy to use

Companion references:
- `docs/ACCESSIBILITY_LOCATOR_SPEC.md`
- `PROPOSAL_ACCESSIBILITY_LOCATOR.md`
- `src/task/SELECTOR.md`

## Scope and Constraints

In scope:
- Task-level selector migration using existing `TaskContext` APIs (`api.click`, `api.visible`, `api.wait_for`, etc.)
- Accessibility locator grammar (`role=...`) with CSS fallback
- Deterministic error handling and telemetry alignment

Out of scope:
- New task-facing APIs (`get_by_role`, etc.)
- Changing `TaskContext` method signatures
- Silent fallback from malformed locator grammar

## Runtime Contract (Must Follow)
