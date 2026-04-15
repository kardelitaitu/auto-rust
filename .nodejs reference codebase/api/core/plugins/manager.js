/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Plugin Manager
 * Manages registration, lifecycle, and hook binding for plugins.
 *
 * @module api/plugins/manager
 */

import { getAvailableHooks } from '../events.js';
import { createLogger } from '../../core/logger.js';
import { BasePlugin } from './base.js';

const logger = createLogger('api/plugins/manager.js');

const PLUGIN_HOOKS = getAvailableHooks();

export class PluginManager {
    #plugins = new Map();
    #hookBindings = new Map();
    #enabled = new Set();
    #events;

    constructor(events) {
        if (!events) throw new Error('PluginManager requires an APIEvents instance');
        this.#events = events;
        this.#initializeHookBindings();
    }

    #initializeHookBindings() {
        for (const hook of PLUGIN_HOOKS) {
            this.#hookBindings.set(hook, new Set());
        }
    }

    /**
     * Register a plugin.
     * @param {object} plugin - Plugin object
     * @param {string} plugin.name - Unique plugin name
     * @param {string} [plugin.version] - Plugin version
     * @param {object} [plugin.hooks] - Hook handlers
     * @param {object} [plugin.config] - Plugin configuration
     * @param {Function} [plugin.init] - Initialization function
     * @param {Function} [plugin.destroy] - Cleanup function
     * @returns {this} For chaining
     * @throws {Error} If plugin name already registered
     */
    register(plugin) {
        if (!plugin || typeof plugin !== 'object') {
            throw new Error('Plugin must be an object');
        }
        if (!plugin.name || typeof plugin.name !== 'string') {
            throw new Error('Plugin must have a name string');
        }
        if (this.#plugins.has(plugin.name)) {
            throw new Error(`Plugin "${plugin.name}" is already registered`);
        }

        // Validate hook handlers
        let hooks;
        if (plugin instanceof BasePlugin) {
            hooks = plugin.getHooks();
            plugin.context = { events: this.#events }; // Inject minimal context
        } else {
            hooks = plugin.hooks;
            if (hooks && typeof hooks !== 'object') {
                throw new Error('Plugin hooks must be an object');
            }
        }

        // Store plugin
        this.#plugins.set(plugin.name, {
            instance: plugin, // Keep reference to original instance
            name: plugin.name,
            version: plugin.version,
            registeredAt: Date.now(),
            enabled: true,
            hooks: hooks, // Store normalized hooks
        });

        // Bind hooks if present
        if (hooks) {
            this.#bindPluginHooks(plugin.name, hooks);
        }

        // Call init/onLoad
        try {
            if (plugin instanceof BasePlugin) {
                // Standard Lifecycle
                plugin.onLoad({ events: this.#events });
                plugin.onEnable();
            } else if (plugin.init) {
                // Legacy Lifecycle
                plugin.init(plugin.config || {});
            }
            logger.debug(`[Plugin] Initialized: ${plugin.name}`);
        } catch (e) {
            logger.warn(`[Plugin] Init failed for "${plugin.name}":`, e.message);
        }

        // Auto-enable
        this.#enabled.add(plugin.name);

        logger.info(`[Plugin] Registered: ${plugin.name}`);
        return this;
    }

    #bindPluginHooks(pluginName, hooks) {
        for (const [hook, handler] of Object.entries(hooks)) {
            if (!PLUGIN_HOOKS.includes(hook)) {
                logger.warn(`[Plugin] Unknown hook "${hook}" in "${pluginName}"`);
                continue;
            }

            const wrappedHandler = async (...args) => {
                if (!this.isEnabled(pluginName)) return;

                try {
                    const result = await handler(...args);
                    return result;
                } catch (e) {
                    logger.error(`[Plugin] Error in ${hook} for "${pluginName}":`, e.message);
                    throw e;
                }
            };

            this.#events.on(hook, wrappedHandler);
            this.#hookBindings.get(hook).add({ plugin: pluginName, handler: wrappedHandler });
        }
    }

    /**
     * Unregister a plugin.
     * @param {string} name - Plugin name
     * @returns {this} For chaining
     */
    unregister(name) {
        const record = this.#plugins.get(name);
        if (!record) {
            logger.warn(`[Plugin] Cannot unregister "${name}": not found`);
            return this;
        }

        const { instance } = record;

        // Call destroy/onUnload
        try {
            if (instance instanceof BasePlugin) {
                if (this.isEnabled(name)) {
                    instance.onDisable();
                }
                instance.onUnload();
            } else if (instance.destroy) {
                instance.destroy();
            }
            logger.debug(`[Plugin] Destroyed: ${name}`);
        } catch (e) {
            logger.warn(`[Plugin] Destroy failed for "${name}":`, e.message);
        }

        // Remove hook bindings
        for (const [hook, bindings] of this.#hookBindings) {
            for (const binding of bindings) {
                if (binding.plugin === name) {
                    this.#events.off(hook, binding.handler);
                    bindings.delete(binding);
                }
            }
        }

        // Remove from enabled set
        this.#enabled.delete(name);

        // Remove plugin
        this.#plugins.delete(name);
        logger.info(`[Plugin] Unregistered: ${name}`);
        return this;
    }

    /**
     * Enable a plugin.
     * @param {string} name - Plugin name
     * @returns {this} For chaining
     */
    enable(name) {
        const record = this.#plugins.get(name);
        if (record) {
            if (!this.#enabled.has(name)) {
                this.#enabled.add(name);
                if (record.instance instanceof BasePlugin) {
                    record.instance.onEnable();
                }
                logger.debug(`[Plugin] Enabled: ${name}`);
            }
        }
        return this;
    }

    /**
     * Disable a plugin.
     * @param {string} name - Plugin name
     * @returns {this} For chaining
     */
    disable(name) {
        const record = this.#plugins.get(name);
        if (record) {
            if (this.#enabled.has(name)) {
                this.#enabled.delete(name);
                if (record.instance instanceof BasePlugin) {
                    record.instance.onDisable();
                }
                logger.debug(`[Plugin] Disabled: ${name}`);
            }
        }
        return this;
    }

    /**
     * Check if plugin is enabled.
     * @param {string} name - Plugin name
     * @returns {boolean}
     */
    isEnabled(name) {
        return this.#enabled.has(name);
    }

    /**
     * Evaluate all registered plugins against a URL and enable/disable them.
     * @param {string} url
     */
    evaluateUrl(url) {
        for (const [name, record] of this.#plugins) {
            const instance = record.instance;
            let shouldEnable = true;
            if (typeof instance.matches === 'function') {
                try {
                    shouldEnable = instance.matches(url);
                } catch (e) {
                    logger.warn(`[Plugin] matches() failed for "${name}":`, e.message);
                }
            }

            if (shouldEnable) {
                this.enable(name);
            } else {
                this.disable(name);
            }
        }
    }

    /**
     * Get plugin by name.
     * @param {string} name - Plugin name
     * @returns {object|undefined}
     */
    get(name) {
        return this.#plugins.get(name);
    }

    /**
     * Get all plugin names.
     * @returns {string[]}
     */
    list() {
        return [...this.#plugins.keys()];
    }

    /**
     * Get enabled plugin names.
     * @returns {string[]}
     */
    listEnabled() {
        return [...this.#enabled];
    }

    /**
     * Get plugin info (without handlers).
     * @returns {object[]}
     */
    listInfo() {
        return [...this.#plugins.values()].map((p) => ({
            name: p.name,
            version: p.version,
            enabled: this.#enabled.has(p.name),
            registeredAt: p.registeredAt,
        }));
    }

    /**
     * Unregister all plugins.
     * @returns {this}
     */
    clear() {
        for (const name of this.#plugins.keys()) {
            this.unregister(name);
        }
        return this;
    }

    /**
     * Destroy the plugin manager - unregister all plugins and clear bindings.
     * @returns {this}
     */
    destroy() {
        this.clear();
        for (const hook of PLUGIN_HOOKS) {
            this.#hookBindings.set(hook, new Set());
        }
        logger.info('[PluginManager] Destroyed');
        return this;
    }
}

export default PluginManager;
