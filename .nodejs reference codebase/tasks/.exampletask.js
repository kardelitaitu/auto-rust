/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Task template based on cookiebot.js
 * @module tasks/exampleTask
 */

import { api } from '../api/index.js';
import { createLogger } from '../api/core/logger.js';
import fs from 'fs/promises';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const CONFIG = {
    // File paths
    sitesFile: '../config/popularsite.txt',

    // Timeouts (ms)
    taskTimeoutMs: 50000,
    navigationTimeout: 30000,
    responsivenessTimeout: 5000,

    // Loop settings
    loopCountMin: 5,
    loopCountMax: 10,

    // Scroll settings
    scrollPausesMin: 4,
    scrollPausesMax: 8,
    scrollAmountMin: 300,
    scrollAmountMax: 600,

    // Delays (ms)
    postLoadDelay: 1000,
    postScrollDelay: 1000,
};

const SITES_FILE = path.join(__dirname, CONFIG.sitesFile);

let urls = [];

// Read the list of sites from the external file when the module is first loaded.
try {
    const data = await fs.readFile(SITES_FILE, 'utf8');
    urls = data
        .split('\n')
        .map((line) => {
            let url = line.trim();
            if (url.startsWith('http_')) {
                url = url.replace('http_', 'http:');
            }
            if (url.startsWith('https_')) {
                url = url.replace('https_', 'https:');
            }
            return url;
        })
        .filter((line) => line.startsWith('http'));

    if (urls.length === 0) {
        console.error(
            '[exampleTask.js] Warning: sites file was read, but it is empty or contains no valid URLs.'
        );
    }
} catch (error) {
    console.error(
        `[exampleTask.js] CRITICAL: Failed to read site list from ${SITES_FILE}. The task will not be able to run. Error: ${error.message}`
    );
}

/**
 * An automation task that navigates to a random URL.
 * @param {object} page - The Playwright page object.
 * @param {object} payload - The payload data for the task.
 * @param {string} payload.browserInfo - A unique identifier for the browser.
 */
export default async function exampleTask(page, payload) {
    const startTime = process.hrtime.bigint();
    const browserInfo = payload.browserInfo || 'unknown_profile';
    const logger = createLogger(`exampleTask.js [${browserInfo}]`);

    try {
        // Use a Promise.race to enforce a global timeout for the "work" phase
        await Promise.race([
            api.withPage(page, async () => {
                await api.init(page, {
                    logger,
                    lite: false,
                    blockNotifications: true,
                    blockDialogs: true,
                    autoBanners: false, //auto click cookies popup banner
                    muteAudio: true,
                });

                logger.info(`URL list size: ${urls.length}`);
                if (urls.length === 0) {
                    logger.error('URL list is empty or failed to load. Aborting task.');
                    return;
                }

                const loopCount = api.randomInRange(CONFIG.loopCountMin, CONFIG.loopCountMax);
                logger.info(`Starting random visits loop for ${loopCount} times.`);

                const abortSignal = payload.abortSignal;

                for (let i = 0; i < loopCount; i++) {
                    if (page.isClosed() || abortSignal?.aborted) break;

                    const randomUrl = urls[Math.floor(Math.random() * urls.length)];
                    logger.info(`(${i + 1} of ${loopCount}) Navigating to: ${randomUrl}`);

                    try {
                        // 1. Navigate with a timeout
                        await api.goto(randomUrl, {
                            waitUntil: 'domcontentloaded',
                            timeout: CONFIG.navigationTimeout,
                        });

                        // 2. Check responsiveness
                        try {
                            await api.waitFor(
                                async () => {
                                    return await api.eval(() => true).catch(() => false);
                                },
                                { timeout: CONFIG.responsivenessTimeout }
                            );
                        } catch (_e) {
                            logger.warn(`Page ${randomUrl} is unresponsive after load. Skipping.`);
                            continue;
                        }

                        await api.wait(CONFIG.postLoadDelay);

                        // 3. Scroll/Read
                        await api.scroll.read(null, {
                            pauses: api.randomInRange(
                                CONFIG.scrollPausesMin,
                                CONFIG.scrollPausesMax
                            ),
                            scrollAmount: api.randomInRange(
                                CONFIG.scrollAmountMin,
                                CONFIG.scrollAmountMax
                            ),
                        });
                        await api.wait(CONFIG.postScrollDelay);
                    } catch (navError) {
                        if (
                            navError.message.includes('interrupted by another navigation') ||
                            navError.message.includes('Session closed') ||
                            navError.message.includes('Page has been closed')
                        ) {
                            logger.warn(
                                `Navigation to ${randomUrl} was interrupted (or page closed).`
                            );
                            break;
                        } else if (
                            navError.message.includes('timeout') ||
                            navError.message.includes('Timeout')
                        ) {
                            logger.warn(`Visit to ${randomUrl} timed out. Skipping to next.`);
                        } else if (navError.message.includes('net::ERR_')) {
                            logger.warn(
                                `Network error visiting ${randomUrl}: ${navError.message.split('\n')[0]}`
                            );
                        } else {
                            logger.error(`Failed to load ${randomUrl}:`, `page is not responding`);
                            if (page.isClosed()) break;
                        }
                    }
                }
            }),
            new Promise((_, reject) =>
                setTimeout(
                    () => reject(new Error(`exampleTask exceeded ${CONFIG.taskTimeoutMs}ms limit`)),
                    CONFIG.taskTimeoutMs
                )
            ),
        ]);
    } catch (error) {
        if (error.message.includes('exceeded') && error.message.includes('limit')) {
            logger.warn(`[exampleTask] Task forced to stop: ${error.message}`);
        } else if (error.message.includes('Target page, context or browser has been closed')) {
            logger.warn(`Task interrupted: Browser/Page closed.`);
        } else {
            logger.error(`### CRITICAL ERROR in main task loop:`, error);
        }
    } finally {
        try {
            if (page && !page.isClosed()) {
                await page.close();
                logger.debug(`Page closed successfully.`);
            }
        } catch (closeError) {
            logger.warn(`Error closing page: ${closeError.message}`);
        }
        const endTime = process.hrtime.bigint();
        const durationInSeconds = (Number(endTime - startTime) / 1_000_000_000).toFixed(2);
        logger.info(`[exampleTask] Total task duration: ${durationInSeconds} seconds`);
    }
}
