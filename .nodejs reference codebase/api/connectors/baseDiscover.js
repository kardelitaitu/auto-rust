/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Base class for all browser discovery connectors.
 * @module connectors/baseDiscover
 */

import { createLogger } from '../core/logger.js';

class BaseDiscover {
    constructor() {
        if (new.target === BaseDiscover) {
            throw new TypeError('Cannot construct BaseDiscover instances directly');
        }
        /**
         * @type {string}
         * @description The type of browser this connector discovers (e.g., 'ixbrowser', 'chrome').
         */
        this.browserType = 'base';
        /** @type {object} */
        this.logger = createLogger('baseDiscover.js');
        this.logger.info(`BaseDiscover initialized for type: ${this.browserType}`);
    }

    /**
     * Discovers running browser instances.
     * @async
     * @abstract
     * @returns {Promise<object[]>} A promise that resolves to an array of discovered browser profiles.
     * Each profile should have: id, name, type, ws or http endpoint.
     */
    async discover() {
        this.logger.info('Discover method called');
        throw new Error('Method "discover()" must be implemented by subclass');
    }
}

export default BaseDiscover;
