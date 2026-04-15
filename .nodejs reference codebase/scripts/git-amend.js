#!/usr/bin/env node
/**
 * Auto-AI Framework - Git Amend Helper
 * Stage → Lint → Amend (default: no push)
 *
 * Usage: pnpm amend ["new message"] [--no-verify] [--push]
 *
 * Options:
 *   --no-verify, -n  Skip lint-staged checks
 *   --push, -P       Force push after amend (default: no push)
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

const message = args.filter((arg) => !arg.startsWith("-")).join(" ");

let attempt = 0;
const maxAttempts = 2;

while (attempt < maxAttempts) {
  attempt++;

  try {
    log.step("Staging changes...");
    execSync("git add -A", { stdio: "inherit" });

    if (!skipVerify) {
      log.step("Running lint-staged...");
      try {
        execSync("pnpm exec lint-staged", { stdio: "inherit" });
      } catch (lintError) {
        if (attempt >= maxAttempts) {
          log.error("Lint failed");
          console.log(
            `\n${colors.yellow}📋 Run: pnpm lint:fix then pnpm amend${colors.reset}`,
          );
        }
        throw lintError;
      }

      log.step("Re-staging lint-fixed files...");
      execSync("git add -A", { stdio: "inherit" });
    }

    if (message) {
      log.step("Amending with new message...");
      execSync(`git commit --amend -m "${message}"`, { stdio: "inherit" });
      log.success("Amend successful!");
      console.log(
        `   ${colors.bright}New message:${colors.reset} "${message}"`,
      );
    } else {
      log.step("Amending (keeping same message)...");
      execSync("git commit --amend --no-edit", { stdio: "inherit" });
      log.success("Amend successful!");
    }

    if (push) {
      log.step("Force pushing to remote...");
      execSync("git push --force", { stdio: "inherit" });
      log.success("Force pushed to remote!");
    } else {
      log.info("Skipping push (default behavior)");
      log.info("To push later, run: git push");
    }

    process.exit(0);
  } catch (_error) {
    if (attempt < maxAttempts) {
      log.warn(`Attempt ${attempt} failed, retrying...`);
    } else {
      log.error("Amend failed after 2 attempts");
      process.exit(1);
    }
  }
}
