/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Hook Helpers
 * wrappers that emit events using the current context's event emitter.
 *
 * @module api/hooks
 */

import { getEvents } from './context.js';

/**
 * Create a hook wrapper that emits before/after events.
 * @param {string} name - Action name (e.g., 'click')
 * @param {Function} fn - Original function
 * @param {object} [options]
 * @param {boolean} [options.emitBefore=true] - Emit before event
 * @param {boolean} [options.emitAfter=true] - Emit after event
 * @returns {Function} Wrapped function
 */
export function createHookWrapper(name, fn, options = {}) {
    const { emitBefore = true, emitAfter = true } = options;

    return async function (...args) {
        const events = getEvents();
        const beforeHook = `before:${name}`;
        const afterHook = `after:${name}`;

        if (emitBefore) {
            events.emitSafe(beforeHook, ...args);
        }

        try {
            const result = await fn.apply(this, args);

            if (emitAfter) {
                events.emitSafe(afterHook, ...args, result);
            }

            return result;
        } catch (error) {
            events.emitSafe('on:action:error', { action: name, error, args });
            throw error;
        }
    };
}

/**
 * Create an error handler hook.
 * @param {string} context - Context description
 * @param {Function} fn - Function that may throw
 * @returns {Promise<any>} Result or null on error
 */
export async function withErrorHook(context, fn) {
    const events = getEvents();
    try {
        return await fn();
    } catch (e) {
        events.emitSafe('on:error', { context, error: e });
        events.emitSafe('on:detection', { type: 'error', details: e.message });
        throw e;
    }
}
