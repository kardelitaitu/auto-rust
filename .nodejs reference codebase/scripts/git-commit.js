#!/usr/bin/env node
/**
 * Auto-AI Framework - Git Commit Helper
 * Stage → Lint → Commit (default: no push)
 *
 * Usage: pnpm commit "message" [--no-verify] [--push]
 *
 * Options:
 *   --no-verify, -n  Skip lint-staged checks
 *   --push, -P       Push to remote after commit (default: no push)
 */

import { execSync } from "child_process";

const colors = {
  reset: "\x1b[0m",
  bright: "\x1b[1m",
  green: "\x1b[32m",
  red: "\x1b[31m",
  yellow: "\x1b[33m",
  cyan: "\x1b[36m",
  magenta: "\x1b[35m",
};

const log = {
  info: (msg) => console.log(`${colors.cyan}ℹ${colors.reset} ${msg}`),
  success: (msg) => console.log(`${colors.green}✅${colors.reset} ${msg}`),
  error: (msg) => console.log(`${colors.red}❌${colors.reset} ${msg}`),
  warn: (msg) => console.log(`${colors.yellow}⚠️${colors.reset} ${msg}`),
  step: (msg) => console.log(`${colors.magenta}▸${colors.reset} ${msg}`),
};

const args = process.argv.slice(2);
const skipVerify = args.includes("--no-verify") || args.includes("-n");
const push = args.includes("--push") || args.includes("-P");

let message = args.filter((arg) => !arg.startsWith("-")).join(" ");

// Auto-generate message if not provided
if (!message) {
  const now = new Date();
  const months = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
  ];

  const day = String(now.getDate()).padStart(2, "0");
  const month = months[now.getMonth()];
  const year = now.getFullYear();

  let hours = now.getHours();
  const minutes = String(now.getMinutes()).padStart(2, "0");
  const ampm = hours >= 12 ? "PM" : "AM";
  hours = hours % 12 || 12;

  message = `${day} ${month} ${year} - ${hours}:${minutes} ${ampm}`;
  log.info(`No message provided, using auto-generated: "${message}"`);
}

let attempt = 0;
const maxAttempts = 2;
let lintPassed = false;

while (attempt < maxAttempts) {
  attempt++;

  try {
    log.step("Staging files...");
    execSync("git add -A", { stdio: "inherit" });

    if (skipVerify) {
      log.info("Skipping lint-staged (--no-verify)");
    } else {
      log.step("Running lint-staged (auto-fix + format)...");
      try {
        execSync("pnpm exec lint-staged", { stdio: "inherit" });
        lintPassed = true;
      } catch (lintError) {
        if (attempt >= maxAttempts) {
          log.error(
            "Lint failed - files have issues that cannot be auto-fixed",
          );
          console.log(`\n${colors.yellow}📋 To fix manually:${colors.reset}`);
          console.log("   1. Check the eslint errors above");
          console.log("   2. Run: pnpm lint:fix");
          console.log("   3. Run: pnpm format");
          console.log(`   4. Then retry: pnpm commit "${message}"`);
        }
        throw lintError;
      }

      log.step("Re-staging lint-fixed files...");
      execSync("git add -A", { stdio: "inherit" });
    }

    const status = execSync("git status --porcelain").toString().trim();
    if (!status) {
      log.warn("No changes to commit");
      process.exit(0);
    }

    log.step("Committing...");
    execSync(`git commit -m "${message}"`, { stdio: "inherit" });

    log.success("Commit successful!");
    console.log(`   ${colors.bright}Message:${colors.reset} "${message}"`);

    if (push) {
      log.step("Pushing to remote...");
      execSync("git push", { stdio: "inherit" });
      log.success("Pushed to remote!");
    } else {
      log.info("Skipping push (default behavior)");
      log.info("To push later, run: git push");
    }

    process.exit(0);
  } catch (_error) {
    if (attempt < maxAttempts) {
      log.warn(`Attempt ${attempt} failed, retrying...`);
    } else {
      log.error("Commit failed after 2 attempts");

      if (!lintPassed && !skipVerify) {
        console.log(`\n${colors.yellow}📋 To fix:${colors.reset}`);
        console.log("   1. Run: pnpm lint:fix");
        console.log("   2. Run: pnpm format");
        console.log(`   3. Run: pnpm commit "${message}"`);
      }
      process.exit(1);
    }
  }
}
