# Validation Checklist

- [x] Click and type share one shared internal interaction pipeline.
- [x] Public TaskContext verb names and signatures stay unchanged.
- [x] Click, nativeclick, focus, select_all, clear, keyboard, and type_text still work on representative selectors.
- [x] Post-action pause is applied consistently across the shared flow.
- [x] Verification behavior is explicit and testable, not hidden in duplicated method code.
- [x] `click_and_wait` still behaves as a composition of click plus wait.
- [x] `.\check-fast.ps1` passes during implementation.
- [x] `.\check.ps1` passes before handoff.
