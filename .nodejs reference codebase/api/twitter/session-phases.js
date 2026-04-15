/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Session Phase Detection Module
 * Manages session phase transitions (warmup/active/cooldown)
 * and provides action modifiers based on current phase.
 *
 * @module utils/session-phases
 */

const SESSION_PHASES = {
    warmup: { start: 0, end: 0.1 },
    active: { start: 0.1, end: 0.8 },
    cooldown: { start: 0.8, end: 1.0 },
};

const PHASE_MODIFIERS = {
    warmup: {
        reply: 0.5,
        like: 0.4,
        retweet: 0.3,
        quote: 0.3,
        follow: 0.2,
        bookmark: 0.4,
        dive: 0.7,
        overall: 0.6,
    },
    active: {
        reply: 1.0,
        like: 1.0,
        retweet: 1.0,
        quote: 1.0,
        follow: 1.0,
        bookmark: 1.0,
        dive: 1.0,
        overall: 1.0,
    },
    cooldown: {
        reply: 0.6,
        like: 0.7,
        retweet: 0.5,
        quote: 0.4,
        follow: 0.3,
        bookmark: 0.6,
        dive: 0.8,
        overall: 0.4,
    },
};

function getSessionPhase(elapsedMs, totalMs) {
    const progress = elapsedMs / totalMs;

    if (progress < SESSION_PHASES.warmup.end) {
        return 'warmup';
    } else if (progress < SESSION_PHASES.cooldown.start) {
        return 'active';
    } else {
        return 'cooldown';
    }
}

function getPhaseModifier(action, phase) {
    const modifiers = PHASE_MODIFIERS[phase];
    if (!modifiers) {
        console.warn(`[session-phases.js] Unknown phase: ${phase}, using active modifiers`);
        return PHASE_MODIFIERS.active[action] || 1.0;
    }
    return modifiers[action] ?? 1.0;
}

function getOverallModifier(phase) {
    const modifiers = PHASE_MODIFIERS[phase];
    if (!modifiers) {
        return 1.0;
    }
    return modifiers.overall;
}

function getPhaseDescription(phase) {
    const descriptions = {
        warmup: 'Warming up - slower actions, more reading',
        active: 'Peak engagement - all actions available',
        cooldown: 'Slowing down - more reading, fewer actions',
    };
    return descriptions[phase] || 'Unknown phase';
}

function getPhaseStats(phase) {
    const modifiers = PHASE_MODIFIERS[phase];
    if (!modifiers) {
        return null;
    }
    return {
        phase,
        description: getPhaseDescription(phase),
        modifiers: { ...modifiers },
    };
}

function calculateRemainingTime(elapsedMs, totalMs) {
    return Math.max(0, totalMs - elapsedMs);
}

function isNearEnd(elapsedMs, totalMs, threshold = 0.9) {
    return elapsedMs / totalMs >= threshold;
}

function isNearStart(elapsedMs, totalMs, threshold = 0.1) {
    return elapsedMs / totalMs <= threshold;
}

export const sessionPhases = {
    phases: SESSION_PHASES,
    modifiers: PHASE_MODIFIERS,

    getSessionPhase,
    getPhaseModifier,
    getOverallModifier,
    getPhaseDescription,
    getPhaseStats,
    calculateRemainingTime,
    isNearEnd,
    isNearStart,
};

export default sessionPhases;
