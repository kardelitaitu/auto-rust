/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Engagement Limits Module
 * Tracks and enforces per-session engagement limits
 * to prevent over-engagement and rate limiting.
 *
 * @module utils/engagement-limits
 */

const DEFAULT_LIMITS = {
    replies: 3,
    retweets: 1,
    quotes: 1,
    likes: 5,
    follows: 2,
    bookmarks: 2,
};

const CRITICAL_THRESHOLDS = {
    replies: 0.8,
    retweets: 0.8,
    quotes: 0.8,
    likes: 0.8,
    follows: 0.8,
    bookmarks: 0.8,
};

function createEngagementTracker(limits = DEFAULT_LIMITS) {
    const stats = {
        replies: 0,
        retweets: 0,
        quotes: 0,
        likes: 0,
        follows: 0,
        bookmarks: 0,
    };

    return {
        limits: { ...limits },
        stats: { ...stats },
        history: [],

        canPerform(action) {
            const current = this.stats[action] || 0;
            const limit = this.limits[action] || Infinity;
            return current < limit;
        },

        getRemaining(action) {
            const current = this.stats[action] || 0;
            const limit = this.limits[action] || Infinity;
            return Math.max(0, limit - current);
        },

        getUsage(action) {
            const current = this.stats[action] || 0;
            const limit = this.limits[action] || Infinity;
            if (limit === 0) return 0;
            return current / limit;
        },

        getProgress(action) {
            const current = this.stats[action] || 0;
            const limit = this.limits[action] || Infinity;
            if (limit === Infinity) return `${current} used`;
            return `${current}/${limit}`;
        },

        getProgressPercent(action) {
            return (this.getUsage(action) * 100).toFixed(1) + '%';
        },

        isNearLimit(action, threshold = 0.8) {
            return this.getUsage(action) >= threshold;
        },

        isExhausted(action) {
            return !this.canPerform(action);
        },

        isAnyExhausted() {
            return Object.keys(this.limits).some((action) => !this.canPerform(action));
        },

        hasRemainingCapacity() {
            return Object.values(this.stats).some((count, idx) => {
                const action = Object.keys(this.stats)[idx];
                return this.canPerform(action);
            });
        },

        record(action) {
            if (!Object.prototype.hasOwnProperty.call(this.stats, action)) {
                console.warn(`[engagement-limits.js] Unknown action: ${action}`);
                return false;
            }

            if (!this.canPerform(action)) {
                console.log(
                    `[engagement-limits.js] Limit reached for ${action} (${this.stats[action]}/${this.limits[action]})`
                );
                return false;
            }

            this.stats[action]++;

            this.history.push({
                action,
                timestamp: Date.now(),
                count: this.stats[action],
                limit: this.limits[action],
            });

            return true;
        },

        recordIfAllowed(action) {
            return this.record(action);
        },

        decrement(action) {
            if (this.stats[action] > 0) {
                this.stats[action]--;
                return true;
            }
            return false;
        },

        getStatus() {
            const status = {};
            for (const action of Object.keys(this.limits)) {
                const current = this.stats[action] || 0;
                const limit = this.limits[action] || Infinity;
                status[action] = {
                    current,
                    limit,
                    remaining: Math.max(0, limit - current),
                    percentage: limit > 0 ? ((current / limit) * 100).toFixed(1) + '%' : 'N/A',
                };
            }
            return status;
        },

        getSummary() {
            const summary = [];
            for (const action of Object.keys(this.limits)) {
                const current = this.stats[action] || 0;
                const limit = this.limits[action] || Infinity;
                if (limit !== Infinity) {
                    summary.push(`${action}: ${current}/${limit}`);
                }
            }
            return summary.join(', ');
        },

        getUsageRate() {
            const totalUsed = Object.values(this.stats).reduce((a, b) => a + b, 0);
            const totalLimit = Object.values(this.limits).reduce((a, b) => a + (b || 0), 0);
            return {
                used: totalUsed,
                limit: totalLimit,
                percentage:
                    totalLimit > 0 ? ((totalUsed / totalLimit) * 100).toFixed(1) + '%' : 'N/A',
            };
        },

        getRecentActions(count = 10) {
            return this.history.slice(-count);
        },

        reset() {
            for (const key of Object.keys(this.stats)) {
                this.stats[key] = 0;
            }
            this.history = [];
        },

        setLimit(action, limit) {
            if (Object.prototype.hasOwnProperty.call(this.limits, action)) {
                this.limits[action] = limit;
            }
        },

        setLimits(newLimits) {
            this.limits = { ...this.limits, ...newLimits };
        },
    };
}

function formatLimitStatus(tracker) {
    const status = tracker.getStatus();
    const lines = ['Engagement Limits Status:'];

    for (const [action, data] of Object.entries(status)) {
        const emoji = data.remaining === 0 ? '🚫' : data.percentage >= '80.0%' ? '⚠️' : '✅';
        lines.push(`  ${emoji} ${action}: ${data.current}/${data.limit} (${data.percentage} used)`);
    }

    return lines.join('\n');
}

function shouldSkipAction(action, phase, modifiers) {
    const phaseMod = modifiers[phase]?.[action] || 1.0;
    const randomRoll = Math.random();
    return randomRoll > phaseMod;
}

function getSmartActionProbability(action, tracker, phase, modifiers) {
    const baseMod = modifiers[phase]?.[action] || 1.0;
    const remaining = tracker.getRemaining(action);
    const totalLimit = tracker.limits[action] || 1;
    const scarcityMod = Math.max(0.1, remaining / totalLimit);
    return baseMod * scarcityMod;
}

export const engagementLimits = {
    defaults: DEFAULT_LIMITS,
    thresholds: CRITICAL_THRESHOLDS,
    createEngagementTracker,
    formatLimitStatus,
    shouldSkipAction,
    getSmartActionProbability,
};

export default engagementLimits;
