#!/usr/bin/env node
/**
 * Remove logger mocks from test files
 * Replaces vi.mock("...logger.js") blocks with nothing (logger is silenced via vitest.setup.js)
 */

const fs = require('fs');
const path = require('path');

const testDir = path.join(process.cwd(), 'api/tests/unit');

function findTestFiles(dir, files = []) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory() && !['node_modules', 'edge-cases'].includes(entry.name)) {
            findTestFiles(full, files);
        } else if (entry.name.endsWith('.test.js')) {
            files.push(full);
        }
    }
    return files;
}

// Pattern to match logger mock block (multi-line)
const loggerMockPattern = /vi\.mock\("[^"]*logger\.js", \(\) => \(\{[\s\S]*?\}\)\);[\n]*/g;

let totalFiles = 0;
let modifiedFiles = 0;

const testFiles = findTestFiles(testDir);

console.log(`\n=== Removing Logger Mocks ===\n`);
console.log(`Found ${testFiles.length} test files\n`);

for (const file of testFiles) {
    let content = fs.readFileSync(file, 'utf8');
    const originalContent = content;
    
    // Remove logger mock
    content = content.replace(loggerMockPattern, '');
    
    // Also check for the pattern with different quotes
    // eslint-disable-next-line no-useless-escape
    const loggerMockPattern2 = /vi\.mock\('[^\']*logger\.js', \(\) => \(\{[\s\S]*?\}\)\);[\n]*/g;
    content = content.replace(loggerMockPattern2, '');
    
    if (content !== originalContent) {
        fs.writeFileSync(file, content);
        modifiedFiles++;
        console.log(`✓ ${path.basename(file)}`);
    }
    totalFiles++;
}

console.log(`\n=== Summary ===`);
console.log(`Scanned: ${totalFiles} files`);
console.log(`Modified: ${modifiedFiles} files`);
console.log(`Removed logger mocks that are now handled by vitest.setup.js`);
