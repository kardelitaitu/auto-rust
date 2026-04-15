#!/usr/bin/env node

/**
 * Release Automation Script
 *
 * Automates the release process:
 * - Version bump
 * - Changelog generation
 * - Git tag creation
 * - Release notes generation
 *
 * @example
 * node scripts/release.js patch    # 1.2.0 -> 1.2.1
 * node scripts/release.js minor    # 1.2.0 -> 1.3.0
 * node scripts/release.js major    # 1.2.0 -> 2.0.0
 * node scripts/release.js 1.2.3    # Specific version
 */

import { readFileSync, writeFileSync, existsSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";
import { execSync } from "child_process";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..");

// Colors for output
const colors = {
  reset: "\x1b[0m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  cyan: "\x1b[36m",
};

function log(message, color = "reset") {
  console.log(`${colors[color]}${message}${colors.reset}`);
}

function exec(command) {
  return execSync(command, { cwd: ROOT, encoding: "utf8" });
}

function getCurrentVersion() {
  const packageJson = JSON.parse(
    readFileSync(join(ROOT, "package.json"), "utf8"),
  );
  return packageJson.version;
}

function bumpVersion(current, type) {
  const [major, minor, patch] = current.split(".").map(Number);

  switch (type) {
    case "patch":
      return `${major}.${minor}.${patch + 1}`;
    case "minor":
      return `${major}.${minor + 1}.0`;
    case "major":
      return `${major + 1}.0.0`;
    default:
      // Assume specific version
      if (/^\d+\.\d+\.\d+$/.test(type)) {
        return type;
      }
      throw new Error(`Invalid version type: ${type}`);
  }
}

function updatePackageJson(newVersion) {
  const packagePath = join(ROOT, "package.json");
  const packageJson = JSON.parse(readFileSync(packagePath, "utf8"));
  packageJson.version = newVersion;
  writeFileSync(packagePath, JSON.stringify(packageJson, null, 4) + "\n");
  log(`✓ Updated package.json to v${newVersion}`, "green");
}

function updateChangelog(newVersion) {
  const changelogPath = join(ROOT, "CHANGELOG.md");
  const date = new Date().toISOString().split("T")[0];

  let changelog = "";
  if (existsSync(changelogPath)) {
    changelog = readFileSync(changelogPath, "utf8");
  }

  const newEntry = `## [${newVersion}] - ${date}

### Added

- 

### Changed

- 

### Deprecated

- 

### Removed

- 

### Fixed

- 

### Security

- 

---

`;

  // Insert after the header
  const lines = changelog.split("\n");
  const insertIndex = lines.findIndex((line) => line.startsWith("## [")) || 4;
  lines.splice(insertIndex, 0, newEntry);

  writeFileSync(changelogPath, lines.join("\n"));
  log(`✓ Updated CHANGELOG.md for v${newVersion}`, "green");
}

function generateReleaseNotes(newVersion) {
  try {
    // Get commits since last tag
    const lastTag = exec(
      'git describe --tags --abbrev=0 2>/dev/null || echo ""',
    ).trim();
    let commitRange = lastTag ? `${lastTag}..HEAD` : "HEAD";

    const commits = exec(
      `git log ${commitRange} --pretty=format:"- %s (%h)"`,
    ).trim();

    if (!commits) {
      log("⚠ No new commits since last release", "yellow");
      return "";
    }

    const notes = `## Release v${newVersion}

### Commits

${commits}

### Contributors

$(git log ${commitRange} --pretty=format:"- %an" | sort -u)
`;
    log("✓ Generated release notes", "green");
    return notes;
  } catch (error) {
    log(`⚠ Could not generate release notes: ${error.message}`, "yellow");
    return "";
  }
}

function main() {
  const versionType = process.argv[2];

  if (!versionType) {
    log("Usage: node scripts/release.js <patch|minor|major|version>", "yellow");
    log("  patch  - Bug fix release (1.2.0 -> 1.2.1)", "cyan");
    log("  minor  - Feature release (1.2.0 -> 1.3.0)", "cyan");
    log("  major  - Breaking changes (1.2.0 -> 2.0.0)", "cyan");
    log("  1.2.3  - Specific version", "cyan");
    process.exit(1);
  }

  log("🚀 Release Automation", "blue");
  log("====================\n", "blue");

  // Step 1: Get current version
  const currentVersion = getCurrentVersion();
  log(`Current version: v${currentVersion}`, "cyan");

  // Step 2: Calculate new version
  const newVersion = bumpVersion(currentVersion, versionType);
  log(`New version: v${newVersion}`, "cyan");

  // Step 3: Confirm
  log("\n⚠ This will:", "yellow");
  log(`  1. Update package.json to v${newVersion}`, "yellow");
  log(`  2. Update CHANGELOG.md`, "yellow");
  log(`  3. Create git commit`, "yellow");
  log(`  4. Create git tag v${newVersion}`, "yellow");
  log("\nContinue? (y/n): ", "yellow");

  // For non-interactive use, check for --yes flag
  if (process.argv.includes("--yes") || process.argv.includes("-y")) {
    log("Auto-confirmed via --yes flag\n", "green");
  } else {
    log("Run with --yes to skip confirmation", "gray");
    // In non-interactive mode, exit
    if (!process.stdin.isTTY) {
      log("Error: Non-interactive mode. Use --yes flag.", "red");
      process.exit(1);
    }
  }

  // Step 4: Update files
  log("\n📝 Updating files...", "blue");
  updatePackageJson(newVersion);
  updateChangelog(newVersion);

  // Step 5: Git operations
  log("\n🔧 Git operations...", "blue");

  exec("git add package.json CHANGELOG.md");
  log("✓ Staged changes", "green");

  exec(`git commit -m "chore: release v${newVersion}"`);
  log("✓ Created commit", "green");

  exec(`git tag -a v${newVersion} -m "Release v${newVersion}"`);
  log(`✓ Created tag v${newVersion}`, "green");

  // Step 6: Generate release notes
  log("\n📋 Release Notes:", "blue");
  const notes = generateReleaseNotes(newVersion);
  if (notes) {
    log(notes, "cyan");
  }

  // Step 7: Instructions
  log("\n✅ Release prepared successfully!", "green");
  log("\nNext steps:", "blue");
  log(`  1. Review changes: git show v${newVersion}`, "cyan");
  log(`  2. Push to remote: git push origin main --tags`, "cyan");
  log(`  3. Publish release on GitHub`, "cyan");
  log(`  4. Run: pnpm run setup (for users)`, "cyan");
}

main();
