# AGENTS.md

Quick reference guide for agents working on this repository.

> **Detailed guides**: See `.agents/*.md` files for deeper documentation.
> **MCP Tools**: See [`.agents/MCP-TOOLS-REFERENCE.md`](.agents/MCP-TOOLS-REFERENCE.md) for complete MCP tools documentation.

## MCP Tools Quick Reference

| Task | Primary Tools |
|------|---------------|
| Find a file | `glob`, `filesystem_search_files` |
| Search code | `grep_search`, `ctx_batch_execute` |
| Read file | `read_file`, `filesystem_read_text_file` |
| Edit file | `edit`, `filesystem_edit_file` |
| Run commands | `run_shell_command` (short output), `ctx_execute` (large output) |
| Research | `tavily_search`, `tavily_research` |
| Extract URL | `tavily_extract`, `ctx_fetch_and_index` |
| Store knowledge | `Memory_create_entities`, `Memory_create_relations` |
| Complex problems | `Sequential_Thinking_sequentialthinking` |
| Context protection | `ctx_batch_execute`, `ctx_stats` |

### Context-Mode Tools (Mandatory for Large Output)

Use context-mode tools to prevent context flooding:

- `ctx_execute` — Execute code in sandbox (only stdout enters context)
- `ctx_batch_execute` — Run multiple commands + search in ONE call
- `ctx_execute_file` — Process files without loading full content
- `ctx_fetch_and_index` — Fetch URL, index for search
- `ctx_search` — Query indexed content
- `ctx_index` — Store content in knowledge base
- `ctx_stats` — View context consumption
- `ctx_doctor` — Diagnostic health check

> **Rule**: Any command producing >20 lines output MUST use `ctx_execute` or `ctx_batch_execute`.

### Memory Tools

- `read_graph` — Read entire knowledge graph
- `search_nodes` — Search entities by query
- `create_entities` — Create new entities
- `add_observations` — Add observations to entities
- `create_relations` — Create relations between entities
- `delete_entities`, `delete_observations`, `delete_relations` — Cleanup

### Tavily Tools

- `tavily_search` — AI-powered web search
- `tavily_extract` — Extract content from URLs
- `tavily_crawl` — Deep crawl websites
- `tavily_map` — Map site structure
- `tavily_research` — Comprehensive multi-source research
- `tavily_skill` — Search library documentation

---

## Codebase Overview

**Auto-AI** is a multi-browser automation framework for orchestrating browser automation across multiple anti-detect browser profiles using Playwright's CDP (Chrome DevTools Protocol). It uses AI (local Ollama/Docker LLMs and cloud OpenRouter) for decision-making and includes human-like behavior patterns to reduce detection risk.

## Entry Points

| File            | Purpose                                                                               |
| --------------- | ------------------------------------------------------------------------------------- |
| `main.js`       | Primary automation CLI entry point for tasks like `pageview`, `follow`, and `retweet` |
| `agent-main.js` | OWB / game-agent runner for strategy automation                                       |
| `api/index.js`  | Unified API export: `import { api } from './api/index.js'`                            |

## Architecture at a Glance

- `api/index.js` is the main composition layer for context isolation, interactions, behaviors, recovery, file I/O, and agent helpers.
- `api/core/context.js` and `api/core/context-state.js` keep browser sessions isolated with `AsyncLocalStorage`.
- `api/core/orchestrator.js` owns browser discovery, task queueing, dispatch, and shutdown flow.
- `api/core/sessionManager.js` manages session lifecycle, worker health, and persistent session state.
- `api/agent/` contains the autonomous browser and OWB agent stack, including perception, reasoning, and action execution.
- `api/interactions/` contains the user-action layer for clicks, typing, scrolling, navigation, waits, and game helpers.
- `api/behaviors/` implements humanization, persona, idle, and attention behaviors.
- `api/utils/` holds shared support utilities such as config loading, logging, timing, validation, screenshots, and fingerprint helpers.
- `connectors/` contains browser discovery adapters for vendors like ixBrowser, MoreLogin, Dolphin, and related profiles.
- `tasks/` contains runnable automation scripts loaded dynamically by task name.

## Quick Commands

### Development

```bash
pnpm run lint
pnpm run lint:fix
pnpm run format
```

### Testing


test unit individual file:
pnpm exec bun run vitest run path/to/file.test.js

