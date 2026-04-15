# Setup.js Consolidation & API Documentation Plan

## Phase 1: Setup.js Consolidation (4 hours)

### Current State

Multiple `.bat` and `.ps1` files scattered in root:

- `setup_windows.bat` (280+ lines, full setup with checks)
- `auto-ai.bat` (launcher)
- `backup.ps1` (backup utility)
- `browser-close.bat`, `ix-close_any_profiles.bat`, `ix-open.bat`
- `setup-ollama.bat`, `start-docker.bat`, `start-ollama.bat`
- `startDashboard.bat`

### Target: Unified `setup.js`

```javascript
// setup.js - Cross-platform setup utility
// Usage: node setup.js [command]
// Commands:
//   init     - Full project setup (default)
//   deps     - Install/update dependencies
//   native   - Rebuild native modules
//   hooks    - Setup git hooks
//   backup   - Create backup
//   clean    - Clean build artifacts
//   doctor   - Diagnose issues
```

### Implementation Steps

1. **Create `scripts/setup.js`** (~200 lines)
    - Cross-platform Node.js script (no .bat dependencies)
    - Pre-flight checks (Node, pnpm, Python, disk space)
    - Dependency installation with fallback logic
    - Native module rebuilding (better-sqlite3, sharp)
    - Git hooks setup
    - Dashboard setup
    - Colored output with progress indicators

2. **Create sub-commands as separate modules** (`scripts/setup/`)
    - `init.js` - Full initialization
    - `deps.js` - Dependency management
    - `native.js` - Native module rebuild
    - `backup.js` - Backup utility (migrate from backup.ps1)
    - `doctor.js` - Diagnostic tool

3. **Update package.json scripts**

    ```json
    {
        "setup": "node scripts/setup.js",
        "setup:deps": "node scripts/setup.js deps",
        "setup:native": "node scripts/setup.js native",
        "setup:backup": "node scripts/setup.js backup",
        "setup:doctor": "node scripts/setup.js doctor"
    }
    ```

4. **Keep backward compatibility**
    - Keep `auto-ai.bat` as launcher
    - Deprecate other .bat files (add warning messages)
    - Add deprecation notice to `setup_windows.bat`

### Benefits

- Single source of truth for setup logic
- Cross-platform (works on Windows, Mac, Linux)
- Better error handling and progress reporting
- Easier to maintain and extend

---

## Phase 2: API Documentation (4 hours)

### Current State

- API exports in `api/index.js` with basic JSDoc
- Type definitions already exist (ApiOptions, ClickOptions, etc.)
- Existing docs in `api/docs/` (core.md, interactions.md, etc.)

### Target: Comprehensive API Reference

### Implementation Steps

1. **Create `docs/api-reference.md`**
    - Auto-generated from JSDoc comments
    - Organized by category:
        - Navigation (goto, back, reload)
        - Interaction (click, type, hover, scroll)
        - Waiting (wait, waitVisible, waitSelector)
        - Agent (see, do, find, vision, run)
        - Utilities (eval, config, file, screenshot)

2. **Add JSDoc examples to `api/index.js`**

    ```javascript
    /**
     * Click on an element with human-like behavior
     *
     * @param {string} selector - CSS selector or text
     * @param {ClickOptions} options - Click options
     * @returns {Promise<boolean>} Whether click succeeded
     *
     * @example
     * // Simple click
     * await api.click('#submit-button');
     *
     * @example
     * // Click with recovery
     * await api.click('.dynamic-element', {
     *   recovery: true,
     *   maxRetries: 3
     * });
     *
     * @example
     * // Click by text
     * await api.click('Sign In');
     */
    export const click = async (selector, options = {}) => { ... }
    ```

3. **Create interactive examples** (`docs/examples/`)
    - `01-basic-navigation.js`
    - `02-form-interaction.js`
    - `03-twitter-automation.js`
    - `04-agent-usage.js`
    - `05-error-handling.js`

4. **Update README.md**
    - Link to new API reference
    - Add "API Quick Reference" section

### API Categories to Document

| Category    | Methods                                | Priority |
| ----------- | -------------------------------------- | -------- |
| Navigation  | `goto`, `back`, `reload`, `getUrl`     | High     |
| Interaction | `click`, `type`, `hover`, `scroll`     | High     |
| Waiting     | `wait`, `waitVisible`, `waitSelector`  | High     |
| Agent       | `see`, `do`, `find`, `vision`, `run`   | Medium   |
| Utilities   | `eval`, `config`, `file`, `screenshot` | Medium   |
| Advanced    | `init`, `withPage`, `patch`, `plugins` | Low      |

---

## Timeline

| Day      | Task                        | Hours |
| -------- | --------------------------- | ----- |
| Day 1 AM | Setup.js core structure     | 2h    |
| Day 1 PM | Sub-commands + package.json | 2h    |
| Day 2 AM | API JSDoc examples          | 2h    |
| Day 2 PM | API reference + examples    | 2h    |

## Success Criteria

- [ ] `node setup.js` works on Windows/Mac/Linux
- [ ] All setup .bat files deprecated with warnings
- [ ] API reference documents all 50+ exported methods
- [ ] Each method has at least one usage example
- [ ] Interactive examples run without errors
