/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Token estimation utility for context tracking.
 * Merged from local-agent/utils/tokenCounter.js
 * @module api/agent/tokenCounter
 */

/**
 * Estimates token count for a given text string.
 * @param {string} text - The text to estimate tokens for.
 * @returns {number} Estimated token count.
 */
export function estimateTokens(text) {
    if (!text || typeof text !== 'string') return 0;
    return Math.ceil(text.length / 4);
}

/**
 * Estimates tokens for a message object (with role and content).
 * @param {object} message - Message object with role and content.
 * @returns {number} Estimated token count.
 */
export function estimateMessageTokens(message) {
    if (!message) return 0;

    let total = 0;

    if (message.role) {
        total += estimateTokens(message.role);
    }

    if (typeof message.content === 'string') {
        total += estimateTokens(message.content);
    } else if (Array.isArray(message.content)) {
        for (const part of message.content) {
            if (part.type === 'text' && part.text) {
                total += estimateTokens(part.text);
            }
        }
    }

    return total;
}

/**
 * Estimates total tokens for an array of messages.
 * @param {Array} messages - Array of message objects.
 * @returns {number} Total estimated token count.
 */
export function estimateConversationTokens(messages) {
    if (!Array.isArray(messages)) return 0;
    return messages.reduce((sum, msg) => sum + estimateMessageTokens(msg), 0);
}
