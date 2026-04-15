/**
 * Coverage Badge Generator
 * Reads coverage-final.json and generates an SVG badge
 *
 * Usage: node scripts/generate-coverage-badge.js
 */

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");
const coverageFile = path.join(rootDir, "api/coverage/coverage-final.json");
const badgeFile = path.join(rootDir, "coverage-badge.svg");

// Color thresholds for badge
function getColor(percentage) {
  if (percentage >= 90) return "brightgreen";
  if (percentage >= 80) return "green";
  if (percentage >= 70) return "yellowgreen";
  if (percentage >= 60) return "yellow";
  if (percentage >= 50) return "orange";
  return "red";
}

// Generate SVG badge
function generateBadge(percentage) {
  const color = getColor(percentage);
  const text = `${percentage.toFixed(1)}%`;
  const textWidth = text.length * 7 + 10;
  const totalWidth = 70 + textWidth;

  return `<svg xmlns="http://www.w3.org/2000/svg" width="${totalWidth}" height="20" role="img" aria-label="coverage: ${text}">
  <title>coverage: ${text}</title>
  <linearGradient id="s" x2="0" y2="100%">
    <stop offset="0" stop-color="#bbb" stop-opacity=".1"/>
    <stop offset="1" stop-opacity=".1"/>
  </linearGradient>
  <clipPath id="r">
    <rect width="${totalWidth}" height="20" rx="3" fill="#fff"/>
  </clipPath>
  <g clip-path="url(#r)">
    <rect width="70" height="20" fill="#555"/>
    <rect x="70" width="${textWidth}" height="20" fill="${color}"/>
    <rect width="${totalWidth}" height="20" fill="url(#s)"/>
  </g>
  <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" text-rendering="geometricPrecision" font-size="11">
    <text aria-hidden="true" x="35" y="15" fill="#010101" fill-opacity=".3">coverage</text>
    <text x="35" y="14" fill="#fff">coverage</text>
    <text aria-hidden="true" x="${70 + textWidth / 2}" y="15" fill="#010101" fill-opacity=".3">${text}</text>
    <text x="${70 + textWidth / 2}" y="14" fill="#fff">${text}</text>
  </g>
</svg>`;
}

// Calculate coverage from Istanbul JSON
function calculateCoverage() {
  if (!fs.existsSync(coverageFile)) {
    console.error(`Coverage file not found: ${coverageFile}`);
    console.log(
      'Run "pnpm run test:bun:coverage" first to generate coverage data.',
    );
    process.exit(1);
  }

  const coverageData = JSON.parse(fs.readFileSync(coverageFile, "utf8"));

  let totalStatements = 0;
  let coveredStatements = 0;
  let totalBranches = 0;
  let coveredBranches = 0;
  let totalFunctions = 0;
  let coveredFunctions = 0;
  let totalLines = 0;
  let coveredLines = 0;

  for (const [filePath, fileCoverage] of Object.entries(coverageData)) {
    // Skip test files
    if (filePath.includes(".test.js") || filePath.includes(".spec.js"))
      continue;

    // Statements
    const statements = fileCoverage.s || {};
    totalStatements += Object.keys(statements).length;
    coveredStatements += Object.values(statements).filter((c) => c > 0).length;

    // Functions
    const functions = fileCoverage.f || {};
    totalFunctions += Object.keys(functions).length;
    coveredFunctions += Object.values(functions).filter((c) => c > 0).length;

    // Branches
    const branches = fileCoverage.b || {};
    for (const branchHits of Object.values(branches)) {
      totalBranches += branchHits.length;
      coveredBranches += branchHits.filter((c) => c > 0).length;
    }

    // Lines (approximate from statements)
    totalLines += Object.keys(statements).length;
    coveredLines += Object.values(statements).filter((c) => c > 0).length;
  }

  const statementPercentage =
    totalStatements > 0 ? (coveredStatements / totalStatements) * 100 : 0;
  const branchPercentage =
    totalBranches > 0 ? (coveredBranches / totalBranches) * 100 : 0;
  const functionPercentage =
    totalFunctions > 0 ? (coveredFunctions / totalFunctions) * 100 : 0;
  const linePercentage = totalLines > 0 ? (coveredLines / totalLines) * 100 : 0;

  // Overall coverage (average of all metrics)
  const overallPercentage =
    (statementPercentage +
      branchPercentage +
      functionPercentage +
      linePercentage) /
    4;

  return {
    statements: {
      total: totalStatements,
      covered: coveredStatements,
      percentage: statementPercentage,
    },
    branches: {
      total: totalBranches,
      covered: coveredBranches,
      percentage: branchPercentage,
    },
    functions: {
      total: totalFunctions,
      covered: coveredFunctions,
      percentage: functionPercentage,
    },
    lines: {
      total: totalLines,
      covered: coveredLines,
      percentage: linePercentage,
    },
    overall: overallPercentage,
  };
}

// Main
console.log("Generating coverage badge...\n");

const coverage = calculateCoverage();

console.log("Coverage Summary:");
console.log(
  `  Statements: ${coverage.statements.percentage.toFixed(1)}% (${coverage.statements.covered}/${coverage.statements.total})`,
);
console.log(
  `  Branches:   ${coverage.branches.percentage.toFixed(1)}% (${coverage.branches.covered}/${coverage.branches.total})`,
);
console.log(
  `  Functions:  ${coverage.functions.percentage.toFixed(1)}% (${coverage.functions.covered}/${coverage.functions.total})`,
);
console.log(
  `  Lines:      ${coverage.lines.percentage.toFixed(1)}% (${coverage.lines.covered}/${coverage.lines.total})`,
);
console.log(`  ─────────────────────────────`);
console.log(`  Overall:    ${coverage.overall.toFixed(1)}%\n`);

const badge = generateBadge(coverage.overall);
fs.writeFileSync(badgeFile, badge);

console.log(`Badge saved to: ${badgeFile}`);
console.log("\nAdd this line to your README.md:");
console.log(`![Coverage](./coverage-badge.svg)`);
