## Validation

- Deterministic test added for the `execute_group_with_cancel` timeout branch.
- Deterministic test added for the `execute_group_with_cancel` shutdown-cancel branch.
- Deterministic test added for task cancellation before worker acquisition.
- Result-aggregation invariant verified: `[Ok(()), Ok(()), Err(..)] -> success_count=2, fail_count=1`, `[Err(..), Err(..)] -> success_count=0`, `[Ok(()), Ok(()), Ok(())] -> success_count=3`.
- `spec-lint.ps1` passed before handoff.
