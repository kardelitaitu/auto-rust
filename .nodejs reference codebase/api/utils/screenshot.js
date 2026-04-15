/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Screenshot utility for automation tasks.
 * Re-implemented to support tasks after project refactoring.
 * @module api/utils/screenshot
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { createLogger } from '../core/logger.js';

const logger = createLogger('api.screenshot');

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SCREENSHOT_DIR = path.resolve(__dirname, '../../screenshots');

/**
 * Takes a screenshot of the current page.
 * @param {object} page - Playwright page instance
 * @param {string} [sessionName='default'] - Name for the screenshot file
 * @param {string} [suffix=''] - Optional suffix for the filename
 * @returns {Promise<string|null>} Path to the saved screenshot or null if failed
 */
export async function takeScreenshot(page, sessionName = 'default', suffix = '') {
    try {
        if (!fs.existsSync(SCREENSHOT_DIR)) {
            fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });
        }

        // Sanitize session name
        const sanitizedSession = sessionName.replace(/[:/<>|?*]/g, '-');
        const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
        const filename = `${sanitizedSession}${suffix}-${timestamp}.jpg`;
        const filePath = path.join(SCREENSHOT_DIR, filename);

        await page.screenshot({
            path: filePath,
            type: 'jpeg',
            quality: 30,
            fullPage: false,
        });
        logger.info(`Screenshoted '${filename}'`);
        logger.debug(`Screenshot saved to: ${filePath}`);
        return filePath;
    } catch (error) {
        logger.error(`Failed to take screenshot: ${error.message}`);
        return null;
    }
}

export default { takeScreenshot };
