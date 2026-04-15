/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Local Edge browser discovery connector
 * @module connectors/discovery/localEdge
 */

import BaseDiscover from '../baseDiscover.js';

/**
 * @class LocalEdgeDiscover
 * @extends BaseDiscover
 * @description Discovers local Edge browser instances
 */
class LocalEdgeDiscover extends BaseDiscover {
    constructor() {
        super();
        this.browserType = 'localEdge';
    }

    /**
     * Discovers local Edge browser instances
     * @returns {Promise<Array>} Array of browser endpoints
     */
    async discover() {
        return [];
    }
}

export default LocalEdgeDiscover;
