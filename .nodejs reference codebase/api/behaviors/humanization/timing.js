/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { api } from '../../index.js';
/**
 * Human Timing Engine
 * Natural timing patterns for human-like behavior
 *
 * Features:
 * - Thinking pauses before actions
 * - Variable delays with gaussian distribution
 * - Session-aware timing
 * - Context-aware delays
 */

import { mathUtils } from '../../utils/math.js';
import { entropy as _entropy } from '../../utils/entropyController.js';

export class HumanTiming {
    constructor(page, logger) {
        this.page = page;
        this.logger = logger;
    }

    // ==========================================
    // THINKING PAUSES
    // ==========================================

    /**
     * Get thinking time before an action
     *
     * @param {string} actionType - Type of action
     * @param {object} context - Additional context
     * @returns {number} Milliseconds to wait
     */
    getThinkTime(actionType = 'general', context = {}) {
        const baseTimes = {
            // Quick actions (500-1500ms)
            like: { min: 500, max: 1500, gaussian: 800 },
            bookmark: { min: 500, max: 1500, gaussian: 800 },

            // Medium actions (1000-3000ms)
            view_media: { min: 1000, max: 3000, gaussian: 1500 },
            check_profile: { min: 1500, max: 3000, gaussian: 2000 },

            // Consideration actions (2000-5000ms)
            retweet: { min: 2000, max: 5000, gaussian: 3000 },
            quote: { min: 3000, max: 6000, gaussian: 4000 },

            // Major actions (3000-8000ms)
            reply: { min: 3000, max: 8000, gaussian: 4500 },
            compose: { min: 4000, max: 10000, gaussian: 6000 },
            follow: { min: 4000, max: 8000, gaussian: 5000 },
            message: { min: 5000, max: 12000, gaussian: 7000 },

            // Navigation (500-2000ms)
            click: { min: 500, max: 2000, gaussian: 1000 },
            navigate: { min: 1000, max: 3000, gaussian: 1500 },
            scroll: { min: 200, max: 800, gaussian: 400 },

            // Reading (2000-10000ms)
            read_tweet: { min: 2000, max: 5000, gaussian: 3000 },
            read_thread: { min: 5000, max: 15000, gaussian: 8000 },
            skim_feed: { min: 1000, max: 3000, gaussian: 1800 },

            // General/Default
            general: { min: 500, max: 2000, gaussian: 1000 },
            default: { min: 300, max: 1000, gaussian: 500 },
        };

        const config = baseTimes[actionType] || baseTimes.default;

        // Add variation based on content interest
        let variation = 1.0;
        if (context.interesting) {
            // More time for interesting content
            variation = 1.3;
        } else if (context.boring) {
            // Less time for boring content
            variation = 0.7;
        }

        // Time of day variation (humans are slower in morning, faster at night)
        const hour = new Date().getHours();
        if (hour >= 6 && hour <= 9) {
            // Morning: slower
            variation *= 1.2;
        } else if (hour >= 22 || hour <= 5) {
            // Late night: faster (less thoughtful)
            variation *= 0.8;
        }

        // Previous action effect (humans get tired, speeds up over time)
        if (context.cycleCount > 20) {
            variation *= 0.9;
        } else if (context.cycleCount > 50) {
            variation *= 0.8;
        }

        // Use gaussian distribution for natural variance
        const baseTime = mathUtils.gaussian(config.gaussian * variation, config.gaussian * 0.3);

        // Clamp to min/max
        return Math.max(config.min, Math.min(config.max, Math.round(baseTime)));
    }

    // ==========================================
    // NATURAL PAUSES
    // ==========================================

    /**
     * Get natural pause between actions
     * Context-based timing
     */
    getNaturalPause(context = 'transition') {
        const pauses = {
            // Very short (100-300ms)
            micro: { min: 100, max: 300, gaussian: 180 },

            // Short (200-500ms)
            quick: { min: 200, max: 500, gaussian: 300 },

            // Normal transition (300-800ms)
            transition: { min: 300, max: 800, gaussian: 500 },

            // Processing (500-1500ms)
            processing: { min: 500, max: 1500, gaussian: 800 },

            // Long transition (1000-2000ms)
            extended: { min: 1000, max: 2000, gaussian: 1400 },

            // Default
            default: { min: 200, max: 600, gaussian: 350 },
        };

        const config = pauses[context] || pauses.default;
        return mathUtils.gaussian(config.gaussian, config.gaussian * 0.2);
    }

