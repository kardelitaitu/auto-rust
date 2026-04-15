/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { api } from '../../index.js';
/**
 * Content Skimmer
 * Human-like content consumption patterns
 *
 * Human Reading Patterns:
 * 1. Quick glance (1-2s) - "What is this?"
 * 2. Scan (2-4s) - "Is this interesting?"
 * 3. Read (4-8s) - "Let me understand this"
 * 4. Deep read (8-15s) - "This is interesting!"
 */

import { mathUtils } from '../../utils/math.js';
import { entropy as _entropy } from '../../utils/entropyController.js';
import { scrollRandom } from '../scroll-helper.js';

export class ContentSkimmer {
    constructor(page, logger) {
        this.page = page;
        this.logger = logger;
        this.agent = null;
    }

    setAgent(agent) {
        this.agent = agent;
    }

    /**
     * Main content consumption method
     *
     * @param {string} type - 'tweet', 'thread', 'media', 'profile'
     * @param {string} duration - 'glance', 'skim', 'read', 'deep'
     */
    async skipping(type = 'tweet', duration = 'skim') {
        const durationConfig = {
            glance: { read: 1000, scroll: 50, pause: 500 },
            skim: { read: 2500, scroll: 100, pause: 1000 },
            read: { read: 5000, scroll: 150, pause: 1500 },
            deep: { read: 10000, scroll: 200, pause: 2500 },
        };

        const config = durationConfig[duration] || durationConfig.skim;

        switch (type) {
            case 'tweet':
                await this._skimTweet(config);
                break;
            case 'thread':
                await this._skimThread(config);
                break;
            case 'media':
                await this._skimMedia(config);
                break;
            case 'profile':
                await this._skimProfile(config);
                break;
            default:
                await this._skimTweet(config);
        }
    }

    /**
     * Reading behavior (for "reading" logs)
     */
    async reading(duration = 'normal') {
        const durationMap = {
            quick: { min: 1000, max: 2000 },
            normal: { min: 2000, max: 4000 },
            long: { min: 4000, max: 8000 },
        };

        const config = durationMap[duration] || durationMap.normal;
        const readTime = mathUtils.randomInRange(config.min, config.max);

        await api.wait(readTime);

        if (this.agent) {
            this.agent.log(`[Read] Reading for ${readTime}ms...`);
        }
    }

    // ==========================================
    // PRIVATE METHODS
    // ==========================================

    /**
     * Skim a single tweet
     */
    async _skimTweet(config) {
        // Quick glance
        await api.wait(1000);

        // Sometimes scroll a bit
        if (mathUtils.roll(0.3)) {
            await scrollRandom(config.scroll, config.scroll);
            await api.wait(1000);
        }

        // Micro-adjustments
        await this._microAdjustments(2);
    }

    /**
     * Skim a thread
     */
    async _skimThread(config) {
        // Read first tweet
        await api.wait(1000);

        // Scroll through thread (2-4 tweets)
        const tweetCount = mathUtils.randomInRange(2, 4);

        for (let i = 0; i < tweetCount; i++) {
            await scrollRandom(config.scroll, config.scroll);
            await api.wait(1000);
        }

        // Go back up slightly (finished reading)
        if (mathUtils.roll(0.5)) {
            await scrollRandom(-200, -100);
        }
    }

    /**
     * Skim media (image/video)
     */
    async _skimMedia(config) {
        // Quick glance at media
        await api.wait(1000);

        // Sometimes zoom or interact
        if (mathUtils.roll(0.2)) {
            // Simulate "looking closer"
            await scrollRandom(config.scroll, config.scroll);
        }

        await api.wait(1000);
    }

    /**
     * Skim profile
     */
    async _skimProfile(config) {
        // Check profile info
        await api.wait(config.pause);

        // Scroll through bio and recent tweets
        await scrollRandom(config.scroll * 2, config.scroll * 2);
        await api.wait(config.pause);

        // Sometimes check pinned tweet
        if (mathUtils.roll(0.3)) {
            await api.wait(config.pause * 2);
        }
    }

    /**
     * Micro-adjustments during "reading"
     */
    async _microAdjustments(count = 3) {
        for (let i = 0; i < count; i++) {
            // Tiny random movements
            const x = mathUtils.randomInRange(-20, 20);
            const y = mathUtils.randomInRange(-10, 10);
            await this.page.mouse.move(x, y);
            await api.wait(1000);
        }
    }

    // ==========================================
    // SPECIALIZED PATTERNS
    // ==========================================

    /**
     * "Skimming" pattern - quick scan through feed
     */
    async skimFeed() {
        const cycleCount = mathUtils.randomInRange(3, 6);

        for (let i = 0; i < cycleCount; i++) {
            // Quick scroll
            await scrollRandom(100, 200);

            // Brief pause (glance)
            await api.wait(1000);

            // Occasional stop (interesting content)
            if (mathUtils.roll(0.2)) {
                await api.wait(1000);
            }
        }
    }

    /**
     * "Deep reading" pattern - focus on interesting content
     */
    async deepRead() {
        // Settle in
        await api.wait(2000);

        // Read for extended period
        await api.wait(2000);

        // Occasional note-taking (bookmark/save)
        if (mathUtils.roll(0.2)) {
            // Simulate "bookmarking for later"
            await api.wait(2000);
        }
    }

    /**
     * "Quick glance" pattern - almost no time
     */
    async quickGlance() {
        await api.wait(1000);

        // Tiny movement to show "looking"
        await this.page.mouse.move(mathUtils.randomInRange(-10, 10), 0);
    }
}

export default ContentSkimmer;
