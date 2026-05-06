# CI Commands

Run these from the repo root:

```powershell
.\check-fast.ps1
```

Targeted test for counters and limits:
```powershell
cargo test --lib utils::twitter::twitteractivity_limits::tests
```