    // ==========================================
    // SESSION TIMING
    // ==========================================

    /**
     * Session ramp-up (warmup before full activity)
     */
    async sessionRampUp() {
        // Humans don't start at full speed
        const rampUpSteps = 2;

        for (let i = 1; i <= rampUpSteps; i++) {
            const factor = i / rampUpSteps;
            const _delay = mathUtils.gaussian(800 * factor, 300);
            await api.wait(1000);
        }
    }

    /**
     * Fatigue effect (speeds up over time)
     */
    getFatigueMultiplier(cycleCount) {
        if (cycleCount < 10) return 1.0;
        if (cycleCount < 30) return 0.95;
        if (cycleCount < 50) return 0.9;
        if (cycleCount < 100) return 0.85;
        return 0.8;
    }

    // ==========================================
    // ACTION-SPECIFIC TIMING
    // ==========================================

    /**
     * Typing speed (variable)
     */
    getTypingDelay(charIndex, totalChars, _context = {}) {
        // First few characters: slower (warm-up)
        if (charIndex < 3) {
            return mathUtils.randomInRange(150, 300);
        }

        // Middle: normal typing
        if (charIndex < totalChars * 0.8) {
            // Random variation
            const variation = Math.random();

            if (variation < 0.1) {
                // Occasional pause (thinking)
                return mathUtils.randomInRange(200, 400);
            } else if (variation < 0.85) {
                // Normal typing
                return mathUtils.randomInRange(30, 100);
            } else {
                // Fast typing
                return mathUtils.randomInRange(20, 50);
            }
        }

        // End: slowing down (finishing)
        return mathUtils.randomInRange(50, 150);
    }

    /**
     * Hover time before clicking
     */
    getHoverTime(actionType = 'general') {
        const hoverTimes = {
            like: { min: 200, max: 500, gaussian: 300 },
            retweet: { min: 500, max: 1200, gaussian: 700 },
            reply: { min: 300, max: 800, gaussian: 450 },
            follow: { min: 800, max: 2000, gaussian: 1200 },
            profile: { min: 400, max: 1000, gaussian: 600 },
            link: { min: 300, max: 700, gaussian: 400 },
            general: { min: 200, max: 600, gaussian: 350 },
        };

        const config = hoverTimes[actionType] || hoverTimes.general;
        return mathUtils.gaussian(config.gaussian, config.gaussian * 0.25);
    }

    /**
     * Reading time for content
     */
    getReadingTime(wordCount, type = 'tweet') {
        // Average reading speed: 200-250 words per minute
        const wpm = mathUtils.gaussian(220, 30);
        const minutes = wordCount / wpm;
        const baseMs = minutes * 60 * 1000;

        const multipliers = {
            tweet: 0.5, // Tweets: skim faster
            thread: 1.5, // Threads: read slower
            article: 2.0, // Articles: read slowest
            media: 0.3, // Media: quick glance
            default: 1.0,
        };

        const multiplier = multipliers[type] || multipliers.default;
        const adjustedMs = baseMs * multiplier;

        // Add variation (±30%)
        const variation = 0.7 + Math.random() * 0.6;

        return Math.round(adjustedMs * variation);
    }

    // ==========================================
    // DELAY UTILITIES
    // ==========================================

    /**
     * Create delay with jitter
     */
    withJitter(baseMs, jitterPercent = 0.2) {
        const jitter = baseMs * jitterPercent;
        return mathUtils.gaussian(baseMs, jitter);
    }

    /**
     * Create exponential backoff (human-like: less aggressive)
     */
    humanBackoff(attempt, baseMs = 1000, maxAttempts = 5) {
        if (attempt >= maxAttempts) return baseMs * maxAttempts;

        // Human backoff is gentler than robotic
        const backoff = Math.pow(1.5, attempt) * baseMs;
        return Math.min(backoff, baseMs * maxAttempts);
    }

    /**
     * Random delay between min and max
     */
    random(minMs, maxMs) {
        return mathUtils.randomInRange(minMs, maxMs);
    }
}

export default HumanTiming;
