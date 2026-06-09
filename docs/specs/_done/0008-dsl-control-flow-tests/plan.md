# Plan

## What Is the Solution

### 1. Add tests to `control_flow.rs` existing `#[cfg(test)]` module

All tests go inside the existing `mod tests { ... }` block (starts at line 417).
No new files, no new modules, no production logic changes.

### 2. Test structure

Each control flow handler gets its own test section:

**`execute_if`** — 3+ tests:
- `if_condition_true_runs_then_branch`: mock condition returns true, verify `then` actions are executed and `else` is skipped
- `if_condition_false_runs_else_branch`: mock condition returns false, verify `else` actions are executed and `then` is skipped
- `if_no_else_skips_when_false`: condition false, no else branch, verify no actions executed

**`execute_loop`** — 3+ tests:
- `loop_fixed_count_iterates_n_times`: Loop with count=5, verify inner action called 5 times
- `loop_conditional_ends_when_false`: Loop with condition that returns false on 3rd iteration, verify 2 iterations
- `loop_zero_count_skips`: Loop with count=0, verify no iterations

**`execute_foreach`** — 2+ tests:
- `foreach_iterates_all_items`: 3 items in collection, verify action called 3 times with correct variable
- `foreach_empty_collection_skips`: empty collection, verify no iterations

**`execute_retry`** — 3+ tests:
- `retry_succeeds_on_first_try`: action succeeds, verify 1 call
- `retry_retries_on_failure_up_to_max`: mock fails for 2 attempts, succeeds on 3rd, verify 3 calls
- `retry_fails_after_max_attempts`: mock always fails, verify max_attempts calls and error propagated

**`execute_parallel`** — 2+ tests:
- `parallel_executes_all_actions`: 3 actions, verify all 3 are called
- `parallel_empty_actions_does_nothing`: empty action list, verify no calls

**Helper tests** — 2+ tests:
- `evaluate_condition_element_visible`: verify condition delegation to MockDslApi.visible()
- `evaluate_condition_text_matches`: verify condition delegation to MockDslApi.text()

### 3. Implementation approach

```rust
#[tokio::test]
async fn if_condition_true_runs_then_branch() {
    let mock = MockDslApi::new();
    let def = TaskDefinition {
        name: "test".into(),
        ..Default::default()  // or minimal fields
    };
    let mut executor = DslExecutor::new(&mock, def);

    let condition = Condition::ElementExists { selector: "#btn".into() };
    let then = vec![Action::Wait { duration_ms: 10 }];
    let else_branch = None;

    // Mock the condition to return true
    mock.exists_results.lock().unwrap().insert("#btn".into(), true);
    executor.execute_if(&condition, &then, &else_branch).await.unwrap();

    let calls = mock.calls.lock().unwrap();
    assert!(calls.iter().any(|c| matches!(c, MockCall::Exists { .. })));
    // Then action was executed - verify via call log
}
```

### 4. Files changed

| File | Action |
|------|--------|
| `src/task/dsl/control_flow.rs` | **Edit** — add 15+ tests in existing `#[cfg(test)]` module |

### 5. Verification

- `cargo test --lib task::dsl` — 284+ tests pass (15 new + 269 existing)
- `cargo clippy --all-targets --all-features` — clean
- `check-fast.ps1` — passes
