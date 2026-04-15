/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Context-Aware State Management
 * Moves module-level globals into AsyncLocalStorage for session isolation.
 * Enables concurrent multi-session usage without state pollution.
 *
 * @module api/context-state
 */

import { PERSONAS } from '../behaviors/persona-defs.js';

const DEFAULT_STATE = {
    persona: {
        name: 'casual',
        config: { ...PERSONAS.casual },
        sessionStartTime: Date.now(),
    },
    pathStyle: {
        style: 'bezier',
        options: {},
    },
    attention: {
        distractionChance: 0.2,
        memory: [],
    },
    idle: {
        fidgetInterval: null,
        isRunning: false,
    },
    session: {
        previousUrl: null,
    },
    agent: {
        elementMap: [],
    },
    automation: {
        autoBanners: false,
    },
    audio: {
        mute: false,
    },
    retryBudget: {
        used: 0,
        max: 50,
    },
};

const MAX_ATTENTION_MEMORY = 3;
let contextStoreRef = null;

export function setContextStore(store) {
    contextStoreRef = store;
}

export function getDefaultState() {
    return JSON.parse(JSON.stringify(DEFAULT_STATE));
}

/**
 * Get the current context state, creating default if not exists.
 * @returns {object} Current context state
 */
export function getContextState() {
    const store = contextStoreRef?.getStore?.();
    if (!store || !store.state) {
        return getDefaultState();
    }
    return store.state;
}

/**
 * Set the entire context state (rarely needed).
 * @param {object} state - State object to set
 */
export function setContextState(state) {
    const store = contextStoreRef?.getStore?.();
    if (store) {
        store.state = state;
    }
}

/**
 * Get a specific section of context state.
 * @param {string} section - Section name: 'persona', 'pathStyle', 'attention', 'idle', 'session'
 * @returns {object} The requested section
 */
export function getStateSection(section) {
    const state = getContextState();
    return state[section] || getDefaultState()[section];
}

/**
 * Update a specific section of context state.
 * @param {string} section - Section name
 * @param {object} values - Values to merge
 */
export function updateStateSection(section, values) {
    const state = getContextState();
    state[section] = { ...state[section], ...values };
    setContextState(state);
}

// ─── Persona Section ─────────────────────────────────────────────────

/**
 * Get current persona config.
 * @returns {object} Persona config object
 */
export function getStatePersona() {
    return getStateSection('persona').config;
}

/**
 * Get current persona name.
 * @returns {string} Persona name
 */
export function getStatePersonaName() {
    return getStateSection('persona').name;
}

/**
 * Set persona (context-aware).
 * @param {string} name - Persona name
 * @param {object} [overrides] - Optional overrides
 */
export function setStatePersona(name, overrides = {}) {
    const base = PERSONAS[name];
    if (!base && name !== 'custom') {
        throw new Error(
            `Unknown persona "${name}". Available: ${Object.keys(PERSONAS).join(', ')}`
        );
    }

    const baseConfig = base || PERSONAS.casual;
    let config = { ...baseConfig, ...overrides };

    // Session-level biometric randomization
    if (config.muscleModel) {
        const drift = 0.1;
        config.muscleModel = {
            ...config.muscleModel,
            Kp: config.muscleModel.Kp * (1 + (Math.random() * drift * 2 - drift)),
            Ki: config.muscleModel.Ki * (1 + (Math.random() * drift * 2 - drift)),
            Kd: config.muscleModel.Kd * (1 + (Math.random() * drift * 2 - drift)),
        };
    }

    updateStateSection('persona', {
        name,
        config,
    });
}

// ─── Path Style Section ───────────────────────────────────────────

/**
 * Get current path style.
 * @returns {string} Path style name
 */
export function getStatePathStyle() {
    return getStateSection('pathStyle').style;
}

/**
 * Get current path options.
 * @returns {object} Path options
 */
export function getStatePathOptions() {
    return getStateSection('pathStyle').options;
}

/**
 * Set path style (context-aware).
 * @param {string} style - Path style name
 * @param {object} [options] - Style options
 */
