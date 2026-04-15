/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Human Timing Utilities for api/ module.
 * Internal copy for api/ independence from utils/human-timing.js.
 * Provides Gaussian (bell curve) timing distribution with jitter.
 *
 * @module api/utils/timing
 */

const DEFAULT_JITTER = 0.15;
const DEFAULT_PAUSE_CHANCE = 0.08;
const DEFAULT_BURST_CHANCE = 0.05;

function gaussianRandom(mean, stdev) {
    const u1 = Math.random();
    const u2 = Math.random();
    const z = Math.sqrt(-2.0 * Math.log(u1)) * Math.cos(2.0 * Math.PI * u2);
    return mean + z * stdev;
}

function humanDelay(baseMs, options = {}) {
    const {
        jitter = DEFAULT_JITTER,
        pauseChance = DEFAULT_PAUSE_CHANCE,
        burstChance = DEFAULT_BURST_CHANCE,
        minDelay = 50,
    } = options;

    let delay = gaussianRandom(baseMs, baseMs * jitter);

    if (Math.random() < pauseChance) {
        delay *= 3;
    }

    if (Math.random() < burstChance) {
        delay *= 0.3;
    }

    return Math.max(minDelay, Math.round(delay));
}

function randomInRange(min, max) {
    return Math.floor(Math.random() * (max - min + 1)) + min;
}

function gaussianInRange(mean, stdev, min, max) {
    let value;
    let attempts = 0;
    const maxAttempts = 100;

    do {
        value = gaussianRandom(mean, stdev);
        attempts++;
    } while ((value < min || value > max) && attempts < maxAttempts);

    return Math.max(min, Math.min(max, Math.round(value)));
}

const READING_TIMES = {
    quick: { mean: 3000, stdev: 1000 },
    text: { mean: 5000, stdev: 2000 },
    image: { mean: 8000, stdev: 3000 },
    video: { mean: 15000, stdev: 5000 },
    thread: { mean: 30000, stdev: 10000 },
    longThread: { mean: 60000, stdev: 20000 },
};

function getReadingTime(contentType = 'text', options = {}) {
    const config = READING_TIMES[contentType] || READING_TIMES.text;
    const { min = 1000, max = 120000 } = options;
    return gaussianInRange(config.mean, config.stdev, min, max);
}

const ACTION_DELAYS = {
    quick: { mean: 500, stdev: 200 },
    like: { mean: 500, stdev: 200 },
    bookmark: { mean: 800, stdev: 300 },
    retweet: { mean: 2000, stdev: 800 },
    reply: { mean: 1500, stdev: 500 },
    follow: { mean: 5000, stdev: 2000 },
    dive: { mean: 1000, stdev: 400 },
    scroll: { mean: 300, stdev: 100 },
};

function getActionDelay(action, options = {}) {
    const config = ACTION_DELAYS[action] || ACTION_DELAYS.quick;
    const { min = 100, max = 10000 } = options;
    return gaussianInRange(config.mean, config.stdev, min, max);
}

function getWarmupDelay(options = {}) {
    const { min = 2000, max = 15000 } = options;
    return randomInRange(min, max);
}

function getScrollPause(options = {}) {
    const { min = 1500, max = 4000 } = options;
    return randomInRange(min, max);
}

function getScrollDuration(options = {}) {
    const { min = 300, max = 700 } = options;
    return randomInRange(min, max);
}

function getBetweenCycleDelay(options = {}) {
    const { min = 1000, max = 3000 } = options;
    return randomInRange(min, max);
}

function exponentialBackoff(attempt, baseDelay = 1000, maxDelay = 30000, factor = 2) {
    const delay = Math.min(baseDelay * Math.pow(factor, attempt), maxDelay);
    return humanDelay(delay, { jitter: 0.3 });
}

function jitterValue(value, factor = 0.1) {
    return gaussianRandom(value, value * factor);
}

function formatDuration(ms) {
    if (ms < 1000) return `${Math.round(ms)}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${(ms / 60000).toFixed(1)}m`;
}

export const humanTiming = {
    gaussianRandom,
    humanDelay,
    randomInRange,
    gaussianInRange,
    readingTimes: READING_TIMES,
    actionDelays: ACTION_DELAYS,
    getReadingTime,
    getActionDelay,
    getWarmupDelay,
    getScrollPause,
    getScrollDuration,
    getBetweenCycleDelay,
    exponentialBackoff,
    jitterValue,
    formatDuration,
    defaults: {
        jitter: DEFAULT_JITTER,
        pauseChance: DEFAULT_PAUSE_CHANCE,
        burstChance: DEFAULT_BURST_CHANCE,
    },
};

export default humanTiming;
