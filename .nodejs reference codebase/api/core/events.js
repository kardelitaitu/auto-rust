/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Event System Definitions
 * Provides event emitter class and hook definitions.
 *
 * @module api/events
 */

import { EventEmitter } from 'events';

const HOOKS = {
    // Lifecycle
    'before:init': 'Called before page initialization',
    'after:init': 'Called after page initialization completes',

    // Navigation
    'before:navigate': 'Called before navigation (goto)',
    'after:navigate': 'Called after navigation completes',
    'before:goBack': 'Called before going back',
    'after:goBack': 'Called after going back',
    'before:reload': 'Called before page reload',

    // Actions
    'before:click': 'Called before click action',
    'after:click': 'Called after click action',
    'before:type': 'Called before type action',
    'after:type': 'Called after type action',
    'before:hover': 'Called before hover action',
    'after:hover': 'Called after hover action',

    // Scroll
    'before:scroll': 'Called before scroll action',
    'after:scroll': 'Called after scroll action',
    'before:focus': 'Called before scroll focus',
    'after:focus': 'Called after scroll focus',

    // Cursor
    'before:cursor:move': 'Called before cursor movement',
    'after:cursor:move': 'Called after cursor movement',

    // Timing
    'before:think': 'Called before think delay',
    'before:delay': 'Called before delay',

    // Error & Recovery
    'on:error': 'Called on any error',
    'on:action:error': 'Called when an action fails',
    'on:recovery': 'Called during recovery attempt',
    'on:detection': 'Called when automation is detected',

    // Session
    'on:session:start': 'Called when session starts',
    'on:session:end': 'Called when session ends',
    'on:metrics': 'Called when metrics are collected',
};

/**
 * Get list of available hooks.
 * @returns {string[]} Array of hook names
 */
export function getAvailableHooks() {
    return Object.keys(HOOKS);
}

/**
 * Get hook description.
 * @param {string} hook - Hook name
 * @returns {string|undefined} Hook description
 */
export function getHookDescription(hook) {
    return HOOKS[hook];
}

export class APIEvents extends EventEmitter {
    constructor() {
        super();
        this.setMaxListeners(50);
    }

    /**
     * Emit hook with automatic error handling.
     * @param {string} hook - Hook name
     * @param {...any} args - Arguments to pass to handlers
     * @returns {Promise<any[]>} Results from all handlers (null for failures)
     */
    async emitAsync(hook, ...args) {
        const handlers = this.listeners(hook);
        const results = await Promise.allSettled(handlers.map((handler) => handler(...args)));
        return results.map((r) => {
            if (r.status === 'rejected') {
                console.error(
                    `[API Events] Error in ${hook} handler:`,
                    r.reason?.message || r.reason
                );
                return null;
            }
            return r.value;
        });
    }

    /**
     * Emit hook and throw if any handler fails.
     * @param {string} hook - Hook name
     * @param {...any} args - Arguments to pass to handlers
     * @returns {Promise<any[]>} Results from handlers
     * @throws {AggregateError} If any handler fails
     */
    async emitStrict(hook, ...args) {
        const handlers = this.listeners(hook);
        const results = await Promise.allSettled(handlers.map((handler) => handler(...args)));

        const errors = results.filter((r) => r.status === 'rejected').map((r) => r.reason);
        if (errors.length > 0) {
            throw new AggregateError(errors, `Errors in ${hook} handlers`);
        }

        return results.map((r) => r.value);
    }

    /**
     * Emit hook without waiting (fire and forget).
     * @param {string} hook - Hook name
     * @param {...any} args - Arguments to pass to handlers
     */
    emitSafe(hook, ...args) {
        process.nextTick(() => {
            this.emit(hook, ...args);
        });
    }

    /**
     * Sync version - calls handlers synchronously.
     * @param {string} hook - Hook name
     * @param {...any} args - Arguments to pass to handlers
     * @returns {any[]} Results from handlers
     */
    emitSync(hook, ...args) {
        const handlers = this.listeners(hook);
        return handlers.map((handler) => {
            try {
                return handler(...args);
            } catch (e) {
                console.error(`[API Events] Error in ${hook} handler:`, e.message);
                return null;
            }
        });
    }
}

export const apiEvents = new APIEvents();

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
    // Use the exported singleton

    return async function (...args) {
        const beforeHook = `before:${name}`;
        const afterHook = `after:${name}`;

        if (emitBefore) {
            apiEvents.emitSafe(beforeHook, ...args);
        }

        try {
            const result = await fn.apply(this, args);

            if (emitAfter) {
                apiEvents.emitSafe(afterHook, ...args, result);
            }

            return result;
        } catch (error) {
            apiEvents.emitSafe('on:action:error', { action: name, error, args });
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
    // Use the exported singleton
    try {
        return await fn();
    } catch (e) {
        apiEvents.emitSafe('on:error', { context, error: e });
        apiEvents.emitSafe('on:detection', { type: 'error', details: e.message });
        throw e;
    }
}

export default {
    APIEvents,
    apiEvents,
    getAvailableHooks,
    getHookDescription,
    createHookWrapper,
    withErrorHook,
};
