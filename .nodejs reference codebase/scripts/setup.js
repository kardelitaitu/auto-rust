#!/usr/bin/env node

/**
 * Auto-AI Interactive Setup
 *
 * Cross-platform setup utility with arrow key navigation.
 *
 * Usage:
 *   node scripts/setup.js
 *   pnpm run setup
 *
 * Navigation:
 *   ↑/↓ Arrow keys - Navigate menu
 *   Enter - Select option
 *   Esc/Q - Exit
 *   0-9 - Quick select by number
 */

import { execSync, spawn } from "child_process";
import { existsSync, readFileSync, writeFileSync, mkdirSync, rmSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";
import { createInterface } from "readline";

const __dirname = dirname(fileURLToPath(import.meta.url));
const rootDir = join(__dirname, "..");
const LOG_FILE = join(rootDir, "setup_log.txt");

// ============================================================================
// Menu Options (with categories)
// ============================================================================
const menuOptions = [
  // Main Setup
  {
    id: 1,
    label: "Full Setup (First Time Install)",
    action: "fullSetup",
    category: "Main Setup",
  },
  {
    id: 2,
    label: "Install/Update Dependencies",
    action: "installDeps",
    category: "Main Setup",
  },
  {
    id: 3,
    label: "Rebuild Native Modules",
    action: "rebuildNative",
    category: "Main Setup",
  },
  {
    id: 4,
    label: "Setup Git Hooks",
    action: "setupHooks",
    category: "Main Setup",
  },

  // Browser Management
  {
    id: 5,
    label: "Close Browser Profiles",
    action: "closeBrowsers",
    category: "Browser Management",
  },
  {
    id: 6,
    label: "Open ixBrowser Profiles",
    action: "openIxProfiles",
    category: "Browser Management",
  },
  {
    id: 7,
    label: "Close ixBrowser Profiles",
    action: "closeIxProfiles",
    category: "Browser Management",
  },

  // LLM Servers
  {
    id: 8,
    label: "Start Ollama Server",
    action: "startOllama",
    category: "LLM Servers",
  },
  {
    id: 9,
    label: "Start Docker LLM Server",
    action: "startDocker",
    category: "LLM Servers",
  },
  {
    id: 10,
    label: "Setup Ollama",
    action: "setupOllama",
    category: "LLM Servers",
  },

  // Tools & Utilities
  {
    id: 11,
    label: "Create Backup",
    action: "backup",
    category: "Tools & Utilities",
  },
  {
    id: 12,
    label: "Run Parallel Tests",
    action: "runParallelTests",
    category: "Tools & Utilities",
  },
  {
    id: 13,
    label: "Start Dashboard",
    action: "setupDashboard",
    category: "Tools & Utilities",
  },
  {
    id: 14,
    label: "Open UI Tools",
    action: "openUiTools",
    category: "Tools & Utilities",
  },

  // Diagnostics
  {
    id: 15,
    label: "Diagnose Issues (Doctor)",
    action: "doctor",
    category: "Diagnostics",
  },
  {
    id: 16,
    label: "Clean Build Artifacts",
    action: "clean",
    category: "Diagnostics",
  },
  {
    id: 17,
    label: "View Setup Log",
    action: "viewLog",
    category: "Diagnostics",
  },

  // Exit
  { id: 0, label: "Exit", action: "exit" },
];

// ============================================================================
// Terminal Colors
// ============================================================================
const C = {
  reset: "\x1b[0m",
  bright: "\x1b[1m",
  dim: "\x1b[2m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  red: "\x1b[31m",
  cyan: "\x1b[36m",
  white: "\x1b[37m",
  gray: "\x1b[90m",
};

function clear() {
  process.stdout.write("\x1b[2J\x1b[H");
}

function moveTo(row, col) {
  process.stdout.write(`\x1b[${row};${col}H`);
}

function hideCursor() {
  process.stdout.write("\x1b[?25l");
}

function showCursor() {
  process.stdout.write("\x1b[?25h");
}

// ============================================================================
// Check if terminal supports raw mode (arrow keys)
// ============================================================================
function supportsRawMode() {
  return process.stdin.isTTY && typeof process.stdin.setRawMode === "function";
}

// ============================================================================
// Menu Display
// ============================================================================
function drawMenu(selectedIndex) {
  clear();

  let row = 1;

  // Header (compact)
  moveTo(row++, 1);
  process.stdout.write(`${C.cyan}${C.bright} Auto-AI Setup v1.0.0${C.reset}`);

  // Instructions
  moveTo(row++, 1);
  if (supportsRawMode()) {
    process.stdout.write(
      `${C.gray} ${C.white}↑↓${C.gray} navigate  ${C.white}Enter${C.gray} select  ${C.white}Esc/Q${C.gray} exit${C.reset}`,
    );
  } else {
    process.stdout.write(
      `${C.gray} ${C.white}Number${C.gray} select  ${C.white}0/Q${C.gray} exit${C.reset}`,
    );
  }

  // Menu items with categories
  let currentCategory = null;

  for (let i = 0; i < menuOptions.length; i++) {
    const option = menuOptions[i];

    // Draw compact category header
    if (option.category && option.category !== currentCategory) {
      currentCategory = option.category;
      moveTo(row++, 1);
      process.stdout.write(`${C.cyan}── ${currentCategory} ──${C.reset}`);
    }

    moveTo(row++, 1);
    if (i === selectedIndex) {
      process.stdout.write(
        ` ${C.green}${C.bright}>${C.reset} ${String(option.id).padStart(2)} ${option.label}`,
      );
    } else if (option.id === 0) {
      process.stdout.write(
        `   ${C.gray}${String(option.id).padStart(2)} ${option.label}${C.reset}`,
      );
    } else {
      process.stdout.write(
        `   ${C.white}${String(option.id).padStart(2)} ${option.label}${C.reset}`,
      );
    }
  }

  // Position cursor at bottom
  moveTo(row + 1, 1);
}

// ============================================================================
// Get Key Press
// ============================================================================
function getKey() {
  return new Promise((resolve) => {
    const { stdin } = process;

    // If not a TTY, fall back to simple line input
    if (!supportsRawMode()) {
      const rl = createInterface({
        input: process.stdin,
        output: process.stdout,
      });
      rl.question("", (answer) => {
        rl.close();
        resolve(answer.trim());
      });
      return;
    }

    let escapeTimer = null;

    // Set up raw mode
    stdin.setRawMode(true);
    stdin.resume();
    stdin.setEncoding("utf8");

    const cleanup = () => {
      if (escapeTimer) clearTimeout(escapeTimer);
      stdin.removeListener("data", handler);
      stdin.setRawMode(false);
      stdin.pause();
    };

    const handler = (chunk) => {
      // Escape key prefix
      if (chunk === "\x1b") {
        if (escapeTimer) clearTimeout(escapeTimer);
        escapeTimer = setTimeout(() => {
          cleanup();
          resolve("\x1b");
        }, 50);
        return;
      }

      if (escapeTimer) {
        clearTimeout(escapeTimer);
        escapeTimer = null;
      }

      if (chunk.length === 1) {
        const code = chunk.charCodeAt(0);

        if (code === 13 || code === 10) {
          cleanup();
          resolve("enter");
          return;
        }
        if (code === 3) {
          cleanup();
          resolve("ctrl+c");
          return;
        }
        if (chunk === "q" || chunk === "Q") {
          cleanup();
          resolve(chunk);
          return;
        }
        if (chunk >= "0" && chunk <= "9") {
          cleanup();
          resolve(chunk);
          return;
        }
      }

      cleanup();
      resolve(chunk);
    };

    stdin.on("data", handler);
  });
}

// ============================================================================
// Menu Loop
// ============================================================================
async function showMenu() {
  let selectedIndex = 0;

  hideCursor();

  while (true) {
    drawMenu(selectedIndex);
    const key = await getKey();

    // Handle non-TTY input (number selection)
    if (!supportsRawMode()) {
      if (key === "" || key === "0" || key.toLowerCase() === "q") {
        showCursor();
        return menuOptions.find((o) => o.id === 0);
      }
      const num = parseInt(key);
      if (!isNaN(num)) {
        const index = menuOptions.findIndex((o) => o.id === num);
        if (index !== -1) {
          showCursor();
          return menuOptions[index];
        }
      }
      continue;
    }

    // Handle arrow keys and special keys (TTY mode)
    if (key === "\x1b[A" || key === "\x1bOA") {
      selectedIndex =
        selectedIndex > 0 ? selectedIndex - 1 : menuOptions.length - 1;
    } else if (key === "\x1b[B" || key === "\x1bOB") {
      selectedIndex =
        selectedIndex < menuOptions.length - 1 ? selectedIndex + 1 : 0;
    } else if (key === "enter") {
      showCursor();
      return menuOptions[selectedIndex];
    } else if (key === "\x1b") {
      showCursor();
      return menuOptions.find((o) => o.id === 0);
    } else if (key === "q" || key === "Q") {
      showCursor();
      return menuOptions.find((o) => o.id === 0);
    } else if (key === "ctrl+c") {
      showCursor();
      process.exit(0);
    } else if (key >= "0" && key <= "9") {
      const num = parseInt(key);
      const index = menuOptions.findIndex((o) => o.id === num);
      if (index !== -1) {
        selectedIndex = index;
      }
    }
  }
}

// ============================================================================
// Prompt Helpers
// ============================================================================
function pause() {
  return new Promise((resolve) => {
    showCursor();
    const rl = createInterface({
      input: process.stdin,
      output: process.stdout,
    });
    rl.question("\nPress Enter to continue...", () => {
      rl.close();
      hideCursor();
      resolve();
    });
  });
}

function confirm(message) {
  return new Promise((resolve) => {
    showCursor();
    const rl = createInterface({
      input: process.stdin,
      output: process.stdout,
    });
    rl.question(`${message} (Y/N): `, (answer) => {
      rl.close();
      hideCursor();
      resolve(answer.toLowerCase() === "y" || answer.toLowerCase() === "yes");
    });
  });
}

// ============================================================================
// Logging
// ============================================================================
function writeLog(message) {
  try {
    const timestamp = new Date().toISOString();
    const logLine = `[${timestamp}] ${message}\n`;
    const existing = existsSync(LOG_FILE) ? readFileSync(LOG_FILE, "utf8") : "";
    writeFileSync(LOG_FILE, existing + logLine);
  } catch (e) {
    /* silently ignore logging errors */
  }
}

function log(msg, color = "white") {
  console.log(`${C[color]}${msg}${C.reset}`);
}

function success(msg) {
  log(`  [OK] ${msg}`, "green");
}
function warn(msg) {
  log(`  [WARN] ${msg}`, "yellow");
}
function error(msg) {
  log(`  [ERROR] ${msg}`, "red");
}
function info(msg) {
  log(`  [INFO] ${msg}`, "cyan");
}

// ============================================================================
// Command Execution
// ============================================================================
function run(command, options = {}) {
  const { silent = false, cwd = rootDir } = options;
  writeLog(`Running: ${command}`);

  try {
    const result = execSync(command, {
      cwd,
      stdio: silent ? "pipe" : "inherit",
      env: { ...process.env, FORCE_COLOR: "1" },
    });
    return { success: true, output: result?.toString() };
  } catch (e) {
    return { success: false, error: e.message };
  }
}

function runAsync(command, options = {}) {
  const { cwd = rootDir } = options;
  writeLog(`Running async: ${command}`);

  return new Promise((resolve) => {
    const proc = spawn(command, [], {
      cwd,
      shell: true,
      stdio: "inherit",
    });

    proc.on("close", (code) => {
      resolve({ success: code === 0, code });
    });

    proc.on("error", (err) => {
      resolve({ success: false, error: err.message });
    });
  });
}

// ============================================================================
// Actions
// ============================================================================
const actions = {
  async fullSetup() {
    clear();
    log("╔══════════════════════════════════════════════════╗", "cyan");
    log("║            Full Project Setup                    ║", "cyan");
    log("╚══════════════════════════════════════════════════╝", "cyan");
    console.log("");

    writeLog("Full setup started");
    let hasError = false;

    log("[1/7] Running pre-flight checks...", "white");
    const nodeCheck = run("node --version", { silent: true });
    if (!nodeCheck.success) {
      error("Node.js is not installed");
      hasError = true;
    } else {
      success(`Node.js: ${nodeCheck.output.trim()}`);
    }

    const pnpmCheck = run("pnpm --version", { silent: true });
    if (!pnpmCheck.success) {
      warn("pnpm not found, will install via corepack");
    } else {
      success(`pnpm: ${pnpmCheck.output.trim()}`);
    }

    if (hasError) {
      await pause();
      return;
    }
    console.log("");

    log("[2/7] Checking running processes...", "white");
    // Note: We no longer auto-kill node.exe to prevent killing ourselves
    // Users should manually stop automation processes before setup if needed
    info("If you have running automations, stop them manually (Ctrl+C)");
    await new Promise((r) => setTimeout(r, 1500)); // Brief pause
    console.log("");

    log("[3/7] Enabling pnpm via corepack...", "white");
    run("corepack enable", { silent: true });
    success("pnpm ready");
    console.log("");

    log("[4/7] Backing up configuration...", "white");
    if (existsSync(join(rootDir, ".env"))) {
      if (process.platform === "win32") {
        run("copy /y .env .env.backup", { silent: true });
      } else {
        run("cp .env .env.backup", { silent: true });
      }
      success(".env backed up");
    } else {
      info("No existing .env to backup");
    }
    console.log("");

    log("[5/7] Installing dependencies...", "white");
    if (process.platform === "win32") {
      run("if exist node_modules rmdir /s /q node_modules", { silent: true });
      run("if exist pnpm-lock.yaml del /f /q pnpm-lock.yaml", { silent: true });
    } else {
      run("rm -rf node_modules pnpm-lock.yaml", { silent: true });
    }

    if (
      !existsSync(join(rootDir, ".env")) &&
      existsSync(join(rootDir, ".env-example"))
    ) {
      if (process.platform === "win32") {
        run("copy .env-example .env", { silent: true });
      } else {
        run("cp .env-example .env", { silent: true });
      }
      success("Created .env from template");
    }

    const installResult = run("pnpm install --force");
    if (!installResult.success) {
      warn("pnpm failed, trying npm...");
      run("npm install --force");
    }
    success("Dependencies installed");
    console.log("");

    log("[6/7] Rebuilding native modules...", "white");
    for (const mod of ["better-sqlite3", "sharp", "esbuild"]) {
      run(`npm rebuild ${mod}`, { silent: true });
    }
    success("Native modules ready");
    console.log("");

    log("[7/7] Setting up git hooks...", "white");
    if (existsSync(join(rootDir, ".git"))) {
      run("npx husky install", { silent: true });
      success("Git hooks installed");
    } else {
      info("Not a git repository");
    }
    console.log("");

    log("╔══════════════════════════════════════════════════╗", "green");
    log("║            ✓ Setup Complete!                     ║", "green");
    log("╚══════════════════════════════════════════════════╝", "green");
    console.log("");
    log("Next steps:", "white");
    console.log("  1. Edit .env with your API keys");
    console.log("  2. Start browser with remote debugging");
    console.log("  3. Run: node main.js pageview=example.com");

    writeLog("Full setup completed");
    await pause();
  },

  async installDeps() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("          Install/Update Dependencies", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    log("Cleaning old dependencies...", "white");
    if (process.platform === "win32") {
      run("if exist node_modules rmdir /s /q node_modules", { silent: true });
      run("if exist pnpm-lock.yaml del /f /q pnpm-lock.yaml", { silent: true });
    } else {
      run("rm -rf node_modules pnpm-lock.yaml", { silent: true });
    }

    log("Installing with pnpm...", "white");
    const result = run("pnpm install --force");

    if (result.success) {
      success("Dependencies installed successfully");
    } else {
      warn("pnpm failed, trying npm...");
      run("npm install --force");
    }

    await pause();
  },

  async rebuildNative() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("           Rebuild Native Modules", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    const modules = ["better-sqlite3", "sharp", "esbuild"];
    let found = false;

    try {
      const pkg = JSON.parse(
        readFileSync(join(rootDir, "package.json"), "utf8"),
      );

      for (const mod of modules) {
        if (pkg.dependencies?.[mod] || pkg.devDependencies?.[mod]) {
          found = true;
          log(`Rebuilding ${mod}...`, "white");
          const result = run(`npm rebuild ${mod}`);
          if (result.success) {
            success(`${mod} rebuilt successfully`);
          } else {
            error(`${mod} rebuild failed`);
          }
        }
      }
    } catch (e) {
      error("Failed to read package.json");
    }

    if (!found) {
      info("No native modules found in package.json");
    }

    await pause();
  },

  async setupHooks() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("              Setup Git Hooks", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    if (!existsSync(join(rootDir, ".git"))) {
      error("Not a git repository");
      await pause();
      return;
    }

    log("Installing husky...", "white");
    const result = run("npx husky install");

    if (result.success) {
      success("Git hooks installed successfully");
    } else {
      error("Failed to install git hooks");
    }

    await pause();
  },

  // Browser Management Actions
  async closeBrowsers() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("            Close Browser Profiles", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    const script = join(rootDir, "scripts", "windows", "browser-close.bat");

    if (process.platform === "win32") {
      if (existsSync(script)) {
        log("Running browser close script...", "white");
        run(`"${script}"`);
        success("Browser close script completed");
      } else {
        error("browser-close.bat not found");
      }
    } else {
      warn("This script is Windows-only");
      info("On Mac/Linux: pkill -f chrome");
    }

    await pause();
  },

  async openIxProfiles() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("           Open ixBrowser Profiles", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    const script = join(rootDir, "scripts", "windows", "ix-open.bat");

    if (process.platform === "win32") {
      if (existsSync(script)) {
        log("Running ixBrowser open script...", "white");
        run(`"${script}"`);
        success("ixBrowser open script completed");
      } else {
        error("ix-open.bat not found");
      }
    } else {
      warn("This script is Windows-only");
    }

    await pause();
  },

  async closeIxProfiles() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("          Close ixBrowser Profiles", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    const script = join(
      rootDir,
      "scripts",
      "windows",
      "ix-close_any_profiles.bat",
    );

    if (process.platform === "win32") {
      if (existsSync(script)) {
        log("Running ixBrowser close script...", "white");
        run(`"${script}"`);
        success("ixBrowser close script completed");
      } else {
        error("ix-close_any_profiles.bat not found");
      }
    } else {
      warn("This script is Windows-only");
    }

    await pause();
  },

  // LLM Server Actions
  async startOllama() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("             Start Ollama Server", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    const script = join(rootDir, "scripts", "windows", "start-ollama.bat");

    if (process.platform === "win32") {
      if (existsSync(script)) {
        log("Starting Ollama server...", "white");
        log("Note: This will run in the current window", "gray");
        log("Press Ctrl+C to stop the server", "gray");
        console.log("");
        run(`"${script}"`);
      } else {
        error("start-ollama.bat not found");
      }
    } else {
      warn("This script is Windows-only");
      info("On Mac/Linux: ollama serve");
    }

    await pause();
  },

  async startDocker() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("            Start Docker LLM Server", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    const script = join(rootDir, "scripts", "windows", "start-docker.bat");

    if (process.platform === "win32") {
      if (existsSync(script)) {
        log("Starting Docker LLM server...", "white");
        run(`"${script}"`);
        success("Docker LLM server started");
      } else {
        error("start-docker.bat not found");
      }
    } else {
      warn("This script is Windows-only");
    }

    await pause();
  },

  async setupOllama() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("              Setup Ollama", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    const script = join(rootDir, "scripts", "windows", "setup-ollama.bat");

    if (process.platform === "win32") {
      if (existsSync(script)) {
        log("Running Ollama setup script...", "white");
        run(`"${script}"`);
        success("Ollama setup completed");
      } else {
        error("setup-ollama.bat not found");
      }
    } else {
      warn("This script is Windows-only");
    }

    await pause();
  },

  // Tools & Utilities
  async backup() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("               Create Backup", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    const backupScript = join(rootDir, "scripts", "backup.ps1");

    if (existsSync(backupScript)) {
      log("Running backup script...", "white");
      console.log("");

      if (process.platform === "win32") {
        run(`powershell -ExecutionPolicy Bypass -File "${backupScript}"`);
      } else {
        run(`pwsh -ExecutionPolicy Bypass -File "${backupScript}"`);
      }

      success("Backup completed");
    } else {
      error("backup.ps1 not found");
    }

    await pause();
  },

  async runParallelTests() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("            Run Parallel Tests", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    const script = join(rootDir, "scripts", "windows", "vitest-individual.ps1");

    if (process.platform === "win32") {
      if (existsSync(script)) {
        log("Running parallel tests...", "white");
        log("This may take several minutes...", "gray");
        console.log("");
        run(`powershell -ExecutionPolicy Bypass -File "${script}"`);
        success("Test run completed");
      } else {
        error("vitest-individual.ps1 not found");
      }
    } else {
      warn("This script is Windows-only");
      info("On Mac/Linux: pnpm run test:bun:all");
    }

    await pause();
  },

  async setupDashboard() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("              Setup Dashboard", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    const dashboardDir = join(rootDir, "api", "ui", "electron-dashboard");

    if (!existsSync(dashboardDir)) {
      error("Dashboard directory not found");
      await pause();
      return;
    }

    log("Setting up dashboard...", "white");

    if (process.platform === "win32") {
      run("if exist node_modules rmdir /s /q node_modules", {
        silent: true,
        cwd: dashboardDir,
      });
    } else {
      run("rm -rf node_modules", { silent: true, cwd: dashboardDir });
    }

    if (
      existsSync(join(dashboardDir, ".env-example")) &&
      !existsSync(join(dashboardDir, ".env"))
    ) {
      if (process.platform === "win32") {
        run("copy .env-example .env", { silent: true, cwd: dashboardDir });
      } else {
        run("cp .env-example .env", { silent: true, cwd: dashboardDir });
      }
      success("Created dashboard .env");
    }

    log("Installing dashboard dependencies...", "white");
    const result = run("pnpm install --force", { cwd: dashboardDir });

    if (result.success) {
      success("Dashboard dependencies installed");
    } else {
      warn("pnpm failed, trying npm...");
      run("npm install --force", { cwd: dashboardDir });
    }

    console.log("");
    success("Dashboard setup complete");

    await pause();
  },

  async openUiTools() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("              Open UI Tools", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    const script = join(rootDir, "scripts", "windows", "ui.ps1");

    if (process.platform === "win32") {
      if (existsSync(script)) {
        log("Opening UI tools...", "white");
        run(`powershell -ExecutionPolicy Bypass -File "${script}"`);
      } else {
        error("ui.ps1 not found");
      }
    } else {
      warn("This script is Windows-only");
    }

    await pause();
  },

  // Diagnostics
  async doctor() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("              System Diagnostics", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    let issues = 0;

    log("[1/8] Node.js", "white");
    const nodeCheck = run("node --version", { silent: true });
    if (nodeCheck.success) {
      success(`Installed: ${nodeCheck.output.trim()}`);
    } else {
      error("Not installed");
      issues++;
    }
    console.log("");

    log("[2/8] pnpm", "white");
    const pnpmCheck = run("pnpm --version", { silent: true });
    if (pnpmCheck.success) {
      success(`Installed: ${pnpmCheck.output.trim()}`);
    } else {
      error("Not installed");
      issues++;
    }
    console.log("");

    log("[3/8] Git", "white");
    if (run("git --version", { silent: true }).success) {
      success("Git installed");
    } else {
      warn("Not installed (optional)");
    }
    console.log("");

    log("[4/8] Python", "white");
    if (
      run("python --version", { silent: true }).success ||
      run("python3 --version", { silent: true }).success
    ) {
      success("Python installed");
    } else {
      warn("Not installed (may affect native modules)");
    }
    console.log("");

    log("[5/8] Docker", "white");
    if (run("docker --version", { silent: true }).success) {
      success("Docker installed");
    } else {
      info("Not installed (optional)");
    }
    console.log("");

    log("[6/8] Project Structure", "white");
    if (existsSync(join(rootDir, "package.json"))) {
      success("package.json exists");
    } else {
      error("package.json not found");
      issues++;
    }
    if (existsSync(join(rootDir, "api", "index.js"))) {
      success("api/index.js exists");
    } else {
      error("api/index.js not found");
      issues++;
    }
    console.log("");

    log("[7/8] Dependencies", "white");
    if (existsSync(join(rootDir, "node_modules"))) {
      success("node_modules exists");
    } else {
      error("node_modules not found");
      issues++;
    }
    console.log("");

    log("[8/8] Configuration", "white");
    if (existsSync(join(rootDir, ".env"))) {
      success(".env exists");
    } else {
      warn(".env not found");
      issues++;
    }
    console.log("");

    log("────────────────────────────────────────────────────", "gray");
    if (issues === 0) {
      success("All checks passed!");
    } else {
      warn(`Found ${issues} issue(s) to address`);
    }

    await pause();
  },

  async clean() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("            Clean Build Artifacts", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    log("The following will be removed:", "yellow");
    console.log("  - node_modules/");
    console.log("  - pnpm-lock.yaml");
    console.log("  - api/coverage/");
    console.log("  - api/coverage-v8/");
    console.log("  - screenshots/");
    console.log("");

    const confirmed = await confirm("Continue?");
    if (!confirmed) {
      info("Cancelled");
      await pause();
      return;
    }

    console.log("");
    log("Cleaning...", "white");

    const toRemove = [
      "node_modules",
      "pnpm-lock.yaml",
      "api/coverage",
      "api/coverage-v8",
      "screenshots",
    ];

    for (const item of toRemove) {
      const fullPath = join(rootDir, item);
      if (existsSync(fullPath)) {
        try {
          rmSync(fullPath, { recursive: true, force: true });
          log(`  Removed: ${item}`, "gray");
        } catch (e) {
          /* ignore cleanup errors */
        }
      }
    }

    success("Cleanup complete");
    await pause();
  },

  async viewLog() {
    clear();
    log("════════════════════════════════════════════════════", "cyan");
    log("                Setup Log", "cyan");
    log("════════════════════════════════════════════════════", "cyan");
    console.log("");

    if (existsSync(LOG_FILE)) {
      log(`Log file: ${LOG_FILE}`, "white");
      log("────────────────────────────────────────────────────", "gray");
      console.log(readFileSync(LOG_FILE, "utf8"));
    } else {
      info("No log file found. Run setup first.");
    }

    await pause();
  },
};

// ============================================================================
// Main
// ============================================================================
async function main() {
  process.on("SIGINT", () => {
    showCursor();
    process.exit(0);
  });
  process.on("exit", () => {
    showCursor();
  });

  while (true) {
    const selected = await showMenu();

    if (selected.action === "exit") {
      clear();
      console.log("");
      log("Thank you for using Auto-AI Setup!", "cyan");
      console.log("");
      process.exit(0);
    }

    if (actions[selected.action]) {
      await actions[selected.action]();
    }
  }
}

main().catch((err) => {
  showCursor();
  console.error(err);
  process.exit(1);
});
