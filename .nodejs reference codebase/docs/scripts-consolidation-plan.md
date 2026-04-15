# Scripts Consolidation Plan

## Goal

Move all root-level scripts into `scripts/` directory, keeping only `setup.bat` at root as the entry point.

## Current State

### Root Scripts (to move)

| Script                      | Type    | Purpose                  |
| --------------------------- | ------- | ------------------------ |
| `backup.ps1`                | Utility | Project backup           |
| `browser-close.bat`         | Browser | Close Chrome profiles    |
| `ix-close_any_profiles.bat` | Browser | Close ixBrowser profiles |
| `ix-open.bat`               | Browser | Open ixBrowser profiles  |
| `setup-ollama.bat`          | Setup   | Install Ollama           |
| `start-docker.bat`          | Setup   | Start Docker LLM         |
| `start-ollama.bat`          | Setup   | Start Ollama server      |
| `startDashboard.bat`        | Setup   | Start Electron dashboard |
| `ui.ps1`                    | Utility | Windows UI tools         |
| `vitest-individual.ps1`     | Testing | Parallel test runner     |

### Already in scripts/

| File                              | Purpose                  |
| --------------------------------- | ------------------------ |
| `setup.js`                        | Main interactive setup   |
| `benchmark.js`                    | Performance benchmarks   |
| `generate-coverage-badge.js`      | Coverage badge generator |
| `git-amend.js`                    | Git amend helper         |
| `git-commit.js`                   | Git commit helper        |
| `ixbrowser-change-*.js`           | ixBrowser config scripts |
| `ixbrowser-proxies-pasang-tok.js` | Proxy assignment         |

### Keep at root

| File          | Reason                                       |
| ------------- | -------------------------------------------- |
| `setup.bat`   | Entry point wrapper (calls scripts/setup.js) |
| `auto-ai.bat` | PATH environment variable entry point        |

## Implementation Plan

### Phase 1: Move Scripts to scripts/

1. **Move batch files** → `scripts/windows/`
    - `browser-close.bat` → `scripts/windows/browser-close.bat`
    - `ix-close_any_profiles.bat` → `scripts/windows/ix-close_any_profiles.bat`
    - `ix-open.bat` → `scripts/windows/ix-open.bat`
    - `setup-ollama.bat` → `scripts/windows/setup-ollama.bat`
    - `start-docker.bat` → `scripts/windows/start-docker.bat`
    - `start-ollama.bat` → `scripts/windows/start-ollama.bat`
    - `startDashboard.bat` → `scripts/windows/startDashboard.bat`

2. **Move PowerShell files** → `scripts/windows/`
    - `ui.ps1` → `scripts/windows/ui.ps1`
    - `vitest-individual.ps1` → `scripts/windows/vitest-individual.ps1`
    - `backup.ps1` → `scripts/backup.ps1` (cross-platform)

### Phase 2: Update Entry Points

1. **Update `setup.bat`** → Simple wrapper

    ```batch
    @echo off
    cd /d "%~dp0"
    node scripts\setup.js %*
    ```

2. **Update `scripts/setup.js`** → Add new menu options:
    - Browser Management (close/open profiles)
    - LLM Server (start Ollama, start Docker)
    - Utilities (backup, UI tools, test runner)
    - ixBrowser Tools (change fingerprint, resolution, proxies)

### Phase 3: Update package.json Scripts

```json
{
    "scripts": {
        "start": "node main.js",
        "setup": "node scripts/setup.js",
        "backup": "powershell -ExecutionPolicy Bypass -File scripts/backup.ps1",
        "test:parallel": "powershell -ExecutionPolicy Bypass -File scripts/windows/vitest-individual.ps1",
        "browser:close": "scripts/windows/browser-close.bat",
        "ixbrowser:close": "scripts/windows/ix-close_any_profiles.bat",
        "ixbrowser:open": "scripts/windows/ix-open.bat",
        "llm:ollama": "scripts/windows/start-ollama.bat",
        "llm:docker": "scripts/windows/start-docker.bat",
        "dashboard": "scripts/windows/startDashboard.bat"
    }
}
```

### Phase 4: New Setup Menu Structure

```
Auto-AI Setup & Maintenance v1.0.0

══════════════════════════════════════════════════
 Main Setup
══════════════════════════════════════════════════
  1. Full Setup (First Time Install)
  2. Install/Update Dependencies
  3. Rebuild Native Modules
  4. Setup Git Hooks

══════════════════════════════════════════════════
 Browser Management
══════════════════════════════════════════════════
  5. Close Browser Profiles
  6. Open ixBrowser Profiles
  7. Close ixBrowser Profiles

══════════════════════════════════════════════════
 LLM Servers
══════════════════════════════════════════════════
  8. Start Ollama Server
  9. Start Docker LLM Server

══════════════════════════════════════════════════
 Tools & Utilities
══════════════════════════════════════════════════
  10. Create Backup
  11. Run Parallel Tests
  12. Start Dashboard
  13. ixBrowser Tools

══════════════════════════════════════════════════
 Diagnostics
══════════════════════════════════════════════════
  14. Diagnose Issues (Doctor)
  15. Clean Build Artifacts
  16. View Setup Log

  0. Exit
```

## File Updates Required

| File                 | Changes                             |
| -------------------- | ----------------------------------- |
| `setup.bat`          | Simplify to wrapper only            |
| `auto-ai.bat`        | Keep at root (PATH requirement)     |
| `scripts/setup.js`   | Add new menu categories and actions |
| `scripts/backup.ps1` | Remove deprecation notice           |
| `package.json`       | Update script paths                 |

## Benefits

1. **Clean root directory** - Only `setup.bat` remains
2. **Organized scripts** - All utilities in `scripts/windows/`
3. **Centralized menu** - One entry point for everything
4. **Cross-platform** - setup.js works everywhere, .bat/.ps1 only on Windows

## Timeline

- Phase 1: Move files (15 min)
- Phase 2: Update setup.js (30 min)
- Phase 3: Update package.json (5 min)
- Phase 4: Test all options (15 min)

**Total: ~1 hour**
