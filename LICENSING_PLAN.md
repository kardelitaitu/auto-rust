# Licensing and Monthly Subscription Plan

## Objective

Implement a reliable monthly subscription licensing system for the CLI that:
- enables paid plans and team seats,
- enforces limits without breaking user trust,
- supports offline usage with a grace period,
- and is maintainable with clear operational processes.

## Success Criteria

- Paid users can subscribe, receive a license, and activate the CLI in under 5 minutes.
- License checks are secure (signed tokens, server-side authority, revocation support).
- Plan entitlements are enforced at runtime (sessions, tasks/features, usage limits).
- Non-paying users have a clear free path and clear upgrade flow.
- Churn and support load stay manageable with self-service commands and docs.

## Product and Pricing Model

### Recommended Tiers

- Free
  - limited sessions/task throughput,
  - non-commercial or limited-commercial usage depending on policy,
  - no SLA support.
- Pro (monthly per seat)
  - higher session/task limits,
  - premium tasks/features,
  - standard support and updates.
- Team/Enterprise (monthly per seat or annual)
  - multi-seat management,
  - admin controls,
  - priority support and commercial terms.

### Entitlement Strategy

Prefer gradual feature gates over hard lockouts:
- session concurrency cap by plan,
- premium task set restricted to paid plans,
- advanced automation modules restricted to paid plans,
- optional cloud features (dashboard/history/scheduler) paid-only.

## Licensing Architecture

## Components

- Billing Provider: Stripe (recommended initial choice).
- License API Service: owns subscription/license truth and policy decisions.
- CLI License Client: handles auth, token storage, refresh, and local validation.
- Admin Portal (phase 2+): support operations, revocation, seat management.

### Trust Model

- Server is the source of truth.
- CLI receives signed license tokens (JWT/PASETO; prefer asymmetric signing, Ed25519).
- CLI validates signature and expiry locally.
- CLI periodically refreshes token from server.
- Revocation and plan changes are applied via refresh window.

### Offline Policy

- Allow offline execution with grace window (3-7 days recommended).
- After grace expiry, restrict paid-only features until refresh succeeds.
- Always preserve read-only/status commands to reduce lockout frustration.

## Data Model (Minimum)

- Customer
  - id, email, organization_name, created_at
- Subscription
  - id, customer_id, provider_subscription_id, plan_id, status, renews_at, canceled_at
- License
  - id, subscription_id, license_key/public_id, status, max_seats, metadata
- SeatActivation
  - id, license_id, device_fingerprint, hostname, user_alias, last_seen_at
- LicenseTokenIssueLog
  - id, license_id, token_jti, issued_at, expires_at, revoked

## CLI Command Surface

Add `license` command group:
- `auto-rust license login`
  - authenticate and bind local machine/account.
- `auto-rust license status`
  - show plan, expiry, seats used, offline grace remaining.
- `auto-rust license refresh`
  - force token refresh and entitlement sync.
- `auto-rust license logout`
  - remove local credentials/token.
- `auto-rust license whoami` (optional)
  - show customer/team context.

## Runtime Enforcement Points

Integrate license checks at orchestration boundaries to keep task code thin:
- before task-group execution starts,
- when calculating allowed parallel sessions,
- when resolving premium task availability,
- and when reporting final run summaries.

Design principle:
- resolve entitlements once per run,
- pass normalized entitlement object into orchestration/task context,
- avoid scattered ad-hoc checks inside each task implementation.

## Implementation Phases

## Phase 0 - Foundation (1 week)

- Finalize tier definitions and entitlement matrix.
- Define legal stance (commercial use, seat definition, refund and cancellation policy).
- Pick billing provider and create test products/plans.
- Document threat model and abuse assumptions.

Deliverables:
- entitlement matrix document,
- API contract draft,
- legal policy draft.

## Phase 1 - Licensing Backend MVP (1-2 weeks)

- Build License API service with endpoints:
  - `POST /auth/login`
  - `POST /license/activate`
  - `GET /license/status`
  - `POST /license/refresh`
  - `POST /license/logout`
- Implement webhook handler for subscription events:
  - payment success/failure,
  - cancel/reactivate,
  - plan changes.
- Implement token signing and rotation-ready key handling.

Deliverables:
- deployable backend MVP,
- webhook processing with idempotency,
- basic admin scripts for revoke/list activations.

## Phase 2 - CLI Integration (1-2 weeks)

