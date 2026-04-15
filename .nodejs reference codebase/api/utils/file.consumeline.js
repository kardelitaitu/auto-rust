/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Read and remove a random line from a text file with concurrency protection.
 * @module api/utils/file.consumeline
 */

import fs from 'fs/promises';
import path from 'path';

/**
 * Read and remove a random non-empty line from a text file.
 * Implements a simple file-based lock to prevent race conditions.
 *
 * @param {string} filePath - Path to the txt file
 * @param {object} [options]
 * @param {number} [options.maxRetries=20] - Max retries for lock acquisition
 * @param {number} [options.retryDelay=50] - Delay between retries in ms
 * @returns {Promise<string|null>} Consumed line or null if empty/missing
 * @example
 * const line = await api.file.consumeline('data.txt');
 */
export async function consumeline(filePath, options = {}) {
    const { maxRetries = 20, retryDelay = 50 } = options;
    const lockFile = `${filePath}.lock`;
    const fullPath = path.resolve(filePath);

    let acquiredLock = false;

    for (let attempt = 0; attempt < maxRetries; attempt++) {
        try {
            // Try to create lock file (wx flag fails if file exists)
            await fs.writeFile(lockFile, process.pid.toString(), { flag: 'wx' });
            acquiredLock = true;
            break;
        } catch (e) {
            if (e.code === 'EEXIST') {
                // Wait and retry
                await new Promise((r) => setTimeout(r, retryDelay));
                continue;
            }
            // If it's another error (like directory missing), just fail
            return null;
        }
    }

    if (!acquiredLock) {
        // Could not get lock, return null to be safe
        return null;
    }

    try {
        const data = await fs.readFile(fullPath, 'utf8');
        const lines = data.split(/\r?\n/).filter((line) => line.trim() !== '');

        if (lines.length === 0) {
            await fs.unlink(lockFile);
            return null;
        }

        const index = Math.floor(Math.random() * lines.length);
        const line = lines[index];

        lines.splice(index, 1);
        await fs.writeFile(fullPath, lines.join('\n'), 'utf8');

        return line;
    } catch (_error) {
        return null;
    } finally {
        // Always cleanup lock
        if (acquiredLock) {
            try {
                await fs.unlink(lockFile);
            } catch {
                /* ignore */
            }
        }
    }
}

export default consumeline;
