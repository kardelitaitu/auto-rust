/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { createLogger } from '../api/core/logger.js';
import { verifyGhostOnPage } from '../verify-ghost-anybrowser.js';

export default async function verifyGhostAnyBrowserTask(page, payload = {}) {
    const logger = createLogger('verify-ghost-anybrowser.js');

    const timeoutMsRaw = payload.timeoutMs ?? process.env.VERIFY_TIMEOUT_MS;
    const timeoutMs = timeoutMsRaw ? Number(timeoutMsRaw) : 20000;

    logger.info(
        `[verify-ghost-anybrowser.js] Starting Sannysoft verification (timeoutMs=${timeoutMs})...`
    );

    const result = await verifyGhostOnPage(page, { timeoutMs, manageExitCode: false });

    logger.info(
        `[verify-ghost-anybrowser.js] Result ok=${result.ok} failedCount=${result.failedCount}`
    );

    if (!result.ok) {
        const failedPreview = (result.failedRows || []).slice(0, 10);
        logger.error(
            `[verify-ghost-anybrowser.js] Failed rows (first 10): ${JSON.stringify(failedPreview)}`
        );
        throw new Error(`verify-ghost-anybrowser failed: ${result.failedCount} failed checks`);
    }

    return result;
}