- Add `license` command group and local secure storage.
- Add local token verification + refresh flow.
- Add orchestration-level entitlement guardrails.
- Add UX copy for upgrade prompts and lock reasons.

Deliverables:
- end-to-end activation in staging,
- enforcement of at least 2 paid entitlements,
- clear CLI messaging for all major states.

## Phase 3 - Hardening and Observability (1 week)

- Add audit logs for token issuance and refresh failures.
- Add retry/backoff and resilience around network failures.
- Add structured metrics:
  - activation success rate,
  - refresh success rate,
  - grace-window fallbacks,
  - entitlement-denied counts.
- Security review for token handling and local storage.

Deliverables:
- dashboards and alerts,
- security checklist complete,
- incident runbook.

## Phase 4 - Commercial Launch (1 week)

- Publish pricing page and purchase flow.
- Publish docs: quickstart, seat management, cancellations, troubleshooting.
- Run closed beta with selected users.
- Iterate on friction points before public launch.

Deliverables:
- public docs and upgrade links,
- beta report and fixes,
- launch readiness checklist sign-off.

## API Contract (MVP)

### `POST /auth/login`
- Input: email + magic link/OAuth token
- Output: session token (short-lived)

### `POST /license/activate`
- Input: session token + device fingerprint + host metadata
- Output: signed license token + entitlement payload

### `GET /license/status`
- Input: bearer token
- Output: current plan, seat usage, renewal date, status

### `POST /license/refresh`
- Input: refresh credential or valid token
- Output: new signed token + updated entitlements

### `POST /license/logout`
- Input: current device/session identity
- Output: revoke local session (best-effort)

## Security and Abuse Controls

- Use asymmetric signatures; keep private key server-side only.
- Token TTL short enough for revocation response (for example 24h).
- Include `jti`, `license_id`, `plan`, `exp` in token claims.
- Maintain revocation list and check on refresh.
- Rate-limit activation and refresh endpoints.
- Soft device binding; avoid overly strict hardware locking to reduce support burden.

## Legal and Policy Checklist

- Draft EULA/commercial terms for CLI usage.
- Define seat policy clearly (human seat vs machine seat).
- Define trial terms, cancellation behavior, and grace policy.
- Add privacy disclosures for telemetry and license validation data.
- Add enforcement language for abuse without overreaching consumer rights.

## Testing Strategy

- Unit tests:
  - token validation,
  - entitlement resolution,
  - offline grace transitions.
- Integration tests:
  - full login/activate/refresh flow against staging API,
  - webhook-driven plan downgrade/upgrade behavior.
- Scenario tests:
  - expired card -> paid features restricted,
  - restored payment -> access restored after refresh,
  - revoked token -> denied after refresh cycle.

## KPI Targets (First 90 Days)

- activation success rate >= 95%
- license refresh success rate >= 98%
- support tickets related to licensing < 10% of active paid accounts
- conversion from trial/free to paid >= 5-15% (depends on channel quality)

## Risks and Mitigations

- Platform volatility in automation domain
  - mitigate by transparent scope and frequent compatibility updates.
- False-positive lockouts
  - mitigate with offline grace, clear messages, and self-service refresh.
- Webhook/billing desync
  - mitigate with periodic reconciliation job and idempotent webhook processing.
- Over-complex pricing early
  - mitigate with 2-3 simple plans first, expand after usage signals.

## Execution Checklist

- [ ] Finalize plans and entitlement matrix.
- [ ] Build and deploy license API MVP.
- [ ] Integrate CLI license commands and runtime checks.
- [ ] Implement billing webhooks and reconciliation.
- [ ] Complete security review and key management setup.
- [ ] Publish legal terms and user documentation.
- [ ] Run beta and measure activation/refresh KPIs.
- [ ] Launch publicly and monitor support/retention signals.

## Recommended Immediate Next Step

Start with Phase 0 by locking the entitlement matrix and API contract first. This prevents rework across CLI, backend, and billing integration.

## Task Handler Plan (Runtime Tasks, Not Compiled `.rs`)

## Goal

Move task definitions out of Rust source so new/updated tasks can be shipped without rebuilding the `.exe`.

Target result:
- core engine stays in Rust binary,
- task logic is loaded at runtime from external files/packages,
- task execution remains safe, validated, and observable.

## Locked Decisions (MVP)

- Task files are YAML only (`.yaml`, `.yml`) for MVP.
- Runtime task execution is TaskContext-only (closed opcode list mapped to TaskContext APIs).
- Task discovery is allowlist-only via configured scan folders (supports multiple folders).
- No scripted plugin runtime (JS/WASM) in MVP; reconsider after stable v1 rollout.
- Signatures are required for official/premium task packs; unsigned local tasks are allowed by policy flag.

