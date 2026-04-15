/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview AI Tools - Wrapper for AgentConnector with utility methods
 * @module utils/ai-tools
 */

import AgentConnector from '../core/agent-connector.js';
import { createLogger } from '../core/logger.js';
import { mathUtils } from '../utils/math.js';

const logger = createLogger('ai-tools.js');

/**
 * @class AITools
 * @description Utility class for working with AI services through AgentConnector
 */
class AITools {
    /**
     * Creates a new AITools instance
     * @param {object} config - Configuration options
     * @param {number} config.timeout - Request timeout in milliseconds (default: 30000)
     */
    constructor(config = {}) {
        this.connector = new AgentConnector();
        this.config = {
            timeout: config.timeout || 30000,
        };
        this.stats = {
            totalRequests: 0,
            successfulRequests: 0,
            failedRequests: 0,
            avgResponseTime: 0,
        };
        logger.info('[AITools] Initialized');
    }

    /**
     * Sends a request with timeout
     * @private
     */
    async sendWithTimeout(request) {
        const timeoutPromise = new Promise((_, reject) => {
            this.timeoutId = setTimeout(() => {
                reject(new Error(`Request timeout after ${this.config.timeout}ms`));
            }, this.config.timeout);
        });

        const requestPromise = this.connector.processRequest(request);

        try {
            const response = await Promise.race([requestPromise, timeoutPromise]);
            if (this.timeoutId) {
                clearTimeout(this.timeoutId);
                this.timeoutId = undefined;
            }
            return response;
        } catch (error) {
            if (this.timeoutId) {
                clearTimeout(this.timeoutId);
                this.timeoutId = undefined;
            }
            throw error;
        }
    }

    /**
     * Processes an AI request
     * @param {object} request - Request object
     * @returns {Promise<object>} Processed result
     */
    async processRequest(request) {
        const startTime = Date.now();
        this.stats.totalRequests++;

        try {
            const response = await this.sendWithTimeout(request);

            const duration = Date.now() - startTime;
            this.stats.successfulRequests++;
            this.updateAvgResponseTime(duration);

            return {
                success: true,
                data: response.data || response.content,
                raw: response.content,
                metadata: {
                    provider: response.metadata?.routedTo || 'unknown',
                    model: response.metadata?.model,
                    duration,
                },
            };
        } catch (error) {
            this.stats.failedRequests++;
            return {
                success: false,
                error: error.message,
                metadata: { duration: Date.now() - startTime },
            };
        }
    }

    /**
     * Queues a request for processing
     * @param {object} request - Request to queue
     * @returns {Promise<object>} Processed result
     */
    async queueRequest(request) {
        return this.processRequest(request);
    }

    /**
     * Generates a reply using AI
     * @param {string} tweet - Tweet text to reply to
     * @param {string} user - Username
     * @param {object} options - Generation options
     * @returns {Promise<object>} Reply result
     */
    async generateReply(tweet, user, options = {}) {
        const systemPrompt = options.systemPrompt || 'You are a neutral, casual Twitter user';
        const userPrompt = `Tweet from @${user}: "${tweet}"`;

        return this.queueRequest({
            action: 'generate_reply',
            payload: {
                systemPrompt,
                userPrompt,
                maxTokens: options.maxTokens || 100,
                temperature: options.temperature || 0.7,
            },
        });
    }

    /**
     * Analyzes a tweet
     * @param {string} tweet - Tweet text
     * @param {string} user - Username
     * @returns {Promise<object>} Analysis result
     */
    async analyzeTweet(tweet, user) {
        return this.queueRequest({
            action: 'analyze_tweet',
            payload: {
                text: tweet,
                user,
                context: {
                    textLength: tweet.length,
                    timestamp: Date.now(),
                },
            },
        });
    }

    /**
     * Classifies tweet content
     * @param {string} tweet - Tweet text
     * @returns {Promise<object>} Classification result
     */
    async classifyTweet(tweet) {
        return this.queueRequest({
            action: 'classify_content',
            payload: {
                text: tweet,
                categories: ['spam', 'promotional', 'organic', 'toxic'],
            },
        });
    }

    /**
     * Generates a conversation reply
     * @param {Array} history - Conversation history
     * @param {object} options - Generation options
     * @returns {Promise<object>} Reply result
     */
    async generateConversationReply(history, options = {}) {
        const systemPrompt = options.systemPrompt || 'You are a friendly conversationalist';

        return this.queueRequest({
            action: 'generate_conversation',
            payload: {
                systemPrompt,
                history,
                maxTokens: options.maxTokens || 100,
                temperature: options.temperature || 0.7,
            },
        });
    }

    /**
     * Gets a unique session ID
     * @returns {string} Session ID
     */
    getSessionId() {
        const timestamp = Date.now();
        const random = mathUtils.randomInRange(1000, 9999);
        return `session_${timestamp}_${random}`;
    }

    /**
     * Updates average response time
     * @private
     */
    updateAvgResponseTime(duration) {
        if (this.stats.successfulRequests === 1) {
            this.stats.avgResponseTime = duration;
        } else {
            const oldAvg = this.stats.avgResponseTime;
            const count = this.stats.successfulRequests;
            this.stats.avgResponseTime = Math.round((oldAvg * (count - 1) + duration) / count);
        }
    }

    /**
     * Gets formatted statistics
     * @returns {object} Formatted stats
     */
    getStats() {
        const { totalRequests, successfulRequests, failedRequests, avgResponseTime } = this.stats;
        const successRate =
            totalRequests > 0
                ? ((successfulRequests / totalRequests) * 100).toFixed(1) + '%'
                : '0%';

        return {
            totalRequests,
            successfulRequests,
            failedRequests,
            successRate,
            avgResponseTime: `${Math.round(avgResponseTime)}ms`,
        };
    }

    /**
     * Updates configuration
     * @param {object} config - New configuration
     */
    updateConfig(config) {
        if (config.timeout) {
            this.config.timeout = config.timeout;
        }
    }

    /**
     * Checks health of the AI service
     * @returns {Promise<boolean>} Health status
     */
    async isHealthy() {
        try {
            // For now, just check if connector is available
            return !!this.connector;
        } catch (error) {
            logger.error('[AITools] Health check failed:', error.message);
            return false;
        }
    }

    /**
     * Resets statistics
     */
    resetStats() {
        this.stats = {
            totalRequests: 0,
            successfulRequests: 0,
            failedRequests: 0,
            avgResponseTime: 0,
        };
    }
}

export { AITools };
export default AITools;
