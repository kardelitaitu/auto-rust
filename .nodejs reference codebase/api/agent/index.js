/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unified Agent API
 * Combines semantic actions (see/do) with LLM-driven automation (run/stop)
 * @module api/agent
 */

// Semantic Layer - Observer
export { see } from './observer.js';

// Semantic Layer - Executor
export { doAction as do } from './executor.js';

// Semantic Layer - Finder
export { find } from './finder.js';

// Vision Layer
export {
    screenshot,
    buildPrompt,
    parseResponse,
    captureAXTree,
    captureState,
    processWithVPrep,
    getVPrepPresets,
    getVPrepStats,
} from './vision.js';

// Action Engine - JSON action execution
import { actionEngine } from './actionEngine.js';
const executeAction = actionEngine.execute.bind(actionEngine);
export { actionEngine, executeAction };

// LLM Client
export { llmClient, LLMClient } from './llmClient.js';

// Agent Runner
import { agentRunner } from './runner.js';
const runAgent = agentRunner.run.bind(agentRunner);
const stopAgent = agentRunner.stop.bind(agentRunner);
export { agentRunner, runAgent, stopAgent };

// Token utilities
export {
    estimateTokens,
    estimateMessageTokens,
    estimateConversationTokens,
} from './tokenCounter.js';

/**
 * @typedef {Object} AgentConfig
 * @property {string} [goal] - The goal to accomplish
 * @property {number} [maxSteps=20] - Maximum steps to run
 * @property {number} [stepDelay=2000] - Delay between steps in ms
 * @property {string} [sessionId] - Session ID for logging
 */

/**
 * @typedef {Object} AgentResult
 * @property {boolean} success - Whether the agent completed successfully
 * @property {boolean} done - Whether the goal was achieved
 * @property {number} steps - Number of steps taken
 * @property {string} [reason] - Reason for stopping (max_steps, stopped)
 */
