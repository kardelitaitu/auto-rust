/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Engagement constants for social media interactions
 * @module constants/engagement
 */

export const TWITTER_CLICK_PROFILES = {
    like: {
        hoverMin: 800,
        hoverMax: 2000,
        holdMs: 150,
        hesitation: true,
        microMove: true,
    },
    reply: {
        hoverMin: 1500,
        hoverMax: 3000,
        holdMs: 200,
        hesitation: true,
        microMove: true,
    },
    retweet: {
        hoverMin: 1200,
        hoverMax: 2500,
        holdMs: 180,
        hesitation: true,
        microMove: true,
    },
    follow: {
        hoverMin: 2000,
        hoverMax: 4000,
        holdMs: 250,
        hesitation: true,
        microMove: false,
    },
    bookmark: {
        hoverMin: 1000,
        hoverMax: 2000,
        holdMs: 120,
        hesitation: false,
        microMove: false,
    },
    nav: {
        hoverMin: 200,
        hoverMax: 800,
        holdMs: 80,
        hesitation: false,
        microMove: false,
    },
};

export default { TWITTER_CLICK_PROFILES };
