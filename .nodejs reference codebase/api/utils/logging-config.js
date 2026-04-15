/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Logging Configuration Loader
 * Reads logging settings from config/settings.json
 */

import { getSettings } from './configLoader.js';

let loggingConfig = null;

/**
 * Load logging configuration from settings
 */
export async function getLoggingConfig() {
    if (loggingConfig) return loggingConfig;

    try {
        const settings = await getSettings();
        loggingConfig = settings?.logging || getDefaultLoggingConfig();
        return loggingConfig;
    } catch {
        console.warn('[logging-config] Failed to load logging config, using defaults');
        return getDefaultLoggingConfig();
    }
}

/**
 * Get default logging configuration
 */
export function getDefaultLoggingConfig() {
    return {
        engagementProgress: {
            enabled: true,
            showProgressBar: true,
            progressBarStyle: 'blocks',
            showPercent: true,
            showCounts: true,
            types: {
                replies: { show: true, label: 'replies' },
                retweets: { show: true, label: 'retweets' },
                quotes: { show: true, label: 'quotes' },
                likes: { show: true, label: 'likes' },
                follows: { show: true, label: 'follows' },
                bookmarks: { show: true, label: 'bookmarks' },
            },
        },
        finalStats: {
            enabled: true,
            showQueueStatus: true,
            showEngagement: true,
            showDuration: true,
        },
        queueMonitor: {
            enabled: true,
            interval: 30000,
        },
    };
}

/**
 * Generate progress bar string
 * @param {number} percent - 0-100
 * @param {number} length - Bar length (default 10)
 * @param {string} style - 'blocks' | 'stars' | 'equals' | 'simple'
 */
export function generateProgressBar(percent, length = 10, style = 'blocks') {
    const filled = Math.round((percent / 100) * length);
    const empty = length - filled;

    const styles = {
        blocks: { filled: '█', empty: '░' },
        stars: { filled: '★', empty: '☆' },
        equals: { filled: '=', empty: '-' },
        simple: { filled: '#', empty: '.' },
    };

    const chars = styles[style] || styles.blocks;
    return chars.filled.repeat(filled) + chars.empty.repeat(empty);
}

/**
 * Format engagement progress line
 */
export function formatEngagementLine(action, data, config) {
    const typeConfig = config?.types?.[action];
    if (!typeConfig?.show) return null;

    const label = typeConfig?.label || action;
    let line = `   ${label.padEnd(10)}: `;

    if (config?.showProgressBar) {
        const bar = generateProgressBar(data.percentUsed, 10, config.progressBarStyle);
        line += `[${bar}] `;
    }

    if (config?.showCounts) {
        line += `${data.current}/${data.limit}`;
    }

    if (config?.showPercent) {
        line += ` (${data.percentUsed}%)`;
    }

    return line;
}

/**
 * Format all engagement progress into a single summary line
 */
export function formatEngagementSummary(engagementProgress, config) {
    const parts = [];

    for (const [action, data] of Object.entries(engagementProgress)) {
        const typeConfig = config?.types?.[action];
        if (!typeConfig?.show) continue;

        const label = typeConfig?.label || action;
        let part = `${label}: ${data.current}/${data.limit}`;

        if (config?.showPercent) {
            part += ` (${data.percentUsed}%)`;
        }

        parts.push(`[${part}]`);
    }

    return parts.join(' ');
}

/**
 * Reset cached config (useful for testing)
 */
export function resetLoggingConfig() {
    loggingConfig = null;
}

export default {
    getLoggingConfig,
    getDefaultLoggingConfig,
    generateProgressBar,
    formatEngagementLine,
    formatEngagementSummary,
    resetLoggingConfig,
};
