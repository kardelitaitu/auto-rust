/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Interaction Handler for Twitter agent
 * @module utils/twitter-agent/InteractionHandler
 */

import { BaseHandler } from './BaseHandler.js';

/**
 * Handles user interactions with Twitter content.
 * @class
 * @extends BaseHandler
 */
export class InteractionHandler extends BaseHandler {
    /**
     * Creates an InteractionHandler instance.
     * @param {Object} agent - The parent Twitter agent instance
     */
    constructor(agent) {
        super(agent);
    }
}
