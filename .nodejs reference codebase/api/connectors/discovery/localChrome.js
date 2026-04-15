/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Local Chrome browser discovery connector
 * @module connectors/discovery/localChrome
 */

import BaseDiscover from '../baseDiscover.js';

/**
 * @class LocalChromeDiscover
 * @extends BaseDiscover
 * @description Discovers local Chrome browser instances
 */
class LocalChromeDiscover extends BaseDiscover {
    constructor() {
        super();
        this.browserType = 'localChrome';
    }

    /**
     * Discovers local Chrome browser instances
     * @returns {Promise<Array>} Array of browser endpoints
     */
    async discover() {
        // Placeholder for local Chrome discovery logic
        return [];
    }
}

export default LocalChromeDiscover;
