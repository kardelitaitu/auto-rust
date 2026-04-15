/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Twitter Timeout Constants
 * Centralized timeout values for Twitter automation tasks
 * @module constants/twitter-timeouts
 */

export const TWITTER_TIMEOUTS = {
    PAGE_LOAD: 30000,
    ELEMENT_VISIBLE: 5000,
    ELEMENT_CLICKABLE: 10000,
    NAVIGATION: 15000,
    COMPOSER_OPEN: 8000,
    POST_SENT: 5000,
    AI_GENERATION: 60000,
    DIVE_TIMEOUT: 120000,
    QUICK_MODE_TIMEOUT: 30000,
    QUEUE_ITEM_TIMEOUT: 5000,
    FALLBACK_TIMEOUT: 3000,
};

export function importTimeouts(settings = {}) {
    return {
        ...TWITTER_TIMEOUTS,
        ...settings.timeouts?.twitter,
    };
}

export default TWITTER_TIMEOUTS;
