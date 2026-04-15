#!/usr/bin/env node
/**
 * Remove math.js mocks from test files
 * math.js now has seeded random via vitest.setup.js
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

// Pattern to match math.js mock block (multi-line)
const mathMockPattern = /vi\.mock\("[^"]*math\.js", \(\) => \(\{[\s\S]*?\}\)\);[\n]*/g;
// eslint-disable-next-line no-useless-escape
const mathMockPattern2 = /vi\.mock\('[^\']*math\.js', \(\) => \(\{[\s\S]*?\}\)\);[\n]*/g;

let modifiedFiles = 0;

const testFiles = findTestFiles(testDir);

console.log(`\n=== Removing Math Mocks ===\n`);

for (const file of testFiles) {
    let content = fs.readFileSync(file, 'utf8');
    const originalContent = content;
    
    content = content.replace(mathMockPattern, '');
    content = content.replace(mathMockPattern2, '');
    
    if (content !== originalContent) {
        fs.writeFileSync(file, content);
        modifiedFiles++;
        console.log(`✓ ${path.basename(file)}`);
    }
}

console.log(`\n=== Summary ===`);
console.log(`Modified: ${modifiedFiles} files`);
console.log(`Math.js now uses seeded random from vitest.setup.js`);
