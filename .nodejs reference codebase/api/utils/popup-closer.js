/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { createLogger } from '../core/logger.js';
import { setSessionInterval, clearSessionInterval } from '../core/context.js';

class PopupCloser {
    constructor(page, logger, options = {}) {
        this.page = page;
        this.logger = logger || createLogger('popup-closer.js');
        this.intervalMs = 120000;
        this.timer = null;
        this.lastClosedAt = Date.now();
        this.nextNotifyMinutes = 2;
        this.lock = options.lock;
        this.signal = options.signal;
        this.shouldSkip = options.shouldSkip;
        this.running = false;
        this.api = options.api;
    }

    start() {
        if (this.timer) return;
        this.timer = setSessionInterval(
            'popup_closer',
            () => {
                this.runOnce().catch(() => {});
            },
            this.intervalMs
        );
    }

    stop() {
        clearSessionInterval('popup_closer');
        this.timer = null;
    }

    async runOnce() {
        if (!this.page || this.page.isClosed()) return;
        if (this.api?.isSessionActive && !this.api.isSessionActive()) return;
        if (this.signal?.aborted) return;
        if (this.shouldSkip?.()) return;
        if (this.running) return;
        this.running = true;
        try {
            if (this.lock) {
                return await this.lock(async () => this._runOnceInternal());
            }
            return await this._runOnceInternal();
        } finally {
            this.running = false;
        }
    }

    async _runOnceInternal() {
        if (!this.page || this.page.isClosed()) return;
        if (this.signal?.aborted) return;
        try {
            const selectors = [
                'button:has-text("Keep less relevant ads")',
                '[role="button"]:has-text("Keep less relevant ads")',
            ];
            for (const selector of selectors) {
                const exists = await this.api.exists(selector).catch(() => false);
                if (!exists) continue;
                await this.api.click(selector);
                this.lastClosedAt = Date.now();
                this.nextNotifyMinutes = 2;
                this.logger.info('[popup-closer] Popup closed');
                return true;
            }
            return false;
        } catch (e) {
            this.logger.debug(`[popup-closer] ${e.message}`);
            return false;
        }
    }
}

export default PopupCloser;
