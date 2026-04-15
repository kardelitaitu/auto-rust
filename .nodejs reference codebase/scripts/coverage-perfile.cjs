#!/usr/bin/env node
/**
 * Per-File Coverage Runner
 * Shows source files with >0% coverage for each test
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const projectRoot = process.cwd();
const testDir = path.join(projectRoot, 'api/tests/unit');
const configPath = path.join(projectRoot, 'config/vitest.config.isolated.js');
const logFile = path.join(__dirname, 'coverage-perfile.log');

// Clear previous log
fs.writeFileSync(logFile, `=== Per-File Coverage Analysis ===\nStarted: ${new Date().toISOString()}\n\n`);

// Log to both console and file
const log = (msg = '') => {
    console.log(msg);
    const cleanMsg = msg.replace(/\x1b\[[0-9;]*m/g, ''); // eslint-disable-line no-control-regex
    fs.appendFileSync(logFile, cleanMsg + '\n');
};

log('\n=== Per-File Coverage Analysis ===\n');

function findTestFiles(dir, files = []) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory() && !['node_modules', 'edge-cases'].includes(entry.name)) {
            findTestFiles(full, files);
        } else if (entry.name.endsWith('.test.js') && !entry.name.includes('twitter')) {
            files.push(full);
        }
    }
    return files;
}

const testFiles = findTestFiles(testDir);
log(`Found ${testFiles.length} test files\n`);

const results = [];
let skipped = 0; // Tests that mock all dependencies
let stubs = 0;   // Tests with no source file
const startTime = Date.now();

log('Test'.padEnd(38) + 'Source'.padEnd(25) + 'Stmts'.padStart(8) + 'Branch'.padStart(9) + 'Funcs'.padStart(8) + 'Lines'.padStart(8));
log('-'.repeat(100));

for (const file of testFiles) {
    const testName = path.basename(file);
    
    try {
        const output = execSync(
            `npx vitest run --config "${configPath}" --coverage --reporter=dot "${file}"`,
            { cwd: projectRoot, stdio: 'pipe', timeout: 60000, shell: 'powershell.exe', env: { ...process.env, FORCE_COLOR: '0' } }
        ).toString();
        
        // Parse: find lines with coverage data (contains | and numbers)
        // Format: "  ...filename.js |     100 |      100 |     100 |     100 |"
        const covered = [];
        for (const line of output.split('\n')) {
            const m = line.match(/\s+([\w-]+\.js)\s+\|\s+([\d.]+|-)\s+\|\s+([\d.]+|-)\s+\|\s+([\d.]+|-)\s+\|\s+([\d.]+|-)/);
            if (m && !m[1].includes('test') && !m[1].includes('spec') && parseFloat(m[2]) > 0) {
                covered.push({ file: m[1], stmts: m[2], branch: m[3], funcs: m[4], lines: m[5] });
            }
        }
        
        if (covered.length > 0) {
            // Show best match
            const best = covered.sort((a, b) => parseFloat(b.stmts) - parseFloat(a.stmts))[0];
            log(`${testName.substring(0, 36).padEnd(38)}${best.file.substring(0, 23).padEnd(25)}${best.stmts.padStart(8)}${best.branch.padStart(9)}${best.funcs.padStart(8)}${best.lines.padStart(8)}`);
            results.push({ test: testName, source: best.file, stmts: parseFloat(best.stmts), branch: parseFloat(best.branch), funcs: parseFloat(best.funcs), lines: parseFloat(best.lines) });
        } else if (output.includes('|') && !output.includes('FAIL')) {
            // Has coverage output but all source files at 0% (mocks everything)
            log(`${testName.substring(0, 36).padEnd(38)}(mocked)`);
            skipped++;
        } else {
            // No source file exists
            log(`${testName.substring(0, 36).padEnd(38)}(stub test)`);
            stubs++;
        }
    } catch (e) {
        log(`${testName.substring(0, 36).padEnd(38)}FAIL`);
    }
    
    // Progress every 25 files
    if ((testFiles.indexOf(file) + 1) % 25 === 0) {
        log(`  ... ${testFiles.indexOf(file) + 1} files processed`);
    }
}

// Summary
const duration = ((Date.now() - startTime) / 1000).toFixed(1);
const avg = (key) => results.length ? (results.reduce((s, r) => s + r[key], 0) / results.length).toFixed(1) : 0;

log('-'.repeat(100));
log(`${'AVERAGE'.padEnd(38)}${''.padEnd(25)}${avg('stmts').padStart(8)}${avg('branch').padStart(9)}${avg('funcs').padStart(8)}${avg('lines').padStart(8)}`);

log('\n' + '='.repeat(60));
log(`Total: ${testFiles.length}`);
log(`  With Coverage: ${results.length} (${((results.length/testFiles.length)*100).toFixed(1)}%)`);
log(`  Mocked (all deps): ${skipped}`);
log(`  Stub tests (no source): ${stubs}`);
log(`  Failed: ${testFiles.length - results.length - skipped - stubs}`);
log(`\nAvg: Stmts ${avg('stmts')}% | Branch ${avg('branch')}% | Funcs ${avg('funcs')}% | Lines ${avg('lines')}%`);
log(`Duration: ${duration}s`);
log(`Finished: ${new Date().toISOString()}`);
log('='.repeat(60));

console.log(`\nLog saved to: ${logFile}`);