test coverage individual file:
pnpm exec bun run vitest run --coverage path/to/file.test.js

**Preferred (Bun via pnpm):**

```bash
pnpm run test:bun:unit
pnpm run test:bun:integration
pnpm run test:bun:edge
pnpm run test:bun:all
pnpm run test:bun:coverage
pnpm run test:bun:watch
pnpm run test:bun:verbose
```

**Alternative (Node.js via pnpm):**

```bash
pnpm run test:unit
pnpm run test:integration
pnpm run test:edge-cases
pnpm run test:smoke
pnpm run test:all
pnpm run test:coverage
pnpm run test:ci
pnpm run test:watch
pnpm run test:verbose
```

> **Performance**: Bun runs tests faster than Node.js. Use the `pnpm run test:bun:*` variants for development, and the `pnpm run test:*` variants when Bun is unavailable or when Node compatibility matters.

### Running

```bash
# Automation tasks
node main.js taskName=url
node main.js pageview=example.com then twitterFollow=url

# Strategy game agent
node agent-main.js owb
node agent-main.js owb play --loops=10
node agent-main.js owb play=rush
node agent-main.js owb state-a x20
```

### Git Workflow

```bash
pnpm commit "message"              # Commit only (no push)
pnpm commit "message" --push       # Commit + push
pnpm commit --no-verify "message" # Skip lint-staged
pnpm amend "updated message"       # Amend only (no push)
pnpm amend "updated message" --push # Amend + force push
pnpm amend --no-verify             # Skip lint-staged
```

> `pnpm commit` auto-generates a date-based message when you omit one, and both helpers run `pnpm exec lint-staged` by default. Push is no longer automatic - use `--push` to push after committing.

### Agent Modes (`agent-main.js`)

| Mode       | Description               |
| ---------- | ------------------------- |
| `play`     | Auto-play mode            |
| `rush`     | Fast attack strategy      |
| `turtle`   | Defensive strategy        |
| `economy`  | Resource-focused          |
| `balanced` | Mixed strategy            |
| `build`    | Construction focus        |
| `train`    | Unit training focus       |
| `attack`   | Aggressive combat         |
| `gather`   | Resource collection       |
| `state-*`  | Run a specific game state |

### Test Audit Runner

```powershell
.\vitest-individual.ps1
```

> Scans `api/**/*.test.js`, runs tests in parallel batches, and writes `vitest-individual.txt` for long-form audit runs.

## Working Conventions

- Use `pnpm` for all install, lint, format, and test commands.
- Keep console output prefixed with the script name when logging from code.
- Prefer `api.*` methods over raw Playwright `page.*` calls for humanized interactions.
- Use `api.withPage(...)` for session isolation and avoid leaking page state globally.
- Follow the task-module pattern documented in the deeper API docs when adding new tasks.
- **Always update `AGENT-JOURNAL.md`** after making changes. Use this format:

    ```
    DD-MM-YYYY--HH:MM > File(s) > Description of changes
    ```

    - Entry goes at the TOP (before existing entries)
    - Use past tense for completed work
    - Be concise but specific about what was changed
    - For larger releases, also update `patchnotes.md`

- **Never run `git push` unless explicitly asked by the user.** Always commit only by default.
- Keep branches small and focused, and prefer PR-based merges for shared work.

---

## Workflow Reminders

### Pre-Commit Checklist

```
☐ Lint passes:        pnpm run lint
☐ Tests pass:         pnpm run test:bun:unit (or test:bun:all)
☐ Coverage acceptable: pnpm run test:bun:coverage (if touching core)
☐ Format applied:     pnpm run format
☐ Journal updated:    AGENT-JOURNAL.md (if exists)
```

### Post-Change Actions

| Change Type | Required Actions |
|-------------|------------------|
| Bug fix | Update `AGENT-JOURNAL.md`, add/fix tests |
| New feature | Update `AGENT-JOURNAL.md`, add tests, update `patchnotes.md` |
| Refactor | Update `AGENT-JOURNAL.md`, verify tests still pass |
| Config change | Update `AGENT-JOURNAL.md`, document in PR |
| Documentation | Update `AGENT-JOURNAL.md` (optional for minor docs) |

### Git Workflow Quick Reference

