#!/usr/bin/env node

/**
 * Benchmark Comparison Script
 *
 * Compares benchmark results between runs:
 * - Loads previous benchmark results
 * - Runs new benchmarks
 * - Compares and reports differences
 * - Detects performance regressions
 *
 * @example
 * node scripts/benchmark-compare.js          # Compare with last run
 * node scripts/benchmark-compare.js --save   # Save new results
 * node scripts/benchmark-compare.js --baseline v1.0.0  # Compare with tag
 */

import { execSync } from "child_process";
import { readFileSync, writeFileSync, existsSync, mkdirSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..");
const BENCHMARKS_DIR = join(ROOT, "api", "tests", "benchmarks");
const RESULTS_FILE = join(ROOT, "benchmark-results.json");

// Colors
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

function loadPreviousResults() {
  if (!existsSync(RESULTS_FILE)) {
    return null;
  }
  return JSON.parse(readFileSync(RESULTS_FILE, "utf8"));
}

function saveResults(results) {
  const dir = dirname(RESULTS_FILE);
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }
  writeFileSync(RESULTS_FILE, JSON.stringify(results, null, 2) + "\n");
}

function runBenchmarks() {
  log("Running benchmarks...", "cyan");

  try {
    // Run benchmark tests
    const output = exec(
      "pnpm run test:bun:unit api/tests/benchmarks 2>&1 || true",
    );

    // Parse benchmark output (adjust regex based on your benchmark format)
    const results = {};
    const lines = output.split("\n");

    for (const line of lines) {
      // Example: ✓ query-performance  45ms
      const match = line.match(/✓\s+([\w-]+)\s+([\d.]+)ms/);
      if (match) {
        results[match[1]] = parseFloat(match[2]);
      }
    }

    return results;
  } catch (error) {
    log(`Error running benchmarks: ${error.message}`, "red");
    return null;
  }
}

function compareResults(previous, current) {
  if (!previous) {
    log("No previous results to compare", "yellow");
    return null;
  }

  const comparisons = [];
  const regressions = [];
  const improvements = [];

  for (const [name, currentValue] of Object.entries(current)) {
    const previousValue = previous[name];

    if (previousValue === undefined) {
      comparisons.push({ name, status: "new", change: 0 });
      continue;
    }

    const change = currentValue - previousValue;
    const percentChange = (change / previousValue) * 100;

    comparisons.push({
      name,
      previous: previousValue,
      current: currentValue,
      change,
      percentChange,
    });

    if (change > 0) {
      regressions.push({ name, change, percentChange });
    } else if (change < 0) {
      improvements.push({ name, change, percentChange });
    }
  }

  // Check for removed benchmarks
  for (const name of Object.keys(previous)) {
    if (!current[name]) {
      comparisons.push({ name, status: "removed" });
    }
  }

  return { comparisons, regressions, improvements };
}

function report(comparison) {
  if (!comparison) {
    return;
  }

  log("\n====================", "blue");
  log("Benchmark Comparison", "blue");
  log("====================\n", "blue");

  const { comparisons, regressions, improvements } = comparison;

  // Summary
  log(`Total benchmarks: ${comparisons.length}`, "cyan");
  log(`Improvements: ${improvements.length}`, "green");
  log(
    `Regressions: ${regressions.length}`,
    regressions.length > 0 ? "red" : "green",
  );
  log("");

  // Regressions (slower)
  if (regressions.length > 0) {
    log("⚠ Regressions (slower):", "red");
    regressions
      .sort((a, b) => b.percentChange - a.percentChange)
      .forEach((r) => {
        log(
          `  ${r.name}: +${r.change.toFixed(2)}ms (+${r.percentChange.toFixed(1)}%)`,
          "red",
        );
      });
    log("");
  }

  // Improvements (faster)
  if (improvements.length > 0) {
    log("✓ Improvements (faster):", "green");
    improvements
      .sort((a, b) => a.percentChange - b.percentChange)
      .forEach((i) => {
        log(
          `  ${i.name}: ${i.change.toFixed(2)}ms (${i.percentChange.toFixed(1)}%)`,
          "green",
        );
      });
    log("");
  }

  // New/Removed
  const newBenchmarks = comparisons.filter((c) => c.status === "new");
  const removedBenchmarks = comparisons.filter((c) => c.status === "removed");

  if (newBenchmarks.length > 0) {
    log("🆕 New benchmarks:", "cyan");
    newBenchmarks.forEach((b) => log(`  ${b.name}`, "cyan"));
    log("");
  }

  if (removedBenchmarks.length > 0) {
    log("🗑️  Removed benchmarks:", "yellow");
    removedBenchmarks.forEach((b) => log(`  ${b.name}`, "yellow"));
    log("");
  }

  // Verdict
  if (regressions.length > 0) {
    const severeRegressions = regressions.filter((r) => r.percentChange > 10);
    if (severeRegressions.length > 0) {
      log("❌ SEVERE REGRESSIONS DETECTED", "red");
      log("Consider investigating before merging", "yellow");
      process.exit(1);
    } else {
      log("⚠ Minor regressions detected", "yellow");
      log("Review and approve if acceptable", "cyan");
    }
  } else {
    log("✅ No performance regressions", "green");
  }
}

function main() {
  log("🏃 Benchmark Comparison", "blue");
  log("======================\n", "blue");

  const args = process.argv.slice(2);
  const shouldSave = args.includes("--save");
  const baselineTag =
    args.find((a) => a === "--baseline")?.[1] ||
    args[args.indexOf("--baseline") + 1];

  // Load previous results
  let previous = loadPreviousResults();

  if (baselineTag) {
    log(`Comparing with baseline: ${baselineTag}`, "cyan");
    // In a real implementation, you'd checkout the tag and run benchmarks
    log("Baseline comparison not fully implemented yet", "yellow");
  } else if (!previous) {
    log("No previous results found. This will be the baseline.", "yellow");
  }

  // Run new benchmarks
  const current = runBenchmarks();

  if (!current || Object.keys(current).length === 0) {
    log("No benchmark results captured", "red");
    process.exit(1);
  }

  log(`Captured ${Object.keys(current).length} benchmark results`, "green");

  // Compare
  const comparison = compareResults(previous, current);
  report(comparison);

  // Save if requested
  if (shouldSave || !previous) {
    const results = {
      timestamp: new Date().toISOString(),
      version: JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8"))
        .version,
      benchmarks: current,
    };
    saveResults(results);
    log(`\n💾 Results saved to benchmark-results.json`, "cyan");
  }
}

main();
