#!/usr/bin/env node

/**
 * Quality Gate Script
 *
 * Pre-commit/pre-PR validation checks:
 * - Lint errors
 * - Test failures
 * - Code coverage
 * - File size limits
 * - TODO/FIXME comments
 * - Console.log statements (in production code)
 *
 * @example
 * node scripts/quality-gate.js          # Full check
 * node scripts/quality-gate.js --quick  # Fast check (lint + tests only)
 * node scripts/quality-gate.js --ci     # CI mode (strict)
 */

import { execSync } from "child_process";
import { readFileSync, readdirSync, statSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..");

// Configuration
const CONFIG = {
  maxFileSize: 500, // lines
  allowedTodos: ["TODO", "FIXME", "HACK", "XXX"],
  allowedConsoleFiles: [
    "scripts/",
    "tasks/",
    "main.js",
    "agent-main.js",
    "api/core/logger.js",
    "api/tests/",
  ],
  minCoverage: 80, // percent
  skipCoverageCheck: false,
};

// Colors
const colors = {
  reset: "\x1b[0m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  cyan: "\x1b[36m",
  gray: "\x1b[90m",
};

function log(message, color = "reset") {
  console.log(`${colors[color]}${message}${colors.reset}`);
}

function exec(command, options = {}) {
  try {
    return execSync(command, {
      cwd: ROOT,
      encoding: "utf8",
      stdio: ["pipe", "pipe", "pipe"],
      ...options,
    });
  } catch (error) {
    if (options.allowFailure) return "";
    throw error;
  }
}

class QualityGate {
  constructor(mode = "full") {
    this.mode = mode;
    this.errors = [];
    this.warnings = [];
    this.passed = [];
  }

  async run() {
    log("🔍 Quality Gate Check", "blue");
    log("====================\n", "blue");
    log(`Mode: ${this.mode}\n`, "gray");

    // Always run these
    await this.checkLint();
    await this.checkTests();
    await this.checkFormat();

    if (this.mode === "full" || this.mode === "ci") {
      await this.checkCoverage();
      await this.checkFileSizes();
      await this.checkTodos();
      await this.checkConsoleLogs();
      await this.checkGitStatus();
    }

    this.report();
  }

  async checkLint() {
    log("Checking lint...", "cyan");
    try {
      exec("pnpm run lint");
      this.passed.push("✓ Lint passed");
      log("  ✓ Lint passed\n", "green");
    } catch (error) {
      this.errors.push("✗ Lint failed - run: pnpm run lint:fix");
      log("  ✗ Lint failed\n", "red");
      if (this.mode === "ci") {
        console.error(error.stdout?.toString() || error.message);
      }
    }
  }

  async checkTests() {
    log("Checking tests...", "cyan");
    try {
      exec("pnpm run test:bun:unit", { timeout: 120000 });
      this.passed.push("✓ Tests passed");
      log("  ✓ Tests passed\n", "green");
    } catch (error) {
      this.errors.push("✗ Tests failed - run: pnpm run test:bun:unit");
      log("  ✗ Tests failed\n", "red");
      if (this.mode === "ci") {
        console.error(error.stdout?.toString() || error.message);
      }
    }
  }

  async checkFormat() {
    log("Checking format...", "cyan");
    try {
      const diff = exec("pnpm exec prettier --check . 2>&1 || true");
      if (diff.includes("Not formatted")) {
        this.errors.push("✗ Format check failed - run: pnpm run format");
        log("  ✗ Format check failed\n", "red");
      } else {
        this.passed.push("✓ Format check passed");
        log("  ✓ Format check passed\n", "green");
      }
    } catch (error) {
      this.warnings.push("⚠ Format check skipped");
      log("  ⚠ Format check skipped\n", "yellow");
    }
  }

  async checkCoverage() {
    if (CONFIG.skipCoverageCheck) {
      this.warnings.push("⚠ Coverage check skipped");
      log("  ⚠ Coverage check skipped\n", "yellow");
      return;
    }

    log("Checking coverage...", "cyan");
    try {
      // Run coverage and parse output
      const output = exec("pnpm run test:bun:coverage 2>&1 || true");

      // Look for coverage percentage
      const match = output.match(/All files\s+\|\s+([\d.]+)\s+\|/);
      if (match) {
        const coverage = parseFloat(match[1]);
        if (coverage >= CONFIG.minCoverage) {
          this.passed.push(
            `✓ Coverage: ${coverage}% (≥${CONFIG.minCoverage}%)`,
          );
          log(
            `  ✓ Coverage: ${coverage}% (≥${CONFIG.minCoverage}%)\n`,
            "green",
          );
        } else {
          this.errors.push(
            `✗ Coverage: ${coverage}% (<${CONFIG.minCoverage}%)`,
          );
          log(`  ✗ Coverage: ${coverage}% (<${CONFIG.minCoverage}%)\n`, "red");
        }
      } else {
        this.warnings.push("⚠ Could not parse coverage output");
        log("  ⚠ Could not parse coverage output\n", "yellow");
      }
    } catch (error) {
      this.warnings.push("⚠ Coverage check failed");
      log("  ⚠ Coverage check failed\n", "yellow");
    }
  }

  async checkFileSizes() {
    log("Checking file sizes...", "cyan");
    const largeFiles = [];

    const checkDir = (dir, exclude = []) => {
      const files = readdirSync(dir, { withFileTypes: true });
      for (const file of files) {
        const fullPath = join(dir, file.name);
        if (exclude.some((e) => fullPath.includes(e))) continue;
        if (file.isDirectory()) {
          checkDir(fullPath, exclude);
        } else if (file.isFile() && file.name.endsWith(".js")) {
          const content = readFileSync(fullPath, "utf8");
          const lines = content.split("\n").length;
          if (lines > CONFIG.maxFileSize) {
            largeFiles.push({ path: fullPath.replace(ROOT + "/", ""), lines });
          }
        }
      }
    };

    checkDir(join(ROOT, "api"), ["tests/", "node_modules/"]);

    if (largeFiles.length === 0) {
      this.passed.push("✓ All files under size limit");
      log("  ✓ All files under size limit\n", "green");
    } else {
      this.warnings.push(`⚠ ${largeFiles.length} large files found`);
      log(`  ⚠ ${largeFiles.length} large files found:\n`, "yellow");
      largeFiles.slice(0, 5).forEach((f) => {
        log(`    - ${f.path} (${f.lines} lines)\n`, "gray");
      });
    }
  }

  async checkTodos() {
    log("Checking TODO comments...", "cyan");
    const todos = [];

    const checkDir = (dir, exclude = []) => {
      const files = readdirSync(dir, { withFileTypes: true });
      for (const file of files) {
        const fullPath = join(dir, file.name);
        if (exclude.some((e) => fullPath.includes(e))) continue;
        if (file.isDirectory()) {
          checkDir(fullPath, exclude);
        } else if (file.isFile() && /\.(js|ts)$/.test(file.name)) {
          const content = readFileSync(fullPath, "utf8");
          const lines = content.split("\n");
          lines.forEach((line, idx) => {
            CONFIG.allowedTodos.forEach((todo) => {
              if (line.includes(todo)) {
                todos.push({
                  file: fullPath.replace(ROOT + "/", ""),
                  line: idx + 1,
                  content: line.trim(),
                });
              }
            });
          });
        }
      }
    };

    checkDir(join(ROOT, "api"), ["tests/", "node_modules/"]);

    if (todos.length === 0) {
      this.passed.push("✓ No TODO comments found");
      log("  ✓ No TODO comments\n", "green");
    } else {
      this.warnings.push(`⚠ ${todos.length} TODO comments found`);
      log(`  ⚠ ${todos.length} TODO comments found:\n`, "yellow");
      todos.slice(0, 5).forEach((t) => {
        log(`    - ${t.file}:${t.line} - ${t.content}\n`, "gray");
      });
      if (todos.length > 5) {
        log(`    ... and ${todos.length - 5} more\n`, "gray");
      }
    }
  }

  async checkConsoleLogs() {
    log("Checking console.log statements...", "cyan");
    const consoleLogs = [];

    const checkDir = (dir) => {
      const files = readdirSync(dir, { withFileTypes: true });
      for (const file of files) {
        const fullPath = join(dir, file.name);
        if (CONFIG.allowedConsoleFiles.some((f) => fullPath.includes(f)))
          continue;
        if (file.isDirectory()) {
          checkDir(fullPath);
        } else if (file.isFile() && /\.(js|ts)$/.test(file.name)) {
          const content = readFileSync(fullPath, "utf8");
          const lines = content.split("\n");
          lines.forEach((line, idx) => {
            if (/console\.(log|warn|error|debug|info)\(/.test(line)) {
              consoleLogs.push({
                file: fullPath.replace(ROOT + "/", ""),
                line: idx + 1,
                content: line.trim(),
              });
            }
          });
        }
      }
    };

    checkDir(join(ROOT, "api"));

    if (consoleLogs.length === 0) {
      this.passed.push("✓ No console.log in production code");
      log("  ✓ No console.log in production code\n", "green");
    } else {
      this.warnings.push(
        `⚠ ${consoleLogs.length} console.log statements in production code`,
      );
      log(
        `  ⚠ ${consoleLogs.length} console.log statements found:\n`,
        "yellow",
      );
      consoleLogs.slice(0, 5).forEach((c) => {
        log(`    - ${c.file}:${c.line}\n`, "gray");
      });
    }
  }

  async checkGitStatus() {
    log("Checking git status...", "cyan");
    try {
      const status = exec("git status --porcelain");
      if (status.trim()) {
        this.warnings.push("⚠ Uncommitted changes found");
        log("  ⚠ Uncommitted changes found:\n", "yellow");
        status
          .split("\n")
          .slice(0, 10)
          .forEach((line) => {
            if (line.trim()) log(`    ${line}\n`, "gray");
          });
      } else {
        this.passed.push("✓ Git working tree clean");
        log("  ✓ Git working tree clean\n", "green");
      }
    } catch (error) {
      this.warnings.push("⚠ Git check skipped (not a git repo?)");
      log("  ⚠ Git check skipped\n", "yellow");
    }
  }

  report() {
    log("\n====================", "blue");
    log("Quality Gate Report", "blue");
    log("====================\n", "blue");

    if (this.passed.length > 0) {
      log("Passed:", "green");
      this.passed.forEach((p) => log(`  ${p}`, "green"));
      log("");
    }

    if (this.warnings.length > 0) {
      log("Warnings:", "yellow");
      this.warnings.forEach((w) => log(`  ${w}`, "yellow"));
      log("");
    }

    if (this.errors.length > 0) {
      log("Errors:", "red");
      this.errors.forEach((e) => log(`  ${e}`, "red"));
      log("");
    }

    log("Summary:", "blue");
    log(
      `  Passed: ${this.passed.length}`,
      this.passed.length > 0 ? "green" : "gray",
    );
    log(
      `  Warnings: ${this.warnings.length}`,
      this.warnings.length > 0 ? "yellow" : "gray",
    );
    log(
      `  Errors: ${this.errors.length}`,
      this.errors.length > 0 ? "red" : "green",
    );

    if (this.errors.length > 0) {
      log("\n❌ Quality gate FAILED", "red");
      log("Fix errors and try again", "yellow");
      process.exit(1);
    } else {
      log("\n✅ Quality gate PASSED", "green");
      process.exit(0);
    }
  }
}

// Main
const mode =
  process.argv.find((a) => a.startsWith("--"))?.replace("--", "") || "full";
const gate = new QualityGate(mode);
gate.run().catch((error) => {
  log(`Fatal error: ${error.message}`, "red");
  process.exit(1);
});
