/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Discovery connector for ixbrowser.
 * @module connectors/discovery/ixbrowser
 */

import BaseDiscover from '../baseDiscover.js';
import { createLogger } from '../../core/logger.js';
import { getEnv } from '../../utils/envLoader.js';
import apiHandler from '../../utils/apiHandler.js';

const logger = createLogger('ixbrowser.js');

/**
 * @class IxbrowserDiscover
 * @extends BaseDiscover
 * @description Discovers running ixbrowser instances by querying its local API.
 */
class IxbrowserDiscover extends BaseDiscover {
    constructor() {
        super();
        this.browserType = 'ixbrowser';
        this.apiBaseUrl = getEnv('IXBROWSER_API_URL', 'http://127.0.0.1:53200');
        logger.info(`Initialized IxbrowserDiscover with API URL: ${this.apiBaseUrl}`);
    }

    /**
     * Discovers running ixbrowser instances.
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
            // Use the native-client endpoint which provides debugging ports directly
            const response = await apiHandler.post(
                `${this.apiBaseUrl}/api/v2/native-client-profile-opened-list`,
                {}
            );

            logger.info(
                `Received response - error code: ${response.error?.code}, message: ${response.error?.message}`
            );

            if (response.error?.code !== 0) {
                logger.warn(
                    `API returned error for ${this.browserType}: ${response.error?.message}`
                );
                return [];
            }

            // The data can be either an array or an object (check both)
            let profilesData = [];
            if (Array.isArray(response.data)) {
                profilesData = response.data;
            } else if (response.data && typeof response.data === 'object') {
                // If data is an object, check if it has profile entries
                profilesData = Object.values(response.data).filter(
                    (item) => item && item.profile_id
                );
            }

            if (profilesData.length === 0) {
                logger.info(
                    `No open profiles found for ${this.browserType}. Make sure profiles are opened via ixbrowser's interface with remote debugging enabled.`
                );
                return [];
            }

            logger.info(`Processing ${profilesData.length} opened profiles from API response`);

            // Map ixbrowser profiles to our standard format
            const discoveredProfiles = profilesData
                .filter((profile) => {
                    if (!profile.ws && !profile.debugging_port) {
                        logger.warn(
                            `Profile ${profile.profile_id} missing ws and debugging_port fields, skipping`
                        );
                        return false;
                    }
                    return true;
                })
                .map((profile) => ({
                    id: `ixbrowser-${profile.profile_id}`,
                    name: `ixBrowser Profile ${profile.profile_id}`,
                    type: this.browserType,
                    ws: profile.ws || `ws://${profile.debugging_address}/devtools/browser`,
                    http: `http://${profile.debugging_address}/json`,
                    windowName: `ixBrowser-${profile.profile_id}`,
                    sortNum: profile.profile_id,
                    port: profile.debugging_port,
                    pid: profile.pid,
                    openTime: profile.open_time,
                }));

            discoveredProfiles.forEach((profile) => {
                logger.info(
                    `Found connectable profile: ${profile.name} on port ${profile.port} (ws: ${profile.ws})`
                );
            });

            logger.info(
                `Discovery complete. Found ${discoveredProfiles.length} ${this.browserType} profiles.`
            );
            return discoveredProfiles;
        } catch (error) {
            logger.error(`Error during ${this.browserType} discovery: ${error.message}`);
            return [];
        }
    }
}

export default IxbrowserDiscover;
