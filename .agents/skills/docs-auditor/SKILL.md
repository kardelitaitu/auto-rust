---
name: docs-auditor
description: Audit technical documentation against verified implementation state, identify drift, and keep docs aligned with code or approved specs.
---

# Documentation-Code Audit & Sync (DCAS)

## 1. Objective
Keep technical documentation accurate, traceable, and minimal. Prefer verified facts over assumptions.

## 2. Trigger Conditions
- User requests a doc audit, sync, or integrity review.
- Code changes affect public APIs, config, schemas, or runtime flow.
- A refactor may have changed documented behavior.

## 3. Source of Truth
- Implemented behavior comes from code.
- Intended behavior comes from an approved spec or ADR.
- If both exist and conflict, follow the more specific approved source.
- If the source of truth is unclear, pause and ask the user.
- Do not invent missing behavior.

## 4. Workflow Logic

### Phase 1: Ingestion
- Read the target documentation file.
- Identify truth anchors:
  - Function signatures and API endpoints.
  - Structs, enums, data shapes, and schemas.
  - Environment variables and config keys.
  - Runtime and operational flows.

### Phase 2: Verification
- Locate each truth anchor in code or approved specs.
- Compare doc claims against verified state.
- Classify each item as:
  - Match
  - Doc drift
  - Code drift
  - Ambiguous

### Phase 3: Repair
- If the doc is outdated, patch the doc to match verified state.
- If the code is outdated, flag the code drift clearly.
- Only patch code when the user explicitly asks for code changes or the task scope includes code remediation.
- Keep changes minimal.
- Preserve the document's structure and detail unless accuracy requires otherwise.

### Phase 4: Finalization
- Add one audit stamp at the top of the audited document only after verification is complete.
- Format: `last audited DD-MM-YY by [AgentName/ID]`
- Replace any existing stamp. Do not stack stamps.
- If the audit was not completed, do not stamp.

## 5. Operational Guidelines
- Prefer fast local search and file reads first.
- Run the narrowest relevant validation step before stamping when behavior changed.
- Report exactly what was synced.
- If evidence is incomplete or ambiguous, stop and ask.
- Do not guess.

## 6. Output Template
- Audit Status: MATCH / DOC DRIFT / CODE DRIFT / AMBIGUOUS
- Action Taken: NO CHANGE / PATCHED DOC / FLAGGED CODE
- Stamp Applied: `last audited DD-MM-YY by AgentName`

## 7. Notes
- Keep wording clear and stable.
- Do not simplify away useful technical detail.
- Favor small, verifiable corrections over broad rewrites.
