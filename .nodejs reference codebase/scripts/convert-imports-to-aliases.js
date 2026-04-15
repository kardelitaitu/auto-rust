/**
 * Script to convert relative imports to alias imports in test files.
 * Run with: node scripts/convert-imports-to-aliases.js
 */

import { readdirSync, readFileSync, writeFileSync, statSync } from 'fs';
import { join, relative, resolve, dirname } from 'path';

const TESTS_DIR = resolve('api/tests');
const PROJECT_ROOT = resolve('.');

// Alias mappings: prefix in resolved path → alias
const ALIAS_MAP = [
    { prefix: 'api/', alias: '@api/' },
    { prefix: 'tasks/', alias: '@tasks/' },
    { prefix: 'api/tests/', alias: '@tests/' },
];

// Files to skip (same-directory relative imports should stay)
function shouldConvert(importPath, testFileDir) {
    // Skip same-directory imports (./something)
    if (importPath.startsWith('./')) {
        return false;
    }
    // Skip imports that don't start with ../
    if (!importPath.startsWith('../')) {
        return false;
    }
    return true;
}

function resolveImportPath(importPath, testFileDir) {
    // Resolve the relative path from the test file's directory
    const resolved = resolve(testFileDir, importPath);
    // Get path relative to project root
    return relative(PROJECT_ROOT, resolved).replace(/\\/g, '/');
}

function convertToAlias(relativePath) {
    for (const { prefix, alias } of ALIAS_MAP) {
        if (relativePath.startsWith(prefix)) {
            return alias + relativePath.slice(prefix.length);
        }
    }
    return null;
}

// Regex to match import statements and vi.mock calls with relative paths
// Matches: import ... from '...' and import ... from "..."
const IMPORT_REGEX = /from\s+['"](\.{1,2}\/[^'"]+)['"]/g;
// Matches: vi.mock('...', ...) and vi.mock("...", ...)
const MOCK_REGEX = /vi\.mock\(\s*['"](\.{1,2}\/[^'"]+)['"]/g;

function replacePath(content, importPath, testFileDir, regex) {
    if (!shouldConvert(importPath, testFileDir)) return content;

    const resolvedPath = resolveImportPath(importPath, testFileDir);
    const aliasPath = convertToAlias(resolvedPath);

    if (!aliasPath) return content;

    const newImportPath = aliasPath.endsWith('.js') ? aliasPath : aliasPath + '.js';

    // Replace with single quotes
    const oldStr = `'${importPath}'`;
    const newStr = `'${newImportPath}'`;
    if (content.includes(oldStr)) {
        return content.split(oldStr).join(newStr);
    }

    // Replace with double quotes
    const oldStrDouble = `"${importPath}"`;
    const newStrDouble = `"${newImportPath}"`;
    if (content.includes(oldStrDouble)) {
        return content.split(oldStrDouble).join(newStrDouble);
    }

    return content;
}

function processFile(filePath) {
    const content = readFileSync(filePath, 'utf8');
    const testFileDir = dirname(filePath);
    let modified = false;
    let newContent = content;

    // Process import statements
    const importMatches = [...content.matchAll(IMPORT_REGEX)];
    for (const match of importMatches) {
        const newContent2 = replacePath(newContent, match[1], testFileDir, 'import');
        if (newContent2 !== newContent) {
            newContent = newContent2;
            modified = true;
        }
    }

    // Process vi.mock calls
    const mockMatches = [...content.matchAll(MOCK_REGEX)];
    for (const match of mockMatches) {
        const newContent2 = replacePath(newContent, match[1], testFileDir, 'mock');
        if (newContent2 !== newContent) {
            newContent = newContent2;
            modified = true;
        }
    }

    if (modified) {
        writeFileSync(filePath, newContent, 'utf8');
        const relPath = relative(PROJECT_ROOT, filePath);
        console.log(`✓ ${relPath}`);
    }

    return modified;
}

function getAllTestFiles(dir) {
    const files = [];
    const entries = readdirSync(dir, { withFileTypes: true });

    for (const entry of entries) {
        const fullPath = join(dir, entry.name);
        if (entry.isDirectory()) {
            files.push(...getAllTestFiles(fullPath));
        } else if (entry.name.endsWith('.test.js')) {
            files.push(fullPath);
        }
    }

    return files;
}

// Main
console.log('Converting relative imports to alias imports...\n');
const testFiles = getAllTestFiles(TESTS_DIR);
let convertedCount = 0;

for (const file of testFiles) {
    if (processFile(file)) {
        convertedCount++;
    }
}

console.log(`\nDone! Converted ${convertedCount} files out of ${testFiles.length} test files.`);
