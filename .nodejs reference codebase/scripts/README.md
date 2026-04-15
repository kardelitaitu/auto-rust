# Scripts Directory

Utility scripts for development, testing, and maintenance.

---

## Quick Reference

| Script | Command | Description |
|--------|---------|-------------|
| Setup | `pnpm run setup` | Interactive setup wizard |
| Git Commit | `pnpm commit "msg"` | Commit with lint-staged |
| Git Amend | `pnpm amend "msg"` | Amend last commit |
| Coverage Badge | `pnpm run coverage:badge` | Generate coverage badge |
| Lint | `pnpm run lint` | Run ESLint |
| Format | `pnpm run format` | Run Prettier |

---

## Git Workflow Scripts

### git-commit.js

Commit changes with automatic lint-staged integration.

```bash
# Basic commit
pnpm commit "Add new feature"

# Commit and push
pnpm commit "Fix bug" --push

# Skip lint-staged
pnpm commit --no-verify "Quick fix"
```

**Features:**
- Auto-runs `lint-staged` before commit
- Generates date-based message if omitted
- Commit-only by default (no push)

---

### git-amend.js

Amend the last commit.

```bash
# Amend with new message
pnpm amend "Updated commit message"

# Amend and force push
pnpm amend "Message" --push

# Skip lint-staged
pnpm amend --no-verify "Message"
```

**Features:**
- Updates commit message
- Optionally force pushes
- Runs lint-staged by default

---

## Testing Scripts

### coverage-perfile.cjs

Generate per-file coverage report.

```bash
# Full report
node scripts/coverage-perfile.cjs

# Limit output
node scripts/coverage-perfile.cjs --limit 20
```

**Output:** Coverage percentage per file, sorted lowest to highest.

---

### generate-coverage-badge.js

Generate SVG coverage badge.

```bash
pnpm run coverage:badge
```

**Output:** `coverage-badge.svg` in project root.

---

### remove-logger-mocks.cjs

Remove logger mocks from test files (cleanup utility).

```bash
node scripts/remove-logger-mocks.cjs
```

---

### remove-math-mocks.cjs

Remove math mocks from test files (cleanup utility).

```bash
node scripts/remove-math-mocks.cjs
```

---

## Browser Management Scripts

### ixbrowser-change-fingerprint-config.js

Change fingerprint configuration for all ixBrowser profiles.

```bash
node scripts/ixbrowser-change-fingerprint-config.js
```

---

### ixbrowser-change-resolution.js

Change resolution for all ixBrowser profiles.

```bash
node scripts/ixbrowser-change-resolution.js
```

---

### ixbrowser-change-ua.js

Change user agent for ixBrowser profiles.

```bash
node scripts/ixbrowser-change-ua.js
```

---

### ixbrowser-proxies-pasang-tok.js

Configure proxies for ixBrowser profiles.

```bash
node scripts/ixbrowser-proxies-pasang-tok.js
```

---

## Utility Scripts

### benchmark.js

Run performance benchmarks.

```bash
node scripts/benchmark.js
```

**Benchmarks:**
- Query performance
- Timing functions
- Action execution speed

---

### setup.js

Interactive setup wizard for new installations.

```bash
pnpm run setup
```

**Features:**
- Environment configuration
- Dependency check
- Browser setup guidance

---

## Windows Scripts (scripts/windows/)

| Script | Command | Description |
|--------|---------|-------------|
| `setup.bat` | `.\setup.bat` | Windows setup script |
| `auto-ai.bat` | `.\auto-ai.bat` | Quick launcher |
| `browser-close.bat` | `pnpm browser:close` | Close all browsers |
| `ix-open.bat` | `pnpm ixbrowser:open` | Open ixBrowser |
| `ix-close_any_profiles.bat` | `pnpm ixbrowser:close` | Close ixBrowser profiles |
| `start-ollama.bat` | `pnpm llm:ollama` | Start Ollama server |
| `start-docker.bat` | `pnpm llm:docker` | Start Docker LLM |
| `startDashboard.bat` | `pnpm run dashboard` | Start Electron dashboard |
| `ui.ps1` | `pnpm run ui` | UI management script |
| `vitest-individual.ps1` | `pnpm run test:parallel` | Parallel test runner |

---

## npm/pnpm Scripts (package.json)

### Development

```bash
pnpm run lint              # ESLint check
pnpm run lint:fix          # Auto-fix lint issues
pnpm run format            # Prettier format
pnpm run test:bun:unit     # Unit tests (fast)
pnpm run test:bun:all      # All tests
pnpm run test:bun:coverage # With coverage
```

### Browser Management

```bash
pnpm browser:close         # Close all browsers
pnpm ixbrowser:open        # Open ixBrowser
pnpm ixbrowser:close       # Close ixBrowser profiles
```

### LLM Management

```bash
pnpm llm:ollama            # Start Ollama
pnpm llm:docker            # Start Docker LLM
pnpm llm:setup             # Setup Ollama models
```

### UI

```bash
pnpm run dashboard         # Start Electron dashboard
pnpm run ui                # UI management
```

---

## Script Development

### Creating New Scripts

1. Place in `scripts/` directory
2. Use ES modules (`import/export`)
3. Add JSDoc comments
4. Update this README

### Script Template

```javascript
#!/usr/bin/env node

/**
 * Script description
 * @example
 * node scripts/your-script.js [options]
 */

import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Your code here
```

---

## Related Documentation

- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guidelines
- [AGENTS.md](../AGENTS.md) - Agent development
- [docs/development.md](development.md) - Developer workflows
