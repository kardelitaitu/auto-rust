/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Discovery connector for Undetectable browser.
 * @module connectors/discovery/undetectable
 */

import BaseDiscover from '../baseDiscover.js';
import { createLogger } from '../../core/logger.js';
import { getEnv } from '../../utils/envLoader.js';
import apiHandler from '../../utils/apiHandler.js';

const logger = createLogger('undetectable.js');

/**
 * @class UndetectableDiscover
 * @extends BaseDiscover
 * @description Discovers running Undetectable browser instances by querying its local API.
 */
class UndetectableDiscover extends BaseDiscover {
    constructor() {
        super();
        this.browserType = 'undetectable';
        // Default port is 25325
        this.apiBaseUrl = getEnv('UNDETECTABLE_API_URL', 'http://127.0.0.1:25325/');

        // Normalize URL to ensure trailing slash
        if (!this.apiBaseUrl.endsWith('/')) {
            this.apiBaseUrl += '/';
        }

        logger.info(`Initialized UndetectableDiscover with API URL: ${this.apiBaseUrl}`);
    }

    /**
     * Discovers running Undetectable browser instances.
     *Queries the /list endpoint and checks for profiles with active websocket links.
     * @returns {Promise<object[]>} A promise that resolves with an array of discovered browser profiles.
     */
    async discover() {
        logger.info(`Starting discovery for ${this.browserType} browser type`);

        if (!this.apiBaseUrl) {
            logger.warn(`API Base URL not configured for ${this.browserType}. Skipping discovery.`);
            return [];
        }

        logger.info(`Discovering ${this.browserType} instances from: ${this.apiBaseUrl}`);

        try {
            // Undetectable API: /list returns all profiles
            // Response format: { code: 0, msg: "success", data: { "uuid": { name: "...", websocket_link: "...", ... } } }
            const response = await apiHandler.get(`${this.apiBaseUrl}list`);

            if (response.code !== 0 || !response.data) {
                logger.warn(
                    `Undetectable API returned error or empty data. Code: ${response.code}`
                );
                return [];
            }

            const profilesMap = response.data;
            const profileIds = Object.keys(profilesMap);

            logger.info(`Retrieved ${profileIds.length} total profiles from Undetectable API.`);

            // Filter for profiles that are actually running (have a websocket_link)
            const runningProfiles = profileIds.filter((id) => {
                const profile = profilesMap[id];
                // Check if websocket_link is present and not empty
                return profile.websocket_link && profile.websocket_link.trim() !== '';
            });

            if (runningProfiles.length === 0) {
                logger.info(
                    `No running ${this.browserType} profiles found (out of ${profileIds.length} total).`
                );
                return [];
            }

            logger.info(
                `Found ${runningProfiles.length} running profiles. Mapping to connector format...`
            );

            // Map to standard format
            const discoveredProfiles = runningProfiles.map((id, index) => {
                const profile = profilesMap[id];
                const wsEndpoint = profile.websocket_link;

                // Derive HTTP endpoint from WS endpoint if possible (usually just removing /devtools/...)
                // URL format: ws://127.0.0.1:PORT/devtools/browser/UUID
                let httpEndpoint = null;
                let port = null;
                try {
                    const url = new URL(wsEndpoint);
                    httpEndpoint = `http://${url.host}`;
                    port = parseInt(url.port, 10);
                } catch (_e) {
                    logger.warn(
                        `Failed to parse WS URL for profile ${profile.name}: ${wsEndpoint}`
                    );
                }

                return {
                    id: id, // The UUID from the key
                    name: profile.name || `Undetectable-${id.substring(0, 8)}`,
                    type: this.browserType,
                    ws: wsEndpoint,
                    http: httpEndpoint,
                    windowName: profile.name || `Undetectable-${index}`,
                    sortNum: index,
                    port: port,
                    // Undetectable specific metadata if needed
                    cloud_id: profile.cloud_id,
                    cloud_group: profile.cloud_group,
                };
            });

            discoveredProfiles.forEach((profile) => {
                logger.info(`Found connectable profile: ${profile.name} (ws: ${profile.ws})`);
            });

            return discoveredProfiles;
        } catch (error) {
            if (error.code === 'ECONNREFUSED') {
                logger.info(
                    `Undetectable API is not reachable at ${this.apiBaseUrl}. Is the application running?`
                );
            } else {
                logger.error(`Error during ${this.browserType} discovery: ${error.message}`);
            }
            return [];
        }
    }
}

export default UndetectableDiscover;
