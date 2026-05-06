# Validation Checklist

- [x] Task-group parsing behavior stays stable after parser extraction.
- [x] Browser filter parsing behavior stays stable after parser extraction.
- [x] Task validation uses task names and registry metadata.
- [x] `--help-task` returns task-specific payload guidance.
- [x] Existing startup modes continue to work.
- [x] `./check-fast.ps1` passes during implementation.
- [x] `./check.ps1` passes before handoff.

# CI Commands

```powershell
.\check-fast.ps1
.\check.ps1
```

# Quality Rules

- Do not break current task syntax.
- Do not duplicate validation logic in the CLI layer.
- Keep help output concise and stable.
- Keep parser code separate from task execution code.
- Keep the spec short and easy to review.

