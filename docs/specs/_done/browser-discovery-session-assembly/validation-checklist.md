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
