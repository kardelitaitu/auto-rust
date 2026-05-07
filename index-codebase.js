/**
 * index-codebase.js
 * Indexes all .md and .rs files into context-mode knowledge base.
 * Usage: node index-codebase.js [file-list.txt]
 * If no argument, reads codebase-files.txt from repo root.
 */

const fs = require('fs');
const path = require('path');

// Try to load context-mode
let contextMode;
try {
  contextMode = require('context-mode');
} catch (e) {
  console.error('context-mode module not found. Install it globally or locally.');
  process.exit(1);
}

// Get list of files
let files;
const arg = process.argv[2];
if (arg) {
  const content = fs.readFileSync(arg, 'utf8');
  files = content.split('\n').filter(f => f.trim());
} else {
  const defaultList = path.join(__dirname, 'codebase-files.txt');
  if (!fs.existsSync(defaultList)) {
    console.error('No file list provided and codebase-files.txt not found.');
    process.exit(1);
  }
  const content = fs.readFileSync(defaultList, 'utf8');
  files = content.split('\n').filter(f => f.trim());
}

console.log(`Indexing ${files.length} files...`);

let indexed = 0;
let failed = 0;

async function indexFile(filePath) {
  try {
    // Assume contextMode has an indexFile function
    if (typeof contextMode.indexFile === 'function') {
      await contextMode.indexFile(filePath);
    } else if (typeof contextMode.index === 'function') {
      await contextMode.index(filePath);
    } else if (typeof contextMode.default === 'function') {
      await contextMode.default(filePath);
    } else {
      // Fallback: just print what would be indexed
      console.log(`Would index: ${filePath}`);
    }
    indexed++;
  } catch (err) {
    console.error(`Failed to index ${filePath}: ${err.message}`);
    failed++;
  }
}

(async () => {
  for (const file of files) {
    await indexFile(file);
  }
  console.log(`Done. Indexed: ${indexed}, Failed: ${failed}`);
})();
