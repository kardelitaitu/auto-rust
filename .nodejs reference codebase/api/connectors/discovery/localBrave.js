/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Discovery connector for local Brave browser instances.
 * @module connectors/discovery/localBrave
 */

import BaseDiscover from '../baseDiscover.js';
import { createLogger } from '../../core/logger.js';

const logger = createLogger('localBrave.js');

/**
 * @class LocalBraveDiscover
 * @extends BaseDiscover
 * @description Discovers running local Brave browser instances by scanning a range of ports.
 */
class LocalBraveDiscover extends BaseDiscover {
    constructor() {
        super();
        this.browserType = 'localBrave';
        logger.info(`Initialized LocalBraveDiscover`);
    }

    /**
     * Discovers running local Brave browser instances.
     * @returns {Promise<object[]>} A promise that resolves with an array of discovered browser profiles.
     */
    async discover() {
        logger.info(`Discovering ${this.browserType} instances`);

        const discoveredProfiles = [];
        const bravePorts = Array.from({ length: 50 }, (_, i) => 9001 + i);

        logger.debug(`Scanning ${bravePorts.length} ports for Brave instances`);

        const portChecks = await Promise.allSettled(
            bravePorts.map(async (bravePort) => {
                try {
                    const cdpUrl = `http://127.0.0.1:${bravePort}/json`;

                    const response = await fetch(cdpUrl, { signal: AbortSignal.timeout(1000) });

                    if (!response.ok) {
                        return null;
                    }

                    const browserTabs = await response.json();

                    if (browserTabs.length === 0) {
                        return null;
                    }

                    logger.debug(
                        `Found ${browserTabs.length} browser tabs from Brave on port ${bravePort}`
                    );

                    let browserWsUrl = null;
                    try {
                        const versionResponse = await fetch(
                            `http://127.0.0.1:${bravePort}/json/version`,
                            {
                                signal: AbortSignal.timeout(1000),
                            }
                        );
                        const versionData = await versionResponse.json();

                        if (versionData.webSocketDebuggerUrl) {
                            browserWsUrl = versionData.webSocketDebuggerUrl;
                        }
                    } catch (_versionError) {
                        const firstTab = browserTabs[0];
                        if (firstTab?.webSocketDebuggerUrl) {
                            const urlMatch = firstTab.webSocketDebuggerUrl.match(
                                /(ws:\/\/[^/]+:\d+)\/devtools\/page\/[^/]+/
                            );
                            if (urlMatch) {
                                browserWsUrl = `${urlMatch[1]}/devtools/browser/${Date.now()}-${bravePort}`;
                            }
                        }
                    }

                    if (browserWsUrl) {
                        const braveProfile = {
                            id: `brave-local-${bravePort}`,
                            name: `Local Brave Browser (Port ${bravePort})`,
                            type: this.browserType,
                            ws: browserWsUrl,
                            http: cdpUrl,
                            windowName: `Brave-${bravePort}`,
                            sortNum: bravePort,
                            browserVersion: browserTabs[0]?.version || 'unknown',
                            userAgent: browserTabs[0]?.userAgent || 'Brave Browser',
                            port: bravePort,
                        };

                        logger.info(`Found connectable Brave instance: ${braveProfile.name}`);
                        return braveProfile;
                    }

                    return null;
                } catch (_error) {
                    return null;
                }
            })
        );

        for (const result of portChecks) {
            if (result.status === 'fulfilled' && result.value) {
                discoveredProfiles.push(result.value);
            }
        }

        if (discoveredProfiles.length > 0) {
            logger.info(
                `Discovery complete. Found ${discoveredProfiles.length} ${this.browserType} profiles.`
            );
        } else {
            logger.info(`No running ${this.browserType} instances found.`);
        }
        return discoveredProfiles;
    }
}

export default LocalBraveDiscover;
