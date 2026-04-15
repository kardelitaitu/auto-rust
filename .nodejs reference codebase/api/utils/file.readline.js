/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Read a random line from a text file.
 * @module api/utils/file.readline
 */

import fs from 'fs/promises';

/**
 * Read a random non-empty line from a text file.
 * @param {string} filePath - Path to the txt file
 * @returns {Promise<string|null>} Random line or null if empty/missing
 * @example
 * const line = await api.file.readline('data.txt');
 */
export async function readline(filePath) {
    try {
        const data = await fs.readFile(filePath, 'utf8');
        const lines = data.split(/\r?\n/).filter((line) => line.trim() !== '');
        if (lines.length === 0) return null;
        return lines[Math.floor(Math.random() * lines.length)];
    } catch (_error) {
        return null;
    }
}

export default readline;
