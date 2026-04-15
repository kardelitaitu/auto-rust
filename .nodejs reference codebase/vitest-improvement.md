# Vitest Improvement Guidelines

## Core Principles

### Efficient Token Usage for Test Improvement

1. **Analyze Before Acting**
   - Run `pnpm run test:bun:coverage 2>&1 | grep "api/"` to identify files below 85% coverage
   - Focus on files with the lowest coverage first (highest impact)
   - Use line numbers from coverage output to target specific uncovered code

2. **One File = One Agent**
   - Each test file should be owned by only ONE agent to prevent conflicts
   - AgentA works on assigned files only
   - AgentB works on assigned files only
   - Never edit a file assigned to another agent

3. **Test Writing Strategy**
   - Use existing test patterns in the codebase as templates
   - Mock external dependencies (fs, child_process, etc.) at the TOP of the file with `vi.mock()`
   - Use `vi.resetModules()` in `beforeAll` before importing the module under test
   - Keep tests focused and fast

4. **Coverage Targets**
   - Priority: Files below 80% statement coverage
   - Target: All source files above 85%
   - New files: Require 100% coverage

5. **Verification Loop**
   - Run `pnpm run test:bun:unit` after changes (fast, no coverage)
   - Run `pnpm run test:bun:coverage` only when ready to verify final coverage
   - Check for warnings like "unawaited promise" in test output

---

## Agent Assignment

### AgentA Tasks (Files: api/tests/utils + Low-hanging fruit)

| Priority | File | Current Coverage | Target | Status |
| -------- | ----- | ----------------- | ------ | ------ |
| 🔴 | `api/tests/utils/test-helpers.js` | 84.8% → 92.41% | 90% | ✅ Completed |
| 🟡 | `api/twitter/intent-follow.js` | ~60% → 95.77% | 85% | ✅ Completed |
| 🟡 | `api/twitter/intent-like.js` | ~60% → 100% | 85% | ✅ Completed |
| 🟡 | `api/twitter/intent-retweet.js` | ~60% → 96.15% | 85% | ✅ Completed |
| 🟢 | Fix unawaited promise in `memory-leaks.test.js:251` | - | - | ✅ Completed |
| 🟢 | `api/utils/ghostCursor.js` | Low → 90%+ | 85% | ✅ Completed |
| 🟢 | `api/utils/locator.js` | Low → 100% | 85% | ✅ Completed |
| 🟢 | `api/utils/sensors.js` | Low → 80%+ | 85% | ✅ Completed |

### AgentB Tasks (Files: twitterAgent.js focus)

| Priority | File | Current Coverage | Target | Status |
| -------- | ----- | ----------------- | ------ | ------ |
| 🔴 | `api/twitter/twitterAgent.js` | 23.8% → ~30% | 50% | In Progress |
| 🟡 | `api/twitter/navigation.js` | 100% | 85% | ✅ Complete |
| 🟡 | `api/twitter/session-phases.js` | 100% | 85% | ✅ Complete |

---

## Quick Reference Commands

```bash
# Run tests without coverage (fast)
pnpm run test:bun:unit

# Run tests with coverage
pnpm run test:bun:coverage

# Run specific test file
pnpm exec vitest run api/tests/unit/specific.test.js

# Check coverage for specific file
pnpm exec vitest run --coverage --coverage.include="**/twitterAgent.js"
```

## Notes

- AgentA and AgentB should coordinate via this file's status column
- Update status as work progresses
- Run lint before committing: `pnpm run lint`
