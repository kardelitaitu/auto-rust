# Validation Checklist

- [ ] BrowserConnector covers the supported discovery/connect sources.
- [ ] SessionFactory owns session construction and capability attachment.
- [ ] SessionPoolManager handles parallel discovery and retry deterministically.
- [ ] Browser filter semantics stay stable.
- [ ] Connection timeout behavior stays stable.
- [ ] `browser.rs` is reduced to thin orchestration.
- [ ] `.
check-fast.ps1` passes during implementation.
- [ ] `.
check.ps1` passes before handoff.

# CI Commands

```powershell
.\check-fast.ps1
.\check.ps1
```

# Quality Rules

- Do not widen scope into CLI or task execution.
- Do not change discovery semantics without a test-backed reason.
- Keep connector boundaries explicit and mockable.
- Keep session assembly in one place.
- Keep the spec short and easy to review.