```bash
# Stage and commit (commit-only, no push)
pnpm commit "message"

# Commit with push
pnpm commit "message" --push

# Skip lint-staged
pnpm commit --no-verify "message"

# Amend last commit
pnpm amend "updated message"

# Amend and force push
pnpm amend "updated message" --push
```

### Agent Journal Format

If `AGENT-JOURNAL.md` exists, add entries at the TOP:

```markdown
31-03-2026--14:30 > AGENTS.md, README.md > Improved documentation with MCP tools reference and troubleshooting section
30-03-2026--09:15 > api/core/context.js > Fixed AsyncLocalStorage leak in withPage()
```

---

## Testing

### Test Structure

```
api/tests/
├── unit/                    # Unit tests (isolated modules)
│   ├── api/                 # API module tests
│   ├── agent/               # Agent system tests
│   └── ...
├── integration/             # Integration tests (cross-module)
├── edge-cases/              # Edge case scenarios
└── *.test.js                # Root-level tests
```

### Test Commands

| Command | Description |
|---------|-------------|
| `pnpm run test:bun:unit` | Unit tests only (fast) |
| `pnpm run test:bun:integration` | Integration tests only |
| `pnpm run test:bun:edge` | Edge case tests |
| `pnpm run test:bun:all` | All tests |
| `pnpm run test:bun:coverage` | With coverage report |
| `pnpm run test:bun:verbose` | Detailed output |
| `pnpm run test:bun:watch` | Watch mode (dev) |

> **Performance**: Bun runs tests 3-5x faster than Node.js. Use `pnpm run test:bun:*` for development.

### Test File Naming

- **Pattern**: `*.test.js` (e.g., `context.test.js`, `actionEngine.test.js`)
- **Location**: Mirror source structure under `api/tests/unit/` or `api/tests/integration/`
- **Example**: `api/core/context.js` → `api/tests/unit/api/context.test.js`

### Coverage

- **Target**: >90% line coverage for core modules
- **Badge**: Run `pnpm run coverage:full` to generate/update `coverage-badge.svg`
- **Report**: Open `api/coverage/index.html` for detailed HTML report
- **Pre-commit**: Run `pnpm run test:bun:coverage` when touching core modules or tests

### Vitest Configuration

- **Pool**: `pool: 'threads'` (required for AsyncLocalStorage isolation)
- **Setup**: `vitest.setup.js` creates coverage directories
- **Config**: `config/vitest.config.js`

### Common Test Patterns

```javascript
import { describe, it, expect, vi, beforeEach } from 'vitest';

// Top-level mocks (BEFORE any imports)
vi.mock('../core/logger.js', () => ({
    createLogger: () => ({
        info: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    }),
}));

describe('myModule', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    it('should do something', async () => {
        // Use api.withPage() for browser tests
        await api.withPage(mockPage, async () => {
            await api.init(mockPage);
            await api.click('.btn');
            expect(mockPage.click).toHaveBeenCalled();
        });
    });
});
```

**Key Patterns:**
- Mock `api/core/logger.js` and core dependencies with top-level `vi.mock()`
- Exercise `api.withPage()` blocks instead of calling raw `page.*`
- Use isolated fixtures for agent and interaction modules

### CI Test Matrix

CI runs on push/PR:
```bash
pnpm run lint
pnpm run test:bun:unit
pnpm run test:bun:integration
pnpm run test:bun:edge
```

## Task System & Configuration

- Tasks are loaded dynamically from `tasks/` by task name.
- Task modules should export a default async function: `async function(page, payload)`.
- Task payloads generally include task-specific parameters plus browser/session context.
- Supported browsers include anti-detect vendors plus local Chrome/Brave/Edge/Vivaldi profiles.
- Configuration is layered:
    - `config/settings.json` for LLM, humanization, and persona settings
    - `config/browserAPI.json` for browser vendor ports
    - `config/timeouts.json` for timeout values
    - `.env` for runtime environment variables
- Humanization features include mouse movement, keystroke dynamics, scrolling patterns, idle behavior, PID-style movement tuning, and sensor noise spoofing.

## What To Inspect First

Start with these hotspots when learning or changing behavior:

- `api/core/orchestrator.js`
- `api/core/sessionManager.js`
- `api/core/context.js`
- `api/agent/`
- `connectors/`
- `tasks/`

## Further Reading

