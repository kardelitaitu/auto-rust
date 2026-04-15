# Agent 2 Spec: Browser Discovery, Circuit Breaker, Session Lifecycle

## Scope
Own browser connection/discovery and session worker mechanics.

## Files
- `src/browser.rs`
- `src/session.rs`
- relevant config handling in `src/config.rs` (only fields used by browser/session)

## Functional Requirements
- Discover configured profiles with retry loop.
- Apply connection timeout.
- Track failures and successes via circuit breaker.
- Block connection attempts when breaker is open.
- Session worker acquisition with timeout.
- Page acquisition/release path with close timeout protection.

## Reliability Requirements
- No panic when no browser is available.
- Graceful partial success if only some browsers connect.
- Graceful shutdown closes connected sessions.

## Acceptance Checks
- Discovery retries follow configured counts/delays.
- Worker acquisition timeout path returns failure cleanly.
- Session close path logs warning on close timeout but continues.

## Out Of Scope
- Parser grammar
- Task business logic

## Report Format
- Connection paths validated
- Timeout/retry behavior validated
- Remaining failure modes
