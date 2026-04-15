/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Local Vivaldi browser discovery connector
 * @module connectors/discovery/localVivaldi
 */

import BaseDiscover from '../baseDiscover.js';

/**
 * @class LocalVivaldiDiscover
 * @extends BaseDiscover
 * @description Discovers local Vivaldi browser instances
 */
class LocalVivaldiDiscover extends BaseDiscover {
    constructor() {
        super();
        this.browserType = 'localVivaldi';
    }

    /**
     * Discovers local Vivaldi browser instances
     * @returns {Promise<Array>} Array of browser endpoints
     */
    async discover() {
        return [];
    }
}

export default LocalVivaldiDiscover;
