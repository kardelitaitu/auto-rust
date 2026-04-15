/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Discovery connector for Roxybrowser.
 * @module connectors/discovery/roxybrowser
 */

import BaseDiscover from '../baseDiscover.js';
import { createLogger } from '../../core/logger.js';
import { getEnv } from '../../utils/envLoader.js';
import apiHandler from '../../utils/apiHandler.js';

const logger = createLogger('roxybrowser.js');

/**
 * @class RoxybrowserDiscover
 * @extends BaseDiscover
 * @description Discovers running Roxybrowser instances by querying its local API.
 */
class RoxybrowserDiscover extends BaseDiscover {
    constructor() {
        super();
        this.browserType = 'roxybrowser';
        this.apiBaseUrl = getEnv('ROXYBROWSER_API_URL', 'http://127.0.0.1:50000/');
        if (this.apiBaseUrl && !this.apiBaseUrl.endsWith('/')) {
            this.apiBaseUrl += '/';
        }
        this.API_KEY = getEnv('ROXYBROWSER_API_KEY', 'c6ae203adfe0327a63ccc9174c178dec');

        if (this.API_KEY) {
            logger.info(`Initialized RoxybrowserDiscover with API URL: ${this.apiBaseUrl}`);
        } else {
            logger.warn('ROXYBROWSER_API_KEY not set in environment variables');
        }
    }

    /**
     * Discovers running Roxybrowser instances.
     * @returns {Promise<object[]>} A promise that resolves with an array of discovered browser profiles.
     */
    async discover() {
        logger.info(`Starting discovery for ${this.browserType} browser type`);

        if (!this.apiBaseUrl) {
            logger.warn(`API Base URL not configured for ${this.browserType}. Skipping discovery.`);
            return [];
        }

        if (!this.API_KEY) {
            logger.warn(`API Key not configured for ${this.browserType}. Skipping discovery.`);
            return [];
        }

        logger.info(`Discovering ${this.browserType} instances from: ${this.apiBaseUrl}`);

        try {
            const profileInfo = await apiHandler.get(`${this.apiBaseUrl}browser/connection_info`, {
                headers: {
                    'X-API-Key': this.API_KEY,
                },
            });

            logger.info(
                `Received profile info with code: ${profileInfo.code}, message: ${profileInfo.msg || 'none'}`
            );

            if (
                profileInfo.code !== 0 ||
                !profileInfo.data ||
                !Array.isArray(profileInfo.data) ||
                profileInfo.data.length === 0
            ) {
                logger.info(`API returned no open profiles for ${this.browserType}.`);
                return [];
            }

            logger.info(`Processing ${profileInfo.data.length} profiles from API response`);

            // Map roxybrowser profiles to standard format
            const discoveredProfiles = profileInfo.data
                .filter((profile) => {
                    if (!profile.ws && !profile.http) {
                        logger.warn(
                            `Profile ${profile.sortNum || 'unknown'} missing ws and http fields, skipping`
                        );
                        return false;
                    }
                    return true;
                })
                .map((profile, index) => ({
                    id: profile.id || `roxybrowser-${profile.sortNum || index}`,
                    name:
                        profile.name ||
                        profile.windowName ||
                        `Roxybrowser Profile ${profile.sortNum || index}`,
                    type: this.browserType,
                    ws: profile.ws,
                    http: profile.http,
                    windowName: profile.windowName || `Roxybrowser-${profile.sortNum || index}`,
                    sortNum: profile.sortNum || index,
                    port: profile.port,
                    userAgent: profile.userAgent,
                    browserVersion: profile.browserVersion,
                }));

            discoveredProfiles.forEach((profile) => {
                logger.info(`Found connectable profile: ${profile.name} (ws: ${profile.ws})`);
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

export default RoxybrowserDiscover;