| Document | Description |
|----------|-------------|
| [`.agents/PROJECT-STRUCTURE.md`](.agents/PROJECT-STRUCTURE.md) | Directory structure and module responsibilities |
| [`.agents/API-ARCHITECTURE.md`](.agents/API-ARCHITECTURE.md) | API patterns, context isolation, agent loop |
| [`.agents/TESTING-GUIDE.md`](.agents/TESTING-GUIDE.md) | Vitest testing strategy and patterns |
| [`.agents/TASK-AND-CONFIG.md`](.agents/TASK-AND-CONFIG.md) | Task system and configuration layers |
| [`.agents/TECH-STACK.md`](.agents/TECH-STACK.md) | Technology stack and dependencies |
| [`.agents/STEALTH-PROTOCOL.md`](.agents/STEALTH-PROTOCOL.md) | Anti-detection and humanization |
| [`.agents/MCP-TOOLS-REFERENCE.md`](.agents/MCP-TOOLS-REFERENCE.md) | Complete MCP tools documentation |
| [`README.md`](README.md) | Project overview and quick start |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Contribution guidelines |

### Configuration References

- See [`.agents/TECH-STACK.md`](.agents/TECH-STACK.md) for scroll multiplier configuration
- See [`.agents/TASK-AND-CONFIG.md`](.agents/TASK-AND-CONFIG.md) for browser port mappings
- See [`.agents/API-ARCHITECTURE.md`](.agents/API-ARCHITECTURE.md) for GhostCursor implementation details
- See [`.agents/STEALTH-PROTOCOL.md`](.agents/STEALTH-PROTOCOL.md) for stealth protocol overview

## Verification Log

| Entry                                                                                                                 | Evidence                                                                                  |
| --------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Git workflow helpers (`pnpm commit`, `pnpm amend`, `pnpm exec lint-staged`, commit-only default)                      | `package.json`, `scripts/git-commit.js`, `scripts/git-amend.js`, commit `035664c`         |
| Parallel Vitest audit runner (`.\vitest-individual.ps1`)                                                              | `vitest-individual.ps1`, commits `9e8e4a8` and `a9a1919`                                  |
| CI test matrix (`pnpm run lint`, `pnpm run test:bun:unit`, `pnpm run test:bun:integration`, `pnpm run test:bun:edge`) | `.github/workflows/ci.yml`, commits `938dd2f`, `37d7e58`, `fc172bc`, `1d6fd25`, `5d4544a` |
| Test mocking standards (top-level vi.mock)                                                                            | `AGENTS.md`, commit `87abc3b`                                                             |

---

## Quick Troubleshooting

### Browser Connection Issues

| Symptom | Check | Fix |
|---------|-------|-----|
| Cannot connect to browser | Remote debugging enabled? | Launch browser with `--remote-debugging-port=9222` |
| Port already in use | `netstat -an \| findstr "9222"` | `npx kill-port 9222` |
| WebSocket connection failed | CDP endpoint correct? | Check `ws` URL from browser discovery |

### LLM Connection Issues

| Symptom | Check | Fix |
|---------|-------|-----|
| Ollama not responding | `curl http://localhost:11434/api/tags` | Start Ollama: `ollama serve` |
| OpenRouter 401 | API key valid? | Verify `OPENROUTER_API_KEY` in `.env` |
| Model not found | Model pulled? | `ollama pull <model-name>` |

### Test Failures

| Symptom | Check | Fix |
|---------|-------|-----|
| Tests fail randomly | AsyncLocalStorage isolation? | Ensure `pool: 'threads'` in vitest config |
| Coverage not generating | Coverage dir exists? | Run `pnpm run test:coverage` |
| Mocks not working | Top-level `vi.mock()`? | Move mocks to top of file, before imports |

### Quick Diagnostic Commands

```bash
# Check port usage
netstat -an | findstr "9222 18800"

# Kill process on port
npx kill-port 9222

# Test Ollama
curl http://localhost:11434/api/tags

# Test OpenRouter
curl -H "Authorization: Bearer $OPENROUTER_API_KEY" https://openrouter.ai/api/v1/models

# Run quick tests
pnpm run test:bun:unit
```

---

# context-mode — MANDATORY routing rules

You have context-mode MCP tools available. These rules are NOT optional — they protect your context window from flooding. A single unrouted command can dump 56 KB into context and waste the entire session.

## BLOCKED commands — do NOT attempt these

