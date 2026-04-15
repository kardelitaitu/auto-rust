/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { api } from '../index.js';
/**
 * Go Home Action
 * Handles navigation back to home feed
 * @module utils/actions/ai-twitter-go-home
 */

import { createLogger } from '../core/logger.js';

export class GoHomeAction {
    constructor(agent, _options = {}) {
        this.agent = agent;
        this.logger = createLogger('ai-twitter-go-home.js');

        this.stats = {
            attempts: 0,
            successes: 0,
            failures: 0,
        };

        this.loadConfig();
    }

    loadConfig() {
        if (this.agent?.twitterConfig?.actions?.goHome) {
            const actionConfig = this.agent.twitterConfig.actions.goHome;
            this.enabled = actionConfig.enabled !== false;
        } else {
            this.enabled = true;
        }
        this.logger.info(`[GoHomeAction] Initialized (enabled: ${this.enabled})`);
    }

    async canExecute(_context = {}) {
        if (!this.agent) {
            return { allowed: false, reason: 'agent_not_initialized' };
        }

        if (!this.enabled) {
            return { allowed: false, reason: 'action_disabled' };
        }

        return { allowed: true, reason: null };
    }

    async execute(_context = {}) {
        this.stats.attempts++;

        this.logger.info(`[GoHomeAction] Executing navigation to home`);

        try {
            if (this.agent?.scrollToGoldenZone && this.agent?.page) {
                const page = this.agent.page;
                const homeTarget = page
                    .locator('[data-testid="AppTabBar_Home_Link"], [aria-label="X"]')
                    .first();
                if (await api.visible(homeTarget).catch(() => false)) {
                    try {
                        await this.agent.scrollToGoldenZone(homeTarget);
                    } catch (_error) {
                        void _error;
                    }
                }
            }
            await this.agent.navigateHome();
            this.stats.successes++;

            const currentUrl = (await api.getCurrentUrl()) || 'unknown';

            this.logger.info(`[GoHomeAction] ✅ Returned to home: ${currentUrl}`);

            return {
                success: true,
                executed: true,
                reason: 'success',
                data: { url: currentUrl },
            };
        } catch (error) {
            this.stats.failures++;
            this.logger.error(`[GoHomeAction] Exception: ${error.message}`);

            return {
                success: false,
                executed: true,
                reason: 'exception',
                data: { error: error.message },
            };
        }
    }

    async tryExecute(context = {}) {
        const can = await this.canExecute(context);
        if (!can.allowed) {
            return {
                success: false,
                executed: false,
                reason: can.reason,
            };
        }

        return await this.execute(context);
    }

    getStats() {
        const total = this.stats.attempts;
        const successRate = total > 0 ? ((this.stats.successes / total) * 100).toFixed(1) : '0.0';

        return {
            attempts: this.stats.attempts,
            successes: this.stats.successes,
            failures: this.stats.failures,
            successRate: `${successRate}%`,
        };
    }

    resetStats() {
        this.stats = {
            attempts: 0,
            successes: 0,
            failures: 0,
        };
    }
}
