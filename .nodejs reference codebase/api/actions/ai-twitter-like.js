/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Like Action
 * Handles liking tweets
 * @module utils/actions/ai-twitter-like
 */

import { createLogger } from '../core/logger.js';

export class LikeAction {
    constructor(agent, _options = {}) {
        this.agent = agent;
        this.logger = createLogger('ai-twitter-like.js');
        this.engagementType = 'likes';

        this.stats = {
            attempts: 0,
            successes: 0,
            failures: 0,
            skipped: 0,
        };

        this.loadConfig();
    }

    loadConfig() {
        if (this.agent?.twitterConfig?.actions?.like) {
            const actionConfig = this.agent.twitterConfig.actions.like;
            this.probability = actionConfig.probability ?? 0.15;
            this.enabled = actionConfig.enabled !== false;
        } else {
            this.probability = 0.15;
            this.enabled = true;
        }
        this.logger.info(
            `[LikeAction] Initialized (enabled: ${this.enabled}, probability: ${(this.probability * 100).toFixed(0)}%)`
        );
    }

    async canExecute(_context = {}) {
        if (!this.agent) {
            return { allowed: false, reason: 'agent_not_initialized' };
        }

        if (this.agent.diveQueue && !this.agent.diveQueue.canEngage('likes')) {
            return { allowed: false, reason: 'engagement_limit_reached' };
        }

        return { allowed: true, reason: null };
    }

    async execute(context = {}) {
        this.stats.attempts++;

        const { tweetElement, tweetUrl } = context;

        this.logger.info(`[LikeAction] Executing like`);

        try {
            if (tweetElement && this.agent?.scrollToGoldenZone) {
                try {
                    await this.agent.scrollToGoldenZone(tweetElement);
                } catch (_error) {
                    void _error;
                }
            }
            await this.agent.handleLike(tweetElement);
            this.stats.successes++;

            this.logger.info(`[LikeAction] ✅ Like posted`);

            return {
                success: true,
                executed: true,
                reason: 'success',
                newEngagement: true,
                data: { tweetUrl },
                engagementType: this.engagementType,
            };
        } catch (error) {
            this.stats.failures++;
            this.logger.error(`[LikeAction] Exception: ${error.message}`);

            return {
                success: false,
                executed: true,
                reason: 'exception',
                newEngagement: false,
                data: { error: error.message },
                engagementType: this.engagementType,
            };
        }
    }

    async tryExecute(context = {}) {
        const can = await this.canExecute(context);
        if (!can.allowed) {
            this.stats.skipped++;
            return {
                success: false,
                executed: false,
                reason: can.reason,
                newEngagement: false,
                engagementType: this.engagementType,
            };
        }

        if (Math.random() > this.probability) {
            this.stats.skipped++;
            return {
                success: false,
                executed: false,
                reason: 'probability',
                newEngagement: false,
                engagementType: this.engagementType,
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
            skipped: this.stats.skipped,
            successRate: `${successRate}%`,
            engagementType: this.engagementType,
        };
    }

    resetStats() {
        this.stats = {
            attempts: 0,
            successes: 0,
            failures: 0,
            skipped: 0,
        };
    }
}