### curl / wget — BLOCKED

Any shell command containing `curl` or `wget` will be intercepted and blocked by the context-mode plugin. Do NOT retry.
Instead use:

- `context-mode_ctx_fetch_and_index(url, source)` to fetch and index web pages
- `context-mode_ctx_execute(language: "javascript", code: "const r = await fetch(...)")` to run HTTP calls in sandbox

### Inline HTTP — BLOCKED

Any shell command containing `fetch('http`, `requests.get(`, `requests.post(`, `http.get(`, or `http.request(` will be intercepted and blocked. Do NOT retry with shell.
Instead use:

- `context-mode_ctx_execute(language, code)` to run HTTP calls in sandbox — only stdout enters context

### Direct web fetching — BLOCKED

Do NOT use any direct URL fetching tool. Use the sandbox equivalent.
Instead use:

- `context-mode_ctx_fetch_and_index(url, source)` then `context-mode_ctx_search(queries)` to query the indexed content

## REDIRECTED tools — use sandbox equivalents

### Shell (>20 lines output)

Shell is ONLY for: `git`, `mkdir`, `rm`, `mv`, `cd`, `ls`, `npm install`, `pip install`, and other short-output commands.
For everything else, use:

- `context-mode_ctx_batch_execute(commands, queries)` — run multiple commands + search in ONE call
- `context-mode_ctx_execute(language: "shell", code: "...")` — run in sandbox, only stdout enters context

### File reading (for analysis)

If you are reading a file to **edit** it → reading is correct (edit needs content in context).
If you are reading to **analyze, explore, or summarize** → use `context-mode_ctx_execute_file(path, language, code)` instead. Only your printed summary enters context.

### grep / search (large results)

Search results can flood context. Use `context-mode_ctx_execute(language: "shell", code: "grep ...")` to run searches in sandbox. Only your printed summary enters context.

## Tool Selection Hierarchy

### Primary Workflow

1. **GATHER**: `ctx_batch_execute(commands, queries)` — Run multiple commands + search in ONE call. Replaces 30+ individual calls.
2. **FOLLOW-UP**: `ctx_search(queries: ["q1", "q2", ...])` — Query indexed content. Pass ALL questions as array in ONE call.
3. **PROCESSING**: `ctx_execute(language, code)` or `ctx_execute_file(path, language, code)` — Sandbox execution. Only stdout enters context.
4. **WEB**: `ctx_fetch_and_index(url, source)` then `ctx_search(queries)` — Fetch, chunk, index, query. Raw HTML never enters context.
5. **INDEX**: `ctx_index(content, source)` — Store content in FTS5 knowledge base for later search.

### File Operations

| Task | Tool | Notes |
|------|------|-------|
| Find files by pattern | `glob(pattern)` | Fast pattern matching |
| Search file contents | `grep_search(pattern, path)` | ripgrep-based, fast |
| List directory | `list_directory(path)` | With ignore options |
| Read to edit | `read_file(path)` | Use when you need content for editing |
| Read to analyze | `ctx_execute_file(path, language, code)` | Summary only, no content flood |
| Edit file | `edit(file_path, old_string, new_string)` | Surgical changes |
| Write file | `write_file(file_path, content)` | New files or full rewrites |

### Code Analysis

| Task | Tool | Notes |
|------|------|-------|
| Simple search | `grep_search(pattern)` | Use for keyword/regex search |
| AST parsing | Use `ctx_execute` with tree-sitter | For structure analysis |
| Find definitions | `grep_search("class Foo", "function bar")` | Quick location |

> **Rule**: Use `grep_search` for simple patterns. Use tree-sitter via `ctx_execute` for complex AST analysis.

## Output constraints

- Keep responses under 500 words.
- Write artifacts (code, configs, PRDs) to FILES — never return them as inline text. Return only: file path + 1-line description.
- When indexing content, use descriptive source labels so others can `search(source: "label")` later.

## ctx commands

| Command       | Action                                                                            |
| ------------- | --------------------------------------------------------------------------------- |
| `ctx stats`   | Call the `stats` MCP tool and display the full output verbatim                    |
| `ctx doctor`  | Call the `doctor` MCP tool, run the returned shell command, display as checklist  |
| `ctx upgrade` | Call the `upgrade` MCP tool, run the returned shell command, display as checklist |
