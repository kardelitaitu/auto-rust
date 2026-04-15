/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Settings Config Loader Bridge for api/ module.
 *
 * This module provides a simple interface to load settings from settings.json.
 * It wraps the full ConfigLoader class for API compatibility.
 *
 * Usage:
 *   import { getSettings } from '@api/utils/config.js';
 *   const settings = await getSettings();
 *
 * @module api/utils/config
 */

import { ConfigLoader } from "./configLoader.js";

const configLoader = new ConfigLoader();

/**
 * Load settings.json using the full ConfigLoader.
 * @returns {Promise<object>}
 */
export async function getSettings() {
  return configLoader.getSettings();
}

/**
 * Clear settings cache.
 */
export function clearSettingsCache() {
  configLoader.clearCache();
}
