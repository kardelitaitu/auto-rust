# Twitter Activity Task — Planning Index

**Task Name:** `twitterActivity`  
**Status:** Planning phase → Ready for implementation  
**Created:** 2026-04-17  
**Last Updated:** 2026-04-17  

---

## Quick Navigation

| Document | Description |
|----------|-------------|
| [README.md](README.md) | This index — overview and document map |
| [01-overview.md](01-overview.md) | Goals, scope, reference architecture, existing codebase assets, CLI usage |
| [02-config.md](02-config.md) | Config schema (TOML + structs), environment variables, profile extension |
| [03-agent.md](03-agent.md) | TwitterAgent state machine, per-cycle execution flow, helper methods |
| [04-modules.md](04-modules.md) | Helper module specifications (navigation, feed, dive, interact, popup, sentiment, limits, persona) |
| [05-metrics.md](05-metrics.md) | Monitoring, logging format, metrics structs, run-summary integration |
| [06-implementation.md](06-implementation.md) | Milestones, rollout plan, known gaps, references, decisions log |

---

## Document Structure

```
plan/twitterActivity/
├── README.md              # Index (you are here)
├── 01-overview.md         # Sections 0–2  (Assets, CLI, Goals, Reference)
├── 02-config.md           # Sections 3–5  (Config schema, ENV vars, Persona mapping)
├── 03-agent.md            # Section 6      (Agent state machine, cycle flow, methods)
├── 04-modules.md          # Section 7      (All helper module specs)
├── 05-metrics.md          # Section 8      (Observability, metrics, logs)
├── 06-implementation.md   # Sections 9–15  (Milestones, rollout, gaps, refs, decisions)
└── (original) twitterActivity.md  # Legacy monolith — DO NOT EDIT
```

---

## How to Use

- **Reading**: Start with `01-overview.md` for context, then jump to relevant section files.
- **Editing**: Update the individual section file(s) that logically contain the content you're modifying.
- **Cross-references**: Internal links use relative paths (e.g., `[Config Schema](../02-config.md#config-schema)`). Update them when section titles change.
- **Monolith updates**: The `twitterActivity.md` is now deprecated. If you need to regenerate it, concatenate all section files in order.

---

## Status Legend

- ✅ Completed / decided
- ⏳ In progress
- ❌ Deferred / not in scope
- 🔄 Under review

See individual files for specific status markers.
