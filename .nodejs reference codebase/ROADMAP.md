# Auto-AI Development Roadmap

> Last updated: 2026-03-29

## Current Focus

Phase 3: Features

---

## Phase 1: Quick Wins (Week 1) ✅ Complete

| Priority | Task                                           | Impact | Effort | Status               |
| -------- | ---------------------------------------------- | ------ | ------ | -------------------- |
| 🔴       | Fix TODO comments in EngagementHandler.js      | Medium | 1h     | ✅ Done (2026-03-28) |
| 🔴       | Enable ESLint for `api/tests/` directory       | Medium | 2h     | ✅ Done (2026-03-28) |
| 🟡       | Add coverage badge to README                   | Low    | 1h     | ✅ Done (2026-03-28) |
| 🟡       | Improve README (TOC, config, troubleshooting)  | Medium | 30m    | ✅ Done (2026-03-28) |
| 🟡       | Improve test coverage (timeout constants)      | Low    | 30m    | ✅ Done (2026-03-28) |
| 🟡       | Create V8 coverage config                      | Low    | 15m    | ✅ Done (2026-03-28) |
| 🟡       | Update patchnotes.md                           | Low    | 10m    | ✅ Done (2026-03-28) |
| 🟡       | Create unified `setup.bat` (interactive menu)  | Medium | 4h     | ✅ Done (2026-03-28) |
| 🟡       | Consolidate root scripts into scripts/windows/ | Medium | 1h     | ✅ Done (2026-03-28) |
| 🟡       | Document API exports with JSDoc examples       | High   | 4h     | ✅ Done (2026-03-28) |

**Exit criteria**: 0 TODO comments, 100% source linting, API docs complete.

---

## Phase 2: Quality & Reliability (Week 2-3) ✅ Complete

| Priority | Task                                               | Impact | Effort | Status               |
| -------- | -------------------------------------------------- | ------ | ------ | -------------------- |
| 🔴       | Standardize error handling in `api/core/errors.js` | High   | 6h     | ✅ Done (2026-03-28) |
| 🔴       | Add performance benchmarks for critical paths      | High   | 8h     | ✅ Done (2026-03-28) |
| 🟡       | Enhance config validation & error messages         | Medium | 4h     | ✅ Done (2026-03-28) |
| 🟡       | Create developer onboarding guide                  | Medium | 4h     | ✅ Done (2026-03-28) |

**Exit criteria**: Consistent error patterns, baseline benchmarks documented.

**Completed**:

- Standardized error handling with new signature (code, message, metadata, cause)
- Added error codes and suggestions to ConfigValidator
- Added LLM config schema validation
- Created 11 performance benchmarks for queries, timing, and actions
- Created `docs/development.md` for internal developer workflows

---

## Phase 3: Features (Week 4-6)

| Priority | Task                                    | Impact | Effort | Status     |
| -------- | --------------------------------------- | ------ | ------ | ---------- |
| 🔴       | Real-time metrics dashboard (web UI)    | High   | 20h    | 📋 Planned |
| 🔴       | Circuit breaker for browser connections | High   | 12h    | 📋 Planned |
| 🟡       | Exponential backoff retry logic         | Medium | 10h    | 📋 Planned |
| 🟡       | Fix 5 flaky tests in coverage run       | Medium | 2h     | ✅ Done (2026-03-29) |
| 🟢       | Plugin system for custom tasks          | Medium | 16h    | 📋 Planned |

**Exit criteria**: Live monitoring, improved reliability metrics (>95% connection success).

---

## Phase 4: Quality & Additional Improvements

| Priority | Task                                      | Impact | Effort | Status     |
| -------- | ----------------------------------------- | ------ | ------ | ---------- |
| 🟡       | Add more integration tests for connectors | Medium | 4h     | 📋 Planned |
| 🟡       | CI workflow improvements                  | Medium | 4h     | 📋 Planned |
| 🟢       | Prettier project config                   | Low    | 2h     | 📋 Planned |

---

## Phase 5: Strategic (Month 2+)

| Priority | Task                                   | Impact | Effort | Status     |
| -------- | -------------------------------------- | ------ | ------ | ---------- |
| 🟡       | Distributed cross-platform testing     | High   | 30h    | 📋 Planned |
| 🟢       | AI model performance tracking          | Medium | 24h    | 📋 Planned |
| 🟢       | Comprehensive user guide               | Medium | 32h    | 📋 Planned |
| 🔴       | TypeScript definitions (`.d.ts` files) | High   | 40h    | 📋 Planned |

**Exit criteria**: Full IDE support, automated cross-platform CI.

---

## Success Metrics

| Metric             | Current | Target              |
| ------------------ | ------- | ------------------- |
| ESLint coverage    | 100%    | 100% source files   |
| Test coverage      | 79.7%   | >80% critical paths |
| TODO comments      | 0       | <5 intentional      |
| Connection success | ~90%    | >95%                |
| Setup time         | ~5 min  | <10 min             |

---

## Tech Debt Log

- [x] Semaphore race condition - Added Promise lock for atomic acquire operations
- [x] Config loader standardization - ConfigLoader is now single source of truth
- [x] CI workflow - Replaced self-hosted with ubuntu-latest runners
- [x] TODO in `api/twitter/twitter-agent/EngagementHandler.js:42` - Implemented `interactWithProfile()` method
- [x] ESLint ignores `api/tests/` - Removed from ignores, fixed 29 errors across test files
- [x] No coverage badge in README - Added auto-generated SVG badge (79.7% coverage)
- [x] Multiple `.bat` setup scripts (consolidated) - Moved to `scripts/windows/`, only `setup.bat` + `auto-ai.bat` at root
- [x] README improvements - Added TOC, Configuration, Troubleshooting sections
- [x] Missing API documentation - Added JSDoc @example blocks to 15+ methods
- [x] No performance benchmarks - Created 11 benchmarks in `api/tests/benchmarks/`
- [x] Config validation lacks error codes - Added error code mapping and suggestions
- [x] No developer onboarding guide - Created `docs/development.md`

---

## Links

- Detailed plan: [`docs/improvement-plan.md`](docs/improvement-plan.md)
- Agent conventions: [`AGENTS.md`](AGENTS.md)
- Changelog: [`patchnotes.md`](patchnotes.md)
