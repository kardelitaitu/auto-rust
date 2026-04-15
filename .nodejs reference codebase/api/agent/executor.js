/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Agent Executor
 * Performs semantic actions on elements discovered via api.agent.see().
 * @module api/agent/executor
 */

import { click, type, hover, drag, clickAt, multiSelect, press } from '../interactions/actions.js';
import { getStateAgentElementMap } from '../core/context-state.js';
import { createLogger } from '../core/logger.js';

const logger = createLogger('api/agent/executor.js');

/**
 * Do - Performs a semantic action on an element.
 *
 * @param {string} action - Action to perform: 'click', 'type', 'hover', 'fill', 'drag', 'clickAt', 'multiSelect', 'press'
 * @param {string|number|Array} target - Element ID (from see()), Label, coordinates, or array for multiSelect
 * @param {string|Object} [value] - Value for 'type', 'fill', or options for 'drag', 'press'
 * @returns {Promise<any>} Result of the action
 * @example
 * await api.agent.do('click', 1);
 * await api.agent.do('type', 'Search', 'Playwright');
 * await api.agent.do('drag', 1, 2); // Drag from element 1 to element 2
 * await api.agent.do('clickAt', {x: 100, y: 200});
 * await api.agent.do('multiSelect', [1, 2, 3], { mode: 'add' });
 * await api.agent.do('press', 'Enter');
 */
export async function doAction(action, target, value) {
    const actionLower = action.toLowerCase();

    if (actionLower === 'clickat') {
        if (typeof target === 'object' && 'x' in target && 'y' in target) {
            return await clickAt(target.x, target.y, value);
        }
        throw new Error('clickAt requires {x, y} coordinates');
    }

    if (actionLower === 'press' || actionLower === 'key') {
        return await press(target, value);
    }

    if (actionLower === 'multiselect') {
        if (!Array.isArray(target)) {
            throw new Error('multiSelect requires an array of element IDs');
        }
        return await multiSelect(target, value || {});
    }

    const elementMap = getStateAgentElementMap();
    let element;

    if (typeof target === 'number') {
        element = elementMap.find((el) => el.id === target);
    } else if (typeof target === 'string') {
        const search = target.toLowerCase();
        element =
            elementMap.find((el) => el.label.toLowerCase() === search) ||
            elementMap.find((el) => el.label.toLowerCase().includes(search));
    }

    if (!element) {
        throw new Error(
            `Target element "${target}" not found in current view. Call api.agent.see() first.`
        );
    }

    logger.info(`Agent action: ${action} on "${element.label}" (${element.id})`);

    switch (actionLower) {
        case 'click':
            return await click(element.selector);
        case 'type':
        case 'fill':
            return await type(element.selector, value);
        case 'hover':
            return await hover(element.selector);
        case 'drag':
            if (!value) {
                throw new Error('drag requires a target element or coordinates');
            }
            return await drag(element.selector, value, {});
        default:
            throw new Error(`Unsupported agent action: ${action}`);
    }
}

export default doAction;
