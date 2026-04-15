/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Follow Action
 * Handles following tweet authors.
 * The actual follow logic is delegated to api/actions/follow.js (followWithAPI)
 * via the makeApiExecutor factory in tasks/api-twitterActivity.js.
 * This class exists solely to register the "follow" slot in agent.actions
 * and carry per-session stats + probability config.
 * @module utils/actions/ai-twitter-follow
 */

import { createLogger } from '../core/logger.js';

export class FollowAction {
    constructor(agent, _options = {}) {
        this.agent = agent;
        this.logger = createLogger('ai-twitter-follow.js');
        this.engagementType = 'follows';

        this.stats = {
            attempts: 0,
            successes: 0,
            failures: 0,
            skipped: 0,
        };

        this.loadConfig();
    }

    loadConfig() {
        if (this.agent?.twitterConfig?.actions?.follow) {
            const actionConfig = this.agent.twitterConfig.actions.follow;
            this.probability = actionConfig.probability ?? 0.1;
            this.enabled = actionConfig.enabled !== false;
        } else {
            this.probability = 0.1;
            this.enabled = true;
        }
        this.logger.info(
            `[FollowAction] Initialized (enabled: ${this.enabled}, probability: ${(this.probability * 100).toFixed(0)}%)`
        );
    }

    async canExecute(_context = {}) {
        if (!this.agent) {
            return { allowed: false, reason: 'agent_not_initialized' };
        }

        if (this.agent.diveQueue && !this.agent.diveQueue.canEngage('follows')) {
            return { allowed: false, reason: 'engagement_limit_reached' };
        }

        return { allowed: true, reason: null };
    }

    /**
     * execute() is replaced at runtime by makeApiExecutor in api-twitterActivity.js.
     * This stub is a safe fallback for any non-api-twitterActivity call path.
     */
    async execute(_context = {}) {
        this.stats.attempts++;
        this.stats.failures++;
        this.logger.warn('[FollowAction] execute() stub called — api executor not yet wired.');
        return {
            success: false,
            executed: false,
            reason: 'executor_not_wired',
            newEngagement: false,
            engagementType: this.engagementType,
        };
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