export function setStatePathStyle(style, options = {}) {
    const validStyles = ['bezier', 'arc', 'zigzag', 'overshoot', 'stopped', 'muscle'];
    if (!validStyles.includes(style)) {
        throw new Error(`Invalid path style: ${style}. Valid: ${validStyles.join(', ')}`);
    }
    updateStateSection('pathStyle', { style, options });
}

// ─── Attention Section ───────────────────────────────────────────

/**
 * Get distraction chance.
 * @returns {number} Chance 0-1
 */
export function getStateDistractionChance() {
    return getStateSection('attention').distractionChance;
}

/**
 * Set distraction chance (context-aware).
 * @param {number} chance - 0-1
 */
export function setStateDistractionChance(chance) {
    updateStateSection('attention', {
        distractionChance: Math.max(0, Math.min(1, chance)),
    });
}

/**
 * Get attention memory (recent selectors).
 * @returns {string[]} Array of selectors
 */
export function getStateAttentionMemory() {
    return getStateSection('attention').memory;
}

/**
 * Record selector in attention memory.
 * @param {string} selector - CSS selector
 */
export function recordStateAttentionMemory(selector) {
    if (!selector) return;

    const state = getContextState();
    const memory = state.attention.memory;

    const idx = memory.indexOf(selector);
    if (idx !== -1) {
        memory.splice(idx, 1);
    }
    memory.unshift(selector);

    if (memory.length > MAX_ATTENTION_MEMORY) {
        memory.pop();
    }

    updateStateSection('attention', { memory });
}

// ─── Idle Section ─────────────────────────────────────────────────

/**
 * Get idle state.
 * @returns {object} Idle state
 */
export function getStateIdle() {
    return getStateSection('idle');
}

/**
 * Set idle state (context-aware).
 * @param {object} idleState - Idle state object
 */
export function setStateIdle(idleState) {
    updateStateSection('idle', idleState);
}

// ─── Session Section ──────────────────────────────────────────────

/**
 * Get previous URL for recovery.
 * @returns {string|null} Previous URL
 */
export function getPreviousUrl() {
    return getStateSection('session').previousUrl;
}

/**
 * Set previous URL.
 * @param {string} url - URL to store
 */
export function setPreviousUrl(url) {
    updateStateSection('session', { previousUrl: url });
}

// ─── Agent Section ──────────────────────────────────────────────

/**
 * Get current agent element map.
 * @returns {object[]} Array of element objects
 */
export function getStateAgentElementMap() {
    return getStateSection('agent').elementMap;
}

/**
 * Set agent element map.
 * @param {object[]} elementMap - Array of element objects
 */
export function setStateAgentElementMap(elementMap) {
    updateStateSection('agent', { elementMap });
}

// ─── Automation Section ───────────────────────────────────────────

/**
 * Get auto-banners state.
 * @returns {boolean}
 */
export function getAutoBanners() {
    return getStateSection('automation').autoBanners;
}

/**
 * Set auto-banners state.
 * @param {boolean} enabled
 */
export function setAutoBanners(enabled) {
    updateStateSection('automation', { autoBanners: !!enabled });
}

// ─── Audio Section ────────────────────────────────────────────────

/**
 * Get audio mute state.
 * @returns {boolean}
 */
export function getMuteAudio() {
    return getStateSection('audio').mute;
}

/**
 * Set audio mute state.
 * @param {boolean} mute
 */
export function setMuteAudio(mute) {
    updateStateSection('audio', { mute: !!mute });
}

export default {
    getContextState,
    setContextState,
    getStateSection,
    updateStateSection,
    getStatePersona,
    getStatePersonaName,
    setStatePersona,
    getStatePathStyle,
    getStatePathOptions,
    setStatePathStyle,
    getStateDistractionChance,
    setStateDistractionChance,
    getStateAttentionMemory,
    recordStateAttentionMemory,
    getStateIdle,
    setStateIdle,
    getPreviousUrl,
    setPreviousUrl,
    getStateAgentElementMap,
    setStateAgentElementMap,
    getAutoBanners,
    setAutoBanners,
    getMuteAudio,
    setMuteAudio,
};
