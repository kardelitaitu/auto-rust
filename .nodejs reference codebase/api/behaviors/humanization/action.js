/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { api } from '../../index.js';
/**
 * Action Predictor
 * Weighted random action selection for natural behavior
 *
 * Action Probabilities (based on human behavior research):
 * - Scroll: 35-45%
 * - Click/Tweet: 20-30%
 * - Go Back: 10-15%
 * - Explore: 8-12%
 * - Profile: 5-10%
 */

import { mathUtils } from '../../utils/math.js';
import { scrollRandom } from '../scroll-helper.js';
import { GhostCursor } from '../../utils/ghostCursor.js';

export class ActionPredictor {
    constructor(logger) {
        this.logger = logger;

        // Base action probabilities
        this.baseProbabilities = {
            scroll: { weight: 0.4, min: 0.35, max: 0.45 },
            click: { weight: 0.25, min: 0.2, max: 0.3 },
            back: { weight: 0.12, min: 0.08, max: 0.15 },
            explore: { weight: 0.1, min: 0.08, max: 0.12 },
            profile: { weight: 0.08, min: 0.05, max: 0.1 },
            idle: { weight: 0.05, min: 0.02, max: 0.08 },
        };

        // Cycle modifiers
        this.cycleCount = 0;
    }

    /**
     * Predict next action based on patterns
     *
     * @param {number} cycleCount - Current cycle count
     * @returns {object} Predicted action
     */
    predict(cycleCount = 0) {
        this.cycleCount = cycleCount;

        // Adjust probabilities based on cycle count
        const adjusted = this._adjustForFatigue(this.baseProbabilities);

        // Select action
        const action = this._weightedRandom(adjusted);

        // Add confidence score
        const confidence = this._calculateConfidence(adjusted[action]);

        return {
            type: action,
            confidence,
            probabilities: adjusted,
            phase: this._getPhase(cycleCount),
        };
    }

    /**
     * Get action probabilities
     */
    getProbabilities() {
        return this._adjustForFatigue(this.baseProbabilities);
    }

    /**
     * Select action for execution
     */
    selectAction() {
        const prediction = this.predict(this.cycleCount);
        return prediction.type;
    }

    /**
     * Execute predicted action (returns async function)
     */
    async executeAction(page, actionType) {
        switch (actionType) {
            case 'scroll':
                await this._actionScroll(page);
                break;
            case 'click':
                await this._actionClick(page);
                break;
            case 'back':
                await this._actionBack(page);
                break;
            case 'explore':
                await this._actionExplore(page);
                break;
            case 'profile':
                await this._actionProfile(page);
                break;
            case 'idle':
                await this._actionIdle(page);
                break;
            default:
                await this._actionScroll(page);
        }
    }

    // ==========================================
    // ACTION IMPLEMENTATIONS
    // ==========================================

    async _actionScroll(page) {
        const direction = Math.random() > 0.3 ? 'down' : 'random';
        const intensity = Math.random() > 0.7 ? 'heavy' : 'normal';

        await this._humanScroll(page, direction, intensity);
    }

    async _actionClick(page) {
        // Find and click a tweet
        const ghost = new GhostCursor(page);
        const tweets = await page.$$('article[data-testid="tweet"]');
        if (tweets.length > 0) {
            const index = Math.floor(Math.random() * Math.min(tweets.length, 5));
            const tweet = tweets[index];

            if (tweet) {
                await tweet.evaluate((el) => el.scrollIntoView({ block: 'center' }));
                await api.wait(1000);

                // Click on tweet text or time
                const textEl = await tweet.$('[data-testid="tweetText"]');
                if (textEl) {
                    await ghost.click(textEl);
                }
            }
        }
    }

    async _actionBack(page) {
        await page.goBack().catch(() => {});
        await api.wait(1000);
    }

    async _actionExplore(_page) {
        await api.goto('https://x.com/explore', { waitUntil: 'domcontentloaded' });
        await api.wait(1000);
    }

    async _actionProfile(page) {
        // Click on a random profile
        const ghost = new GhostCursor(page);
        const profileLinks = await page.$$('a[href*="/"][role="link"]:not([href*="search"]');
        if (profileLinks.length > 0) {
            const index = Math.floor(Math.random() * Math.min(profileLinks.length, 10));
            await ghost.click(profileLinks[index]).catch(() => {});
            await api.wait(1000);
        }
    }

    async _actionIdle(page) {
        // Do nothing for a while
        await api.wait(1000);

        // Maybe a small movement
        if (Math.random() > 0.5) {
            await page.mouse.move(
                mathUtils.randomInRange(-20, 20),
                mathUtils.randomInRange(-20, 20)
            );
        }
    }

    async _humanScroll(page, direction, intensity) {
        const scrollAmounts = {
            light: { min: 50, max: 150 },
            normal: { min: 100, max: 300 },
            heavy: { min: 300, max: 600 },
        };

        const config = scrollAmounts[intensity] || scrollAmounts.normal;
        const burstCount = intensity === 'heavy' ? 3 : 2;

        for (let i = 0; i < burstCount; i++) {
            const amount = mathUtils.randomInRange(config.min, config.max);
            const dirMultiplier = direction === 'up' ? -1 : 1;

            await scrollRandom(amount * dirMultiplier, amount * dirMultiplier);
            await api.wait(1000);

            // Sometimes scroll back slightly
            if (Math.random() > 0.7) {
                await scrollRandom(-20, -50);
            }
        }

        await api.wait(1000);
    }

    // ==========================================
    // INTERNAL METHODS
    // ==========================================

    /**
     * Adjust probabilities based on session fatigue
     */
    _adjustForFatigue(probs) {
        // Humans get bored, scroll more, click less over time
        const fatigueMultiplier = this.cycleCount > 30 ? 0.9 : 1.0;
        const fatigueMultiplier2 = this.cycleCount > 50 ? 0.85 : 1.0;

        const adjusted = { ...probs };

        adjusted.scroll.weight *= fatigueMultiplier;
        adjusted.click.weight *= fatigueMultiplier2;
        adjusted.idle.weight *= 1 + (1 - fatigueMultiplier2);

        // Normalize
        const total = Object.values(adjusted).reduce((sum, p) => sum + p.weight, 0);
        for (const key in adjusted) {
            adjusted[key].weight = adjusted[key].weight / total;
        }

        return adjusted;
    }

    /**
     * Weighted random selection
     */
    _weightedRandom(probs) {
        const items = Object.entries(probs);
        const total = items.reduce((sum, [, p]) => sum + p.weight, 0);

        let random = Math.random() * total;

        for (const [key, p] of items) {
            random -= p.weight;
            if (random <= 0) {
                return key;
            }
        }

        return items[0][0];
    }

    /**
     * Calculate confidence based on variance
     */
    _calculateConfidence(probData) {
        // Higher probability = higher confidence
        const base = probData.weight / (probData.max || 0.5);
        // Add some noise
        const noise = Math.random() * 0.1;
        return Math.min(0.95, base + noise);
    }

    /**
     * Get current session phase
     */
    _getPhase(cycleCount) {
        if (cycleCount < 5) return 'warmup';
        if (cycleCount < 20) return 'active';
        if (cycleCount < 40) return 'established';
        return 'fatigued';
    }
}

export default ActionPredictor;
