import coverage from "./api/coverage/coverage-final.json";

const results = [];

for (const [filePath, fileCoverage] of Object.entries(coverage)) {
  const totalStatements = fileCoverage.s
    ? Object.keys(fileCoverage.s).length
    : 0;
  const coveredStatements = fileCoverage.s
    ? Object.values(fileCoverage.s).filter((s) => s > 0).length
    : 0;
  const statementCoverage =
    totalStatements > 0
      ? ((coveredStatements / totalStatements) * 100).toFixed(2)
      : 0;

  const totalBranches = fileCoverage.b ? Object.keys(fileCoverage.b).length : 0;
  const coveredBranches = fileCoverage.b
    ? Object.values(fileCoverage.b).filter((b) => b > 0).length
    : 0;
  const branchCoverage =
    totalBranches > 0
      ? ((coveredBranches / totalBranches) * 100).toFixed(2)
      : 0;

  const totalFunctions = fileCoverage.f
    ? Object.keys(fileCoverage.f).length
    : 0;
  const coveredFunctions = fileCoverage.f
    ? Object.values(fileCoverage.f).filter((f) => f > 0).length
    : 0;
  const functionCoverage =
    totalFunctions > 0
      ? ((coveredFunctions / totalFunctions) * 100).toFixed(2)
      : 0;

  const totalLines = fileCoverage.l ? Object.keys(fileCoverage.l).length : 0;
  const coveredLines = fileCoverage.l
    ? Object.values(fileCoverage.l).filter((l) => l > 0).length
    : 0;
  const lineCoverage =
    totalLines > 0 ? ((coveredLines / totalLines) * 100).toFixed(2) : 0;

  if (totalStatements > 0) {
    results.push({
      file: filePath.replace("C:\\My Script\\auto-ai\\", ""),
      totalStatements,
      coveredStatements,
      statementCoverage: parseFloat(statementCoverage),
      totalBranches,
      coveredBranches,
      branchCoverage: parseFloat(branchCoverage),
      totalFunctions,
      coveredFunctions,
      functionCoverage: parseFloat(functionCoverage),
      totalLines,
      coveredLines,
      lineCoverage: parseFloat(lineCoverage),
    });
  }
}

// Sort by statement coverage (lowest first)
results.sort((a, b) => a.statementCoverage - b.statementCoverage);

console.log("\n=== LOWEST COVERAGE FILES (Top 50) ===\n");
console.log(
  "File".padEnd(60) +
    "Stmts".padEnd(8) +
    "Cov%".padEnd(6) +
    "Branches".padEnd(8) +
    "Cov%".padEnd(6) +
    "Funcs".padEnd(8) +
    "Cov%".padEnd(6) +
    "Lines".padEnd(8) +
    "Cov%",
);
console.log("-".repeat(120));

results.slice(0, 50).forEach((r) => {
  console.log(
    r.file.padEnd(60).slice(0, 60) +
      r.totalStatements.toString().padEnd(8) +
      r.statementCoverage.toFixed(1).padStart(6) +
      r.totalBranches.toString().padEnd(8) +
      r.branchCoverage.toFixed(1).padStart(6) +
      r.totalFunctions.toString().padEnd(8) +
      r.functionCoverage.toFixed(1).padStart(6) +
      r.totalLines.toString().padEnd(8) +
      r.lineCoverage.toFixed(1).padStart(6),
  );
});

// Summary by directory
const dirStats = {};
results.forEach((r) => {
  const dir = r.file.split("/")[0] || "root";
  if (!dirStats[dir]) {
    dirStats[dir] = { files: 0, totalStmts: 0, coveredStmts: 0 };
  }
  dirStats[dir].files++;
  dirStats[dir].totalStmts += r.totalStatements;
  dirStats[dir].coveredStmts += r.coveredStatements;
});

console.log("\n=== COVERAGE BY DIRECTORY ===\n");
console.log(
  "Directory".padEnd(25) +
    "Files".padEnd(8) +
    "TotalStmts".padEnd(12) +
    "CoveredStmts".padEnd(14) +
    "Coverage%",
);
console.log("-".repeat(75));
Object.entries(dirStats)
  .sort(
    (a, b) =>
      (b[1].coveredStmts / b[1].totalStmts || 0) -
      (a[1].coveredStmts / a[1].totalStmts || 0),
  )
  .forEach(([dir, stats]) => {
    const cov = ((stats.coveredStmts / stats.totalStmts) * 100).toFixed(2);
    console.log(
      dir.padEnd(25) +
        stats.files.toString().padEnd(8) +
        stats.totalStmts.toString().padEnd(12) +
        stats.coveredStmts.toString().padEnd(14) +
        cov,
    );
  });
