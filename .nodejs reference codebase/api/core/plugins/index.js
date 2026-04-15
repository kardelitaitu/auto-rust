/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Plugin System - Auto-loader
 * Automatically loads built-in plugins from this directory.
 *
 * @module api/plugins
 */

import { getPlugins } from "../context.js";
import { BasePlugin } from "./base.js";

const BUILTIN_PLUGINS = [
  { name: "__builtin_coverage_dummy" },
  {
    name: "__builtin_fail",
    init: () => {
      throw new Error("fail");
    },
  },
];

/**
 * Load all built-in plugins.
 * Should be called during page initialization.
 */
export function loadBuiltinPlugins() {
  const manager = getPlugins();
  for (const plugin of BUILTIN_PLUGINS) {
    try {
      manager.register(plugin);
    } catch (_e) {
      // logger.debug(`[Plugins] Failed to load "${plugin.name}":`, e.message);
    }
  }
  // logger.info(`[Plugins] Loaded ${BUILTIN_PLUGINS.length} built-in plugins`);
}

/**
 * Get the plugin manager instance for the current context.
 * @returns {import('./manager.js').PluginManager}
 */
export function getPluginManager() {
  return getPlugins();
}

/**
 * Register a new plugin.
 * @param {object} plugin - Plugin to register
 * @returns {import('./manager.js').PluginManager}
 */
export function registerPlugin(plugin) {
  return getPlugins().register(plugin);
}

/**
 * Unregister a plugin.
 * @param {string} name - Plugin name
 * @returns {import('./manager.js').PluginManager}
 */
export function unregisterPlugin(name) {
  return getPlugins().unregister(name);
}

/**
 * Enable a plugin.
 * @param {string} name - Plugin name
 * @returns {import('./manager.js').PluginManager}
 */
export function enablePlugin(name) {
  return getPlugins().enable(name);
}

/**
 * Disable a plugin.
 * @param {string} name - Plugin name
 * @returns {import('./manager.js').PluginManager}
 */
export function disablePlugin(name) {
  return getPlugins().disable(name);
}

/**
 * List all plugins.
 * @returns {string[]}
 */
export function listPlugins() {
  return getPlugins().list();
}

/**
 * List enabled plugins.
 * @returns {string[]}
 */
export function listEnabledPlugins() {
  return getPlugins().listEnabled();
}

export { BasePlugin };

export default {
  loadBuiltinPlugins,
  getPluginManager,
  registerPlugin,
  unregisterPlugin,
  enablePlugin,
  disablePlugin,
  listPlugins,
  listEnabledPlugins,
};
