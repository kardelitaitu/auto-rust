/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Agent Finder
 * Helps find elements based on fuzzy descriptions.
 * @module api/agent/finder
 */

import { getStateAgentElementMap } from '../core/context-state.js';

/**
 * Find - Locates an element based on a fuzzy description.
 *
 * @param {string} description - Descriptive text or label
 * @returns {Promise<object|null>} Element object or null
 * @example
 * const btn = await api.agent.find('post button');
 */
export async function find(description) {
    const elementMap = getStateAgentElementMap();
    if (!elementMap || elementMap.length === 0) return null;

    const search = description.toLowerCase();

    // 1. Exact match
    let match = elementMap.find((el) => el.label.toLowerCase() === search);
    if (match) return match;

    // 2. Inclusion match
    match = elementMap.find((el) => el.label.toLowerCase().includes(search));
    if (match) return match;

    // 3. Role + Label match (e.g. "button post")
    match = elementMap.find((el) => `${el.role} ${el.label}`.toLowerCase().includes(search));
    if (match) return match;

    return null;
}

export default find;
