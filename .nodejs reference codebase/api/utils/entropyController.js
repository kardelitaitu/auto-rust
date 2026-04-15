/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Centralized Entropy Controller
 * Manages all stochastic behavior across the automation system.
 * Provides human-realistic timing distributions and session-wide consistency.
 * @module utils/entropyController
 */

import { createLogger } from '../core/logger.js';
import { calculateBackoffDelay } from './retry.js';

/**
 * EntropyController Class
 * Create NEW INSTANCE per browser session for parallel safety.
 *
 * @class EntropyController
 */
class EntropyController {
    constructor(options = {}) {
        this.logger = createLogger('EntropyController');
        this.sessionId =
            options.sessionId || `session-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
        this.sessionStart = Date.now();
        this.sessionEntropy = this.generateSessionProfile();
        this.actionLog = [];
        this.lastActionTimestamp = Date.now();

        // Fatigue System - Simulates user fatigue over long sessions
        // Random activation between 3-5 minutes
        this.fatigueEnabled = options.fatigueEnabled !== false;
        this.fatigueMinStartMs = 3 * 60 * 1000;
        this.fatigueMaxStartMs = 5 * 60 * 1000;
        this.fatigueActivationTime = this.gaussian(
            (this.fatigueMinStartMs + this.fatigueMaxStartMs) / 2,
            (this.fatigueMaxStartMs - this.fatigueMinStartMs) / 6,
            this.fatigueMinStartMs,
            this.fatigueMaxStartMs
        );
        this.fatigueActive = false;
        this.fatigueLevel = 0;

        this.logger.info(
            `[${this.sessionId}] Session initialized with paceMultiplier=${this.sessionEntropy.paceMultiplier.toFixed(2)}`
        );
        this.logger.info(
            `[${this.sessionId}] Fatigue scheduled for T+${(this.fatigueActivationTime / 60000).toFixed(1)}m`
        );
    }

    /**
     * Check if fatigue should activate based on session time
     * Call this method periodically during long sessions
     * @param {number} [elapsedMs] - Optional elapsed time override
     * @returns {boolean} Whether fatigue is now active
     */
    checkFatigue(elapsedMs = null) {
        if (!this.fatigueEnabled || this.fatigueActive) {
            return this.fatigueActive;
        }

        const elapsed = elapsedMs !== null ? elapsedMs : Date.now() - this.sessionStart;

        if (elapsed > this.fatigueActivationTime) {
            this.fatigueActive = true;
            this.fatigueLevel = this.calculateFatigueLevel();
            this.logger.info(
                `[${this.sessionId}] [Fatigue] Activated at T+${(elapsed / 60000).toFixed(1)}m (level=${this.fatigueLevel.toFixed(2)})`
            );
        }

        return this.fatigueActive;
    }

    /**
     * Calculate fatigue level (0-1) based on session duration
     * Longer sessions = more fatigue
     * @returns {number} Fatigue level
     */
    calculateFatigueLevel() {
        const elapsed = Date.now() - this.sessionStart;
        // Fatigue increases linearly, capped at 1.0 after 15 minutes past activation
        const fatigueDuration = elapsed - this.fatigueActivationTime;
        const fatigueRate = 1 / (10 * 60 * 1000); // 10 minutes to reach max fatigue
        return Math.min(1.0, fatigueDuration * fatigueRate);
    }

    /**
     * Get fatigue modifiers for humanization behavior
     * Call this when executing actions to get speed/adjustment multipliers
     * @returns {object|null} Fatigue modifiers or null if not active
     */
    getFatigueModifiers() {
        if (!this.fatigueActive) {
            return null;
        }

        const level = this.fatigueLevel;

        return {
            // Movement becomes slower (0.7 - 0.9)
            movementSpeed: 1.0 - level * 0.3,
            // Hesitation increases (1.2 - 1.5x)
            hesitationIncrease: 1.0 + level * 0.5,
            // Click hold time increases (1.1 - 1.3x)
            clickHoldTime: 1.0 + level * 0.3,
            // Scroll amounts decrease (0.8 - 0.95)
            scrollAmount: 1.0 - level * 0.2,
            // Micro-break chance increases (0.05 -> 0.15)
            microBreakChance: 0.05 + level * 0.1,
            // Typing speed decreases
            typingSpeed: 1.0 - level * 0.25,
        };
    }

    /**
     * Apply fatigue modifiers to a timing value
     * @param {string} type - Timing type ('movement', 'hesitation', 'click', 'scroll', 'typing')
     * @param {number} baseValue - Original timing value in ms
     * @returns {number} Adjusted timing value
     */
    applyFatigueToTiming(type, baseValue) {
        const modifiers = this.getFatigueModifiers();
        if (!modifiers) {
            return baseValue;
        }

        switch (type) {
            case 'movement':
                return baseValue / modifiers.movementSpeed;
            case 'hesitation':
            case 'click':
                return baseValue * modifiers.clickHoldTime;
            case 'scroll':
                return baseValue * modifiers.scrollAmount;
            case 'typing':
                return baseValue / modifiers.typingSpeed;
            default:
                return baseValue;
        }
    }

    /**
     * Check if a micro-break should be taken (fatigue-aware)
     * @returns {boolean}
     */
    shouldMicroBreak() {
        const modifiers = this.getFatigueModifiers();
        const baseChance = this.sessionEntropy.microBreakProb;
        const adjustedChance = modifiers ? baseChance + modifiers.microBreakChance : baseChance;

        const timeSinceLastAction = Date.now() - this.lastActionTimestamp;
        const fatigueFactor = Math.min(timeSinceLastAction / 300000, 1);

        return Math.random() < adjustedChance + fatigueFactor * 0.08;
    }

    /**
     * Get micro-break duration (fatigue-aware)
     * @returns {number} Duration in ms
     */
    microBreakDuration() {
        const modifiers = this.getFatigueModifiers();
        let baseDuration;

        // 80% short breaks, 20% longer
        if (Math.random() < 0.8) {
            baseDuration = this.gaussian(3000, 1000, 1000, 8000);
        } else {
            baseDuration = this.logNormal(9.2, 0.5);
        }

        // Fatigue increases break duration
        if (modifiers && Math.random() < modifiers.microBreakChance) {
            baseDuration *= 1.5;
        }

        return Math.floor(baseDuration);
    }

    /**
     * Generate a session-wide "personality" that affects all timing.
     * This ensures consistent behavior within a session (like a real user).
     */
    generateSessionProfile() {
        return {
            // Multiplier for all delays (0.7 = fast user, 1.3 = slow user)
            paceMultiplier: this.gaussian(1.0, 0.15, 0.7, 1.4),
            // Probability of micro-breaks
            microBreakProb: this.gaussian(0.05, 0.02, 0.01, 0.12),
            // Hesitation tendency (0-1) - affects pre-action delays
            hesitationFactor: Math.random(),
            // Scroll preference intensity
            scrollIntensity: this.gaussian(1.0, 0.2, 0.6, 1.5),
            // Reaction speed profile
            reactionProfile: Math.random() < 0.3 ? 'fast' : Math.random() < 0.5 ? 'normal' : 'slow',
        };
    }

    // ============ CORE DISTRIBUTION FUNCTIONS ============

    /**
     * Box-Muller Gaussian distribution
     * @param {number} mean - Center of distribution
     * @param {number} stdDev - Standard deviation
     * @param {number} [min] - Optional minimum clamp
     * @param {number} [max] - Optional maximum clamp
     * @returns {number} Normally distributed value
     */
    gaussian(mean, stdDev, min, max) {
        let u = 0,
            v = 0;
        while (u === 0) u = Math.random();
        while (v === 0) v = Math.random();
        const z = Math.sqrt(-2.0 * Math.log(u)) * Math.cos(2.0 * Math.PI * v);
        let result = mean + z * stdDev;
        if (min !== undefined) result = Math.max(min, result);
        if (max !== undefined) result = Math.min(max, result);
        return result;
    }

    /**
     * Log-normal distribution (for reading times, session durations)
     * Produces values with long right tails - common in human behavior.
     * @param {number} mu - Mean of underlying normal
     * @param {number} sigma - Std dev of underlying normal
     * @returns {number} Log-normally distributed value
     */
    logNormal(mu, sigma) {
        const normal = this.gaussian(0, 1);
        return Math.exp(mu + sigma * normal);
    }

    /**
     * @param {number} [attempt=0] - Current attempt number (0-indexed)
     * @param {number} [baseMs=1500] - Base delay for first retry
     * @returns {number} Delay in milliseconds
     */
    retryDelay(attempt = 0, baseMs = 1500) {
        const delay = calculateBackoffDelay(attempt, {
            baseDelay: baseMs,
            maxDelay: 30000,
            factor: 1.8,
            jitterMin: 0.7,
            jitterMax: 1.3,
        });
        return Math.max(500, delay);
    }

    /**
     * Poisson distribution (for count-based randomness)
     * @param {number} lambda - Expected value
     * @returns {number} Poisson-distributed integer
     */
    poisson(lambda) {
        const L = Math.exp(-lambda);
        let k = 0;
        let p = 1;
        do {
            k++;
            p *= Math.random();
        } while (p > L);
        return k - 1;
    }

    // ============ SEMANTIC TIMING FUNCTIONS ============

    /**
     * Human reaction time (for responding to visual stimuli)
     * Based on research: ~200-300ms median, log-normal distribution
     * @returns {number} Reaction time in milliseconds
     */
    reactionTime() {
        const baseProfiles = {
            fast: { mu: 5.2, sigma: 0.25 }, // ~180ms median
            normal: { mu: 5.5, sigma: 0.3 }, // ~245ms median
            slow: { mu: 5.7, sigma: 0.35 }, // ~300ms median
        };
        const profile = baseProfiles[this.sessionEntropy.reactionProfile];
        const base = this.logNormal(profile.mu, profile.sigma);
        return Math.floor(Math.max(100, Math.min(800, base * this.sessionEntropy.paceMultiplier)));
    }

    /**
     * Scroll settle time (after scrolling, before next action)
     * Humans need time to visually process new content.
     * @returns {number} Settle time in milliseconds
     */
    scrollSettleTime() {
        // Base: 1000-2000ms gaussian, affected by pace
        const base = this.gaussian(1500, 400, 800, 3000);
        return Math.floor(base * this.sessionEntropy.paceMultiplier);
    }

    /**
     * Page load wait (after navigation)
     * Accounts for visual scanning of new page.
     * @returns {number} Wait time in milliseconds
     */
    pageLoadWait() {
        // Base: 1500-3500ms with long tail possibility
        const base = this.gaussian(2500, 700, 1200, 5000);
        // 10% chance of longer wait (distraction simulation)
        const distracted = Math.random() < 0.1;
        const multiplier = distracted ? 1.8 : 1.0;
        return Math.floor(base * this.sessionEntropy.paceMultiplier * multiplier);
    }

    /**
     * Post-click delay (after clicking a button)
     * Short pause to observe effect of action.
     * @returns {number} Delay in milliseconds
     */
    postClickDelay() {
        // Base: 400-900ms
        const base = this.gaussian(650, 150, 350, 1200);
        return Math.floor(base * this.sessionEntropy.paceMultiplier);
    }

    /**
     * Pre-decision delay (before committing to Like, Follow, etc.)
     * Humans hesitate before decisive actions.
     * @returns {{delay: number, microRetreat: boolean}} Delay info
     */
    preDecisionDelay() {
        const hesitant = this.sessionEntropy.hesitationFactor > 0.6;

        if (hesitant) {
            // Hesitant user: longer delay, possible micro-retreat
            return {
                delay: Math.floor(this.gaussian(1500, 500, 800, 4000)),
                microRetreat: Math.random() < 0.25,
            };
        }
        return {
            delay: Math.floor(this.gaussian(500, 200, 200, 1200)),
            microRetreat: false,
        };
    }

    /**
     * Inter-action gap (between major actions in a session)
     * @returns {number} Gap in milliseconds
     */
    interActionGap() {
        // Mix of short and long gaps
        if (Math.random() < 0.7) {
            // Short gap (continuing flow)
            return Math.floor(
                this.gaussian(800, 300, 300, 2000) * this.sessionEntropy.paceMultiplier
            );
        }
        // Long gap (micro-pause/distraction)
        return Math.floor(
            this.gaussian(3000, 1000, 1500, 8000) * this.sessionEntropy.paceMultiplier
        );
    }

    /**
     * Reading time estimate based on content length
     * @param {number} wordCount - Estimated words in content
     * @returns {number} Reading time in milliseconds
     */
    readingTime(wordCount) {
        // Average adult: 200-300 WPM with variance
        const wpm = this.gaussian(250, 50, 150, 400);
        const baseTime = (wordCount / wpm) * 60 * 1000;
        // Add comprehension pauses
        const pauses =
            this.poisson(Math.max(1, wordCount / 50)) * this.gaussian(400, 150, 100, 800);
        return Math.floor((baseTime + pauses) * this.sessionEntropy.paceMultiplier);
    }

    /**
     * Scroll distance (bimodal: small scans vs large sweeps)
     * @returns {number} Scroll distance in pixels
     */
    scrollDistance() {
        // 70% small scans, 30% large sweeps
        if (Math.random() < 0.7) {
            return Math.floor(
                this.gaussian(180, 60, 50, 350) * this.sessionEntropy.scrollIntensity
            );
        }
        return Math.floor(this.gaussian(650, 200, 400, 1200) * this.sessionEntropy.scrollIntensity);
    }

    // ============ AUDIT LOGGING ============

    /**
     * Log an action for audit trail
     * @param {string} actionType - Type of action performed
     * @param {object} [details={}] - Additional details
     */
    logAction(actionType, details = {}) {
        this.lastActionTimestamp = Date.now();
        this.actionLog.push({
            timestamp: this.lastActionTimestamp,
            type: actionType,
            sessionAge: this.lastActionTimestamp - this.sessionStart,
            ...details,
        });

        // Keep log bounded (prevent memory leak)
        if (this.actionLog.length > 500) {
            this.actionLog = this.actionLog.slice(-250);
        }
    }

    /**
     * Get session statistics for debugging/monitoring
     * @returns {object} Session stats
     */
    getSessionStats() {
        return {
            sessionAge: Date.now() - this.sessionStart,
            entropy: this.sessionEntropy,
            actionCount: this.actionLog.length,
            fatigue: {
                active: this.fatigueActive,
                level: this.fatigueLevel,
                modifiers: this.getFatigueModifiers(),
                activationTime: this.fatigueActivationTime,
            },
            recentActions: this.actionLog.slice(-20),
        };
    }

    /**
     * Reset session entropy (call between browser sessions)
     */
    resetSession() {
        this.sessionStart = Date.now();
        this.sessionEntropy = this.generateSessionProfile();
        this.actionLog = [];
        this.lastActionTimestamp = Date.now();

        // Reset fatigue system with new random activation time
        this.fatigueActive = false;
        this.fatigueLevel = 0;
        this.fatigueActivationTime = this.gaussian(
            (this.fatigueMinStartMs + this.fatigueMaxStartMs) / 2,
            (this.fatigueMaxStartMs - this.fatigueMinStartMs) / 6,
            this.fatigueMinStartMs,
            this.fatigueMaxStartMs
        );

        this.logger.info(
            `[${this.sessionId}] Session reset. paceMultiplier=${this.sessionEntropy.paceMultiplier.toFixed(2)}, Fatigue T+${(this.fatigueActivationTime / 60000).toFixed(1)}m`
        );
    }
}

// ============================================================================
// PARALLEL SAFETY NOTE
// ============================================================================
// For parallel browser sessions, create NEW INSTANCES per browser:
//
// ❌ UNSAFE (shared state):
//   import { entropy } from './entropyController.js';
//   entropy.retryDelay();
//
// ✅ SAFE (isolated state):
//   import { EntropyController } from './entropyController.js';
//   const entropy = new EntropyController({ sessionId: browserId });
//
// The singleton below is kept for backward compatibility but should not be
// used when running multiple parallel browser sessions.
// ============================================================================

// Singleton instance for backward compatibility (use with caution in parallel mode)
export const entropy = new EntropyController();

// Export class for parallel-safe usage
export { EntropyController };
