/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Proxy Agent for HTTP/HTTPS requests
 * Handles CONNECT tunneling for HTTPS through HTTP proxies
 * @module utils/proxy-agent
 */

import { createLogger } from '../core/logger.js';

const logger = createLogger('proxy-agent.js');

export function createProxyAgent(proxyUrl) {
    return new ProxyAgent(proxyUrl);
}

export class ProxyAgent {
    constructor(proxyUrl) {
        this.proxyUrl = proxyUrl;
        this.agent = null;
        this._parseProxyUrl();
    }

    _parseProxyUrl() {
        if (!this.proxyUrl) {
            return;
        }

        const url = new URL(this.proxyUrl);
        this.host = url.hostname;
        this.port = url.port;
        this.username = url.username ? decodeURIComponent(url.username) : null;
        this.password = url.password ? decodeURIComponent(url.password) : null;

        // logger.debug(`[ProxyAgent] Parsed proxy: ${this.host}:${this.port}`);
    }

    async getAgent() {
        if (this.agent) {
            return this.agent;
        }

        try {
            const { HttpsProxyAgent } = await import('https-proxy-agent');
            const auth =
                this.username && this.password
                    ? `${this.username}:${this.password}@${this.host}:${this.port}`
                    : `${this.host}:${this.port}`;

            this.agent = new HttpsProxyAgent(`http://${auth}`);
            // logger.debug(`[ProxyAgent] Created HTTPS proxy agent`);
            return this.agent;
        } catch (error) {
            logger.warn(`[ProxyAgent] Failed to create proxy agent: ${error.message}`);
            return null;
        }
    }

    static async fetchWithProxy(url, options, proxyUrl) {
        if (!proxyUrl) {
            return fetch(url, options);
        }

        const agent = createProxyAgent(proxyUrl);
        const httpAgent = await agent.getAgent();

        if (!httpAgent) {
            logger.warn(`[ProxyAgent] Falling back to direct connection`);
            return fetch(url, options);
        }

        return fetch(url, {
            ...options,
            agent: httpAgent,
        });
    }
}

export default ProxyAgent;
