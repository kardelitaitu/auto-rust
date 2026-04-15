/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { api } from '../../index.js';
import { mathUtils } from '../../utils/math.js';
import { scrollRandom } from '../scroll-helper.js';

export class SessionManager {
    constructor(page, logger, agent = null) {
        this.page = page;
        this.logger = logger;
        this.agent = agent;
    }

    /**
     * Get optimal session length based on time-of-day
     *
     * @returns {object} Session configuration
     */
    getOptimalLength() {
        // Return cached value if already calculated for this session
        if (this._cachedOptimalLength) {
            return this._cachedOptimalLength;
        }

        // Use agent config if available (prioritize minSeconds/maxSeconds from settings.json)
        const sessionConfig = this.agent?.config?.session || {};
        const configMinSec = sessionConfig.minSeconds || sessionConfig.minDuration;
        const configMaxSec = sessionConfig.maxSeconds || sessionConfig.maxDuration;

        let result;

        if (configMinSec && configMaxSec && !isNaN(configMinSec) && !isNaN(configMaxSec)) {
            const minMs = configMinSec * 1000;
            const maxMs = configMaxSec * 1000;
            const targetMs = mathUtils.randomInRange(minMs, maxMs);

            result = {
                minMs,
                maxMs,
                targetMs,
                reason: `configured ${configMinSec}-${configMaxSec}s session`,
            };
        } else {
            const hour = new Date().getHours();
            const isWeekend = [0, 6].includes(new Date().getDay());

            let baseLength, variability;

            if (hour >= 8 && hour <= 10) {
                // Morning peak
                baseLength = isWeekend ? 15 : 10; // minutes
                variability = 0.3;
            } else if (hour >= 12 && hour <= 14) {
                // Lunch peak
                baseLength = 12;
                variability = 0.25;
            } else if (hour >= 18 && hour <= 21) {
                // Evening peak
                baseLength = isWeekend ? 20 : 15;
                variability = 0.35;
            } else if (hour >= 22 || hour <= 5) {
                // Late night/early morning (shorter sessions)
                baseLength = isWeekend ? 10 : 7; // Increased from 5 to 7 for consistency
                variability = 0.4;
            } else {
                // Normal hours
                baseLength = isWeekend ? 12 : 8;
                variability = 0.3;
            }

            // Add gaussian variation
            const meanMs = baseLength * 60 * 1000;
            const stdDevMs = baseLength * variability * 60 * 1000;
            const variation = mathUtils.gaussian(meanMs, stdDevMs);

            result = {
                minMs: baseLength * 0.6 * 60 * 1000,
                maxMs: baseLength * 1.4 * 60 * 1000,
                targetMs: Math.max(120000, Math.round(variation)), // Floor at 2 minutes
                reason: this._getReason(hour, isWeekend),
            };
        }

        this._cachedOptimalLength = result;
        return result;
    }

    /**
     * Get reason for session length (for logging)
     */
    _getReason(hour, isWeekend) {
        const timeOfDay =
            hour >= 22 || hour <= 5
                ? 'late night'
                : hour <= 9
                  ? 'morning'
                  : hour <= 14
                    ? 'lunch'
                    : hour <= 21
                      ? 'evening'
                      : 'night';

        const dayType = isWeekend ? 'weekend' : 'weekday';

        return `${timeOfDay} ${dayType} session`;
    }

    /**
     * Check if should take a break
     */
    shouldTakeBreak(sessionDuration) {
        const config = this.getOptimalLength();

        // Take break at 80% of target session length
        const breakThreshold = config.targetMs * 0.8;

        return sessionDuration > breakThreshold;
    }

    /**
     * Get break duration
     */
    getBreakDuration() {
        // Human breaks: 30-90 minutes
        return mathUtils.randomInRange(30 * 60 * 1000, 90 * 60 * 1000);
    }

    /**
     * Session warmup (before starting)
     */
    async warmup() {
        const steps = mathUtils.randomInRange(3, 5);

        for (let i = 0; i < steps; i++) {
            // Gradual ramp up
            const _factor = (i + 1) / steps;
            await api.wait(1000);
        }
    }

    /**
     * Boredom pause during session
     * Every few cycles, humans take a "boredom" break
     */
    async boredomPause(page) {
        const _duration = mathUtils.randomInRange(2000, 5000);

        // Human boredom behaviors:
        // 1. Scroll randomly
        // 2. Check notifications area
        // 3. Just sit idle

        const behaviors = [
            async () => {
                // Random scroll
                await scrollRandom(-100, 100);
            },
            async () => {
                // Glance at top-right (notifications)
                await page.mouse.move(
                    Math.min(800, Math.random() * 200 + 600),
                    Math.random() * 50 + 50
                );
            },
            async () => {
                // Pure idle
            },
        ];

        const behavior = mathUtils.sample(behaviors);
        await behavior();

        await api.wait(1000);

        // Move mouse back
        await page.mouse.move(400, 400);
    }

    /**
     * Wrap-up activities before ending session
     */
    async wrapUp(page) {
        // Human wrap-up behaviors:
        // 1. Bookmark interesting content (30%)
        // 2. Check mentions before leaving (20%)
        // 3. Final scroll up (50%)

        const behaviors = [
            async () => {
                // Bookmark (simulated)
                await page.mouse.move(800, 300);
                await api.wait(1000);
            },
            async () => {
                // Check mentions
                await page.mouse.move(100, 100);
                await api.wait(1000);
            },
            async () => {
                // Final scroll
                await scrollRandom(50, 150);
            },
        ];

        // Weighted: 50% scroll, 30% bookmark, 20% mentions
        const roll = Math.random();
        let behavior;
        if (roll < 0.5) {
            behavior = behaviors[2];
        } else if (roll < 0.8) {
            behavior = behaviors[0];
        } else {
            behavior = behaviors[1];
        }

        await behavior();

        // Final pause
        await api.wait(1000);
    }

    /**
     * Get time until next natural break
     */
    getTimeUntilBreak(currentSessionMs) {
        const config = this.getOptimalLength();
        const remaining = config.targetMs * 0.8 - currentSessionMs;

        return Math.max(0, remaining);
    }

    /**
     * Check if session should end
     */
    shouldEndSession(sessionDurationMs) {
        const config = this.getOptimalLength();

        // Strictly enforce minimum session length
        if (sessionDurationMs < config.minMs) {
            return false;
        }

        // End if past max or at 90% with some randomness
        if (sessionDurationMs > config.maxMs) {
            return true;
        }

        if (sessionDurationMs > config.targetMs * 0.9) {
            // 30% chance to end at target duration
            return Math.random() < 0.3;
        }

        if (sessionDurationMs > config.targetMs) {
            // 60% chance to end at extended duration
            return Math.random() < 0.6;
        }

        return false;
    }

    /**
     * Get current session phase
     */
    getSessionPhase(elapsedMs) {
        const config = this.getOptimalLength();
        const progress = elapsedMs / config.targetMs;

        if (progress < 0.3) {
            return { phase: 'warmup', label: 'Getting started' };
        } else if (progress < 0.7) {
            return { phase: 'active', label: 'Active browsing' };
        } else if (progress < 0.9) {
            return { phase: 'winding_down', label: 'Wrapping up' };
        } else {
            return { phase: 'ending', label: 'About to leave' };
        }
    }

    /**
     * Calculate activity level based on session phase
     * Humans slow down as session progresses
     */
    getActivityMultiplier(phase) {
        const multipliers = {
            warmup: 1.0,
            active: 0.95,
            winding_down: 0.85,
            ending: 0.7,
        };

        return multipliers[phase] || 0.9;
    }
}

export default SessionManager;