## Recommended Architecture

Use a declarative task model for MVP:
- task files in YAML define steps and parameters,
- compiler maps each opcode to existing TaskContext methods,
- orchestrator retains ownership of retries/timeouts/session fan-out.

Script/plugin extensibility is explicitly deferred until post-MVP.

## Directory and Package Layout

Proposed runtime locations:
- `tasks/builtin/` - bundled baseline tasks shipped with installer.
- `tasks/custom/` - user-defined or downloaded tasks.
- `tasks/registry.json` - local index with metadata, version, checksum, signature state.

Configuration contract (supports multiple folders):
- `task_scan_folders`: ordered list of folders to scan for task YAML files.
- `task_scan_globs`: fixed to `["*.yaml", "*.yml"]` for MVP.
- `task_policy.allow_unsigned_local`: boolean for local development.

Example config:

```toml
[tasks]
task_scan_folders = ["tasks/builtin", "tasks/custom", "D:/team-shared/auto-rust-tasks"]
task_scan_globs = ["*.yaml", "*.yml"]
allow_unsigned_local = true
```

Task package format:
- `task.toml` (metadata)
- `steps.yaml` (declarative flow)
- optional assets (templates/selectors/prompts)
- optional signature file (`.sig`)

## Task Schema (MVP)

Each task definition should include:
- `id` (canonical name),
- `version`,
- `description`,
- `requires` (minimum CLI version, required capabilities),
- `entitlements` (plan gates),
- `inputs` schema,
- `steps` array (action + target + retry + timeout + post_wait),
- `outputs` contract.

Important:
- keep aliases and normalization in shared validation logic,
- do not let task files bypass orchestrator policy (timeouts, retries, health checks).
- enforce a closed opcode set that maps only to `TaskContext` APIs (no raw CDP/JS opcodes).

### TaskContext-Only Task Example

This example models: go to `example.com`, click a text box, then type text.

Notes:
- navigation is handled by runtime precondition (`start_url`) before step execution,
- `steps` itself uses only `TaskContext`-mapped ops.

```yaml
task_schema_version: "1.0"
id: "example_fill_textbox"
version: "0.1.0"
description: "Open example.com, click textbox, then type"

runtime:
  start_url: "https://example.com"

inputs:
  text:
    type: string
    required: true
    default: "hello from auto-rust"

steps:
  - op: url_contains
    args:
      value: "example.com"

  - op: wait_for_visible
    args:
      selector: "input[type='text'], textarea"
      timeout_ms: 8000

  - op: click
    args:
      selector: "input[type='text'], textarea"

  - op: focus
    args:
      selector: "input[type='text'], textarea"

  - op: r#type
    args:
      text: "{{inputs.text}}"
```

Opcode mapping for this example:
- `url_contains` -> `api.url()` + string match check
- `wait_for_visible` -> `api.wait_for_visible(...)`
- `click` -> `api.click(...)`
- `focus` -> `api.focus(...)`
- `r#type` -> `api.r#type(...)`

## Runtime Flow

1. CLI receives task request.
2. Task registry scans only configured YAML task folders (allowlist, multi-folder) and resolves task id -> local package.
3. Loader verifies checksum/signature and schema version.
4. Validator resolves inputs and entitlements.
5. Compiler maps steps into existing `TaskContext` verbs (`click`, `pause`, `focus`, `keyboard`, etc.).
6. Orchestrator executes compiled plan across sessions in parallel.
7. Reporter emits per-step and per-session outcomes.

## Task Handler Components

- `TaskRegistry`
  - discovers tasks from configured directories,
  - tracks versions/checksums/signature status.
- `TaskLoader`
  - reads package files,
  - validates schema and capability requirements.
- `TaskCompiler`
  - converts declarative steps into executable actions on top of current runtime API.
- `TaskExecutor`
  - runs compiled task through existing orchestration/session systems.
- `TaskStore`
  - optional remote sync/update channel for signed task packages.

## Non-Goals (MVP)

- No arbitrary script execution from task files.
- No direct CDP primitives exposed in task schema.
- No auto-discovery outside explicit task scan folders.
- No dynamic remote code execution during run start.

## CLI Surface Additions

