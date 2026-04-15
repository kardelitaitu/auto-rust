/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Orchestrates the search for browser instances using all available connectors.
 * @module core/discovery
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { createLogger } from '../core/logger.js';

const logger = createLogger('discovery.js');

/**
 * @class Discovery
 * @description Discovers browser instances by loading and querying connectors.
 */
class Discovery {
    constructor() {
        /** @type {Array<{name: string, instance: object}>} */
        this.connectors = [];
        this.discoveryDir = path.join(
            path.dirname(fileURLToPath(import.meta.url)),
            '../connectors/discovery'
        );
    }

    /**
     * Loads all discovery connectors from the discovery directory.
     * @param {string[]} [allowedConnectors=[]] - Optional list of connector names to load (e.g., ['ixbrowser', 'localBrave']). If empty, loads all.
     * @returns {Promise<void>}
     */
    async loadConnectors(allowedConnectors = []) {
        try {
            let files = fs
                .readdirSync(this.discoveryDir)
                .filter((file) => file.endsWith('.js') && file !== 'baseDiscover.js')
                .map((file) => path.parse(file).name);

            if (allowedConnectors && allowedConnectors.length > 0) {
                const allowed = allowedConnectors.map((c) => c.toLowerCase());
                files = files.filter((f) => allowed.includes(f.toLowerCase()));
                logger.info(`Filtering connectors. Enabled: ${files.join(', ')}`);
            }

            logger.info(`Loading ${files.length} discovery connectors...`);

            for (const connectorName of files) {
                try {
                    const connectorPath = path.join(this.discoveryDir, `${connectorName}.js`);
                    const stat = fs.statSync(connectorPath);

                    if (stat.isFile()) {
                        logger.info(`[DISCOVERY DEBUG] Loading connector: ${connectorName}`);

                        // Use absolute file URL with cache busting to ensure fresh load
                        // const importUrl = fileURLToPath(import.meta.url); // Current file path
                        // We need to construct the file URL for the connector
                        // Since we already have the absolute path in connectorPath
                        const connectorFileUrl = `file://${connectorPath.replace(/\\/g, '/')}`;
                        const cacheBuster = `?t=${Date.now()}`;

                        const connectorModule = await import(`${connectorFileUrl}${cacheBuster}`);

                        // DEBUG: Log what we got
                        // if (connectorName === 'ixbrowser') {
                        //   logger.info(`DEBUG ixbrowser: module keys = ${Object.keys(connectorModule).join(', ')}`);
                        //   logger.info(`DEBUG ixbrowser: default = ${connectorModule.default}`);
                        //   logger.info(`DEBUG ixbrowser: typeof default = ${typeof connectorModule.default}`);
                        // }

                        const ConnectorClass = connectorModule.default;

                        if (ConnectorClass && typeof ConnectorClass === 'function') {
                            const connectorInstance = new ConnectorClass();
                            this.connectors.push({
                                name: connectorName,
                                instance: connectorInstance,
                            });
                            logger.info(`Loaded connector: ${connectorName}`);
                        } else {
                            logger.warn(
                                `Skipping ${connectorName}: invalid connector class export. Export was: ${typeof ConnectorClass}`
                            );
                        }
                    }
                } catch (error) {
                    logger.error(`Failed to load connector ${connectorName}:`, error.message);
                }
            }

            logger.info(`Successfully loaded ${this.connectors.length} connectors`);
        } catch (error) {
            logger.error('Failed to load connectors:', error.message);
            throw error;
        }
    }

    /**
     * Discovers browser instances using all loaded connectors.
     * @returns {Promise<object[]>} A promise that resolves with an array of discovered browser endpoints.
     */
    async discoverBrowsers() {
        if (this.connectors.length === 0) {
            logger.warn('No connectors loaded. Call loadConnectors() first.');
            return [];
        }

        logger.info(
            `Starting browser discovery with ${this.connectors.length} connectors in parallel...`
        );
        const allEndpoints = [];

        const discoveryPromises = this.connectors.map((connector) =>
            connector.instance.discover().then((endpoints) => ({
                connectorName: connector.name,
                endpoints: endpoints || [],
            }))
        );

        const results = await Promise.allSettled(discoveryPromises);

        for (const result of results) {
            if (result.status === 'fulfilled') {
                const { connectorName, endpoints } = result.value;
                if (Array.isArray(endpoints) && endpoints.length > 0) {
                    logger.info(`Connector ${connectorName} found ${endpoints.length} endpoints`);
                    allEndpoints.push(...endpoints);
                } else {
                    logger.info(`Connector ${connectorName} found no endpoints`);
                }
            } else {
                logger.error(`Error in a discovery connector:`, result.reason);
            }
        }

        logger.info(`Discovery complete. Found ${allEndpoints.length} total endpoints`);
        return allEndpoints;
    }

    /**
     * Gets information about the loaded connectors.
     * @returns {Array<{name: string, type: string}>} An array of objects containing connector information.
     */
    getConnectorInfo() {
        return this.connectors.map((connector) => ({
            name: connector.name,
            type: connector.instance.browserType || 'unknown',
        }));
    }
}

export default Discovery;
