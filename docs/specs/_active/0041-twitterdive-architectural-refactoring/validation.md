# Validation Checklist

- [ ] `cargo check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Verify `check_end_of_thread` doesn't exit prematurely on long threads.
- [ ] Verify `tweets_read` accurately counts unique items.
- [ ] Verify no raw `window.scrollBy` remains in the file.
- [ ] `.\check-fast.ps1` passes.
- [ ] `.\spec-lint.ps1` passes.

# CI Commands

```powershell
.\check-fast.ps1
.\spec-lint.ps1
```

# Quality Rules

- Keep imports clean (`use crate::prelude::*`).
- Rely on established `TaskContext` methods; avoid reinventing capabilities in the task file.
- Ensure all pauses have variance (no hardcoded fixed sleeps).