- `auto-rust task list`
- `auto-rust task inspect <id>`
- `auto-rust task validate <path>`
- `auto-rust task install <package-or-url>`
- `auto-rust task uninstall <id>`
- `auto-rust task update [id]`

These commands make runtime tasks operable without touching Rust code.

## Security Model

- Only scan task files from an explicit folder allowlist (support multiple folders).
- Restrict discovery to YAML task files in those folders (for example `*.yaml`, `*.yml`).
- Do not execute tasks discovered outside configured task scan folders.
- Only allow task actions from an approved opcode list mapped to `TaskContext` verbs.
- Validate all task files against strict schema before execution.
- Enforce max step count, recursion depth, and per-step timeout.
- Require signatures for official/premium task packages.
- Keep optional script plugins sandboxed (no raw filesystem/network by default unless explicitly granted).

Additional controls:
- Canonicalize and normalize scan paths before traversal to block path traversal/symlink escape.
- Enforce max YAML file size and max includes/references to avoid resource abuse.
- Cache validated task fingerprints to avoid repeated parse/validate overhead on every run.

## Licensing Integration for Runtime Tasks

Runtime tasks become a licensing primitive:
- Free: can run builtin/community tasks only.
- Pro: can install signed premium task packs.
- Team/Enterprise: includes private task registry + policy controls.

At load time:
- check license entitlement for requested task id/package,
- deny with clear upgrade guidance when entitlement missing.

## Compatibility and Versioning

- Add `task_schema_version` and `min_cli_version`.
- Provide migration adapters for one previous schema version.
- Fail fast with actionable error if incompatible.
- Maintain canonical task ids across `task/mod.rs`, CLI parser, and docs to prevent drift.

## Migration Strategy from `.rs` Tasks

Phase A - Foundation
- implement loader/validator with no behavior change.
- support one pilot task defined declaratively.

Phase B - Dual Runtime
- keep existing Rust tasks and new runtime tasks side by side.
- route by source type (compiled vs external).
- compare outcomes in shadow mode for selected tasks.

Phase C - Gradual Porting
- port stable tasks to declarative format first.
- keep complex/edge tasks in Rust until plugin layer is mature.

Phase D - Default External Tasks
- new tasks must be external by default.
- Rust task implementations reserved for engine-level capabilities.

Rollback strategy:
- keep feature flag `runtime_tasks_enabled` defaulting to `false` until pilot passes.
- support immediate rollback to compiled task resolution if runtime validation errors spike.

## Testing Strategy for Task Handler

- Schema tests: invalid files rejected with deterministic errors.
- Compiler tests: step mapping to `TaskContext` verbs is correct.
- Golden tests: same input task package yields same execution plan.
- Integration tests: runtime task executes correctly across multiple sessions.
- Security tests: forbidden opcodes/capability escalation blocked.

## Risks and Mitigations (Task Handler)

- Too much flexibility can reduce safety
  - mitigate with declarative-first model and strict opcode whitelist.
- Task quality inconsistency
  - mitigate with package signing, validation CLI, and lint rules.
- Version fragmentation
  - mitigate with schema versioning and compatibility matrix.
- Performance overhead from parsing at runtime
  - mitigate with cached compiled plans and checksum-based invalidation.

## Implementation Timeline (Task Handler)

- Week 1: schema definition + loader + validator + `task validate`.
- Week 2: compiler to existing `TaskContext` + pilot task end-to-end.
- Week 3: registry/install/update commands + signature verification.
- Week 4: licensing hooks + dual-runtime rollout + docs.

## Phase Exit Criteria (Go/No-Go Gates)

Week 1 gate:
- schema validator rejects invalid tasks with deterministic error codes,
- path allowlist and YAML-only scan constraints are enforced by tests.

Week 2 gate:
- pilot runtime task matches compiled-task behavior in shadow mode for at least 50 runs,
- no critical regressions in session health metrics.

Week 3 gate:
- signature verification works for signed packs,
- install/update commands are idempotent and recover from interrupted installs.

Week 4 gate:
- entitlement enforcement works for runtime tasks across Free/Pro/Team,
- docs are complete and one fresh machine can onboard end-to-end in under 15 minutes.

## Immediate Next Step for Task Handler

Create two specs before coding:
- `TASK_SCHEMA.md` (fields, step opcodes, validation rules),
- `TASK_HANDLER_ARCHITECTURE.md` (registry/loader/compiler/executor contracts).

Then create a third implementation tracker:
- `TASK_HANDLER_MVP_CHECKLIST.md` with owner, status, and evidence links per exit criterion.
