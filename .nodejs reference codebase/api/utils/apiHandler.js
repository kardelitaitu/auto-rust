/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Centralized HTTP request handling mechanism with retry logic.
 * @module utils/apiHandler
 */

import { withRetry } from './retry.js';

/**
 * @class ApiHandler
 * @description A class for making HTTP requests with default headers and retry logic.
 * @param {string} [baseUrl=''] - The base URL for all requests.
 * @param {object} [defaultHeaders={}] - Default headers to be sent with every request.
 */
class ApiHandler {
    constructor(baseUrl = '', defaultHeaders = {}) {
        this.baseUrl = baseUrl;
        this.defaultHeaders = {
            'Content-Type': 'application/json',
            ...defaultHeaders,
        };
    }

    /**
     * Makes an HTTP request with retry logic.
     * @param {string} endpoint - The endpoint to make the request to.
     * @param {object} [options={}] - The options for the request (e.g., method, headers, body).
     * @returns {Promise<any>} A promise that resolves with the response data.
     * @throws {Error} If the request fails after all retries.
     */
    async request(endpoint, options = {}) {
        const url = endpoint.startsWith('http') ? endpoint : `${this.baseUrl}${endpoint}`;

        const config = {
            method: options.method || 'GET',
            headers: { ...this.defaultHeaders, ...options.headers },
            ...options,
        };

        return withRetry(
            async () => {
                const response = await fetch(url, config);

                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
                }

                const contentType = response.headers.get('content-type');
                if (contentType && contentType.includes('application/json')) {
                    return await response.json();
                } else {
                    return await response.text();
                }
            },
            { description: `API request to ${url}` }
        );
    }

    /**
     * Makes a GET request.
     * @param {string} endpoint - The endpoint to make the request to.
     * @param {object} [options={}] - The options for the request.
     * @returns {Promise<any>} A promise that resolves with the response data.
     */
    async get(endpoint, options = {}) {
        return this.request(endpoint, { ...options, method: 'GET' });
    }

    /**
     * Makes a POST request.
     * @param {string} endpoint - The endpoint to make the request to.
     * @param {object} data - The data to send in the request body.
     * @param {object} [options={}] - The options for the request.
     * @returns {Promise<any>} A promise that resolves with the response data.
     */
    async post(endpoint, data, options = {}) {
        return this.request(endpoint, {
            ...options,
            method: 'POST',
            body: JSON.stringify(data),
        });
    }

    /**
     * Makes a PUT request.
     * @param {string} endpoint - The endpoint to make the request to.
     * @param {object} data - The data to send in the request body.
     * @param {object} [options={}] - The options for the request.
     * @returns {Promise<any>} A promise that resolves with the response data.
     */
    async put(endpoint, data, options = {}) {
        return this.request(endpoint, {
            ...options,
            method: 'PUT',
            body: JSON.stringify(data),
        });
    }

    /**
     * Makes a DELETE request.
     * @param {string} endpoint - The endpoint to make the request to.
     * @param {object} [options={}] - The options for the request.
     * @returns {Promise<any>} A promise that resolves with the response data.
     */
    async delete(endpoint, options = {}) {
        return this.request(endpoint, { ...options, method: 'DELETE' });
    }
}

export default new ApiHandler();
export { ApiHandler };
