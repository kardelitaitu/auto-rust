/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Local Ollama Manager - Dedicated utility to manage Ollama service.
 * Handles checking status and starting the Ollama server and models.
 * @module utils/local-ollama-manager
 */

import { exec, execSync } from 'child_process';
import { createLogger } from '../core/logger.js';
import { getSettings } from './configLoader.js';

const logger = createLogger('ollama-manager.js');
let hasTestWarning = false;

const CACHE_TTL_MS = 30000; // 30s cache
let _cachedIsRunning = null;
let _cacheTimestamp = 0;
const PENALTY_TTL_MS = 30000; // 30s penalty
let _penaltyTimestamp = 0;
let _ongoingCheckPromise = null;
let _ongoingEnsurePromise = null;

const EXEC_TIMEOUT_MS = 5000;

function execWithTimeout(command, options = {}) {
    return new Promise((resolve, reject) => {
        const timeout = setTimeout(() => {
            reject(new Error(`Command timed out after ${EXEC_TIMEOUT_MS}ms: ${command}`));
        }, EXEC_TIMEOUT_MS);

        exec(command, { ...options, windowsHide: true }, (error, stdout, stderr) => {
            clearTimeout(timeout);
            if (error) {
                reject(error);
            } else {
                resolve({ stdout, stderr });
            }
        });
    });
}

function execWithExitCode(command, options = {}) {
    return new Promise((resolve) => {
        let settled = false;
        const finish = (success) => {
            if (!settled) {
                settled = true;
                resolve(success);
            }
        };

        try {
            const child = exec(command, { ...options, windowsHide: true }, (error) => {
                if (error) {
                    finish(false);
                } else {
                    finish(true);
                }
            });

            if (child && typeof child.on === 'function') {
                child.on('exit', (code) => finish(code === 0));
                child.on('error', () => finish(false));
            }
        } catch {
            finish(false);
        }
    });
}

/**
 * Kill any existing ollama processes to prevent duplicates
 */
function killOllamaProcesses() {
    try {
        execSync('taskkill /F /IM ollama.exe 2>nul', { timeout: 5000, windowsHide: true });
    } catch {
        // Ignore if no process found
    }
}

/**
 * Check if the Ollama process is running in Windows.
 * @returns {boolean}
 */
function isOllamaProcessRunning() {
    try {
        const output = execSync('tasklist /FI "IMAGENAME eq ollama.exe" /NH', {
            encoding: 'utf8',
            timeout: EXEC_TIMEOUT_MS,
        });
        return output.toLowerCase().includes('ollama.exe');
    } catch {
        return false;
    }
}

function resolveOllamaCommand() {
    if (process.env.VITEST === 'true' || process.env.NODE_ENV === 'test') {
        return 'ollama';
    }

    try {
        const output = execSync('where ollama', { encoding: 'utf8', timeout: EXEC_TIMEOUT_MS });
        const firstPath = output
            .split('\n')
            .map((line) => line.trim())
            .find(Boolean);
        if (firstPath) {
            return `"${firstPath}"`;
        }
    } catch {
        null;
    }

    if (process.env.LOCALAPPDATA) {
        return `"${process.env.LOCALAPPDATA}\\Programs\\Ollama\\ollama.exe"`;
    }

    return 'ollama';
}

function isLocalBaseUrl(baseUrl) {
    try {
        const url = new URL(baseUrl);
        const host = url.hostname.toLowerCase();
        return host === 'localhost' || host === '127.0.0.1' || host === '::1';
    } catch {
        return true;
    }
}

async function getOllamaBaseUrl() {
    const settings = await getSettings();
    const endpoint = settings.llm?.local?.endpoint || 'http://localhost:11434';
    return endpoint.replace(/\/api\/.*$/, '').replace(/\/$/, '');
}

async function isOllamaEndpointReady(baseUrl) {
    try {
        const rootResponse = await fetch(`${baseUrl}/`, {
            signal: AbortSignal.timeout(1500),
        });
        if (rootResponse.ok) {
            return true;
        }
    } catch {
        null;
    }

    try {
        const tagsResponse = await fetch(`${baseUrl}/api/tags`, {
            signal: AbortSignal.timeout(1500),
        });
        return tagsResponse.ok;
    } catch {
        return false;
    }
}

async function waitForOllamaReady(baseUrl, attempts = 12, delayMs = 2000) {
    for (let i = 0; i < attempts; i++) {
        if (await isOllamaEndpointReady(baseUrl)) {
            return true;
        }
        await new Promise((resolve) => setTimeout(resolve, delayMs));
    }
    return false;
}

/**
 * Check if the Ollama service is reachable at the configured endpoint.
 * Uses caching to avoid repeated expensive checks.
 * @param {boolean} forceRefresh - Bypass cache and force fresh check
 * @returns {Promise<boolean>}
 */
export async function isOllamaRunning(forceRefresh = false) {
    const now = Date.now();

    if (!forceRefresh && now - _penaltyTimestamp < PENALTY_TTL_MS) {
        return false;
    }

    if (!forceRefresh && _cachedIsRunning !== null && now - _cacheTimestamp < CACHE_TTL_MS) {
        return _cachedIsRunning;
    }

    if (_ongoingCheckPromise && !forceRefresh) {
        return _ongoingCheckPromise;
    }

    _ongoingCheckPromise = (async () => {
        try {
            const baseUrl = await getOllamaBaseUrl();
            const isLocal = isLocalBaseUrl(baseUrl);

            // Check HTTP endpoint FIRST (fast)
            const isReady = await isOllamaEndpointReady(baseUrl);
            if (isReady) {
                _cachedIsRunning = true;
                _cacheTimestamp = Date.now();
                return true;
            }

            if (!isLocal) {
                _cachedIsRunning = false;
                _cacheTimestamp = Date.now();
                return false;
            }

            // HTTP down and it's local. Check process (slow)
            const processRunning = isOllamaProcessRunning();
            if (!processRunning) {
                _cachedIsRunning = false;
                _cacheTimestamp = Date.now();
                return false;
            }

            _cachedIsRunning = true;
            _cacheTimestamp = Date.now();
            return true;
        } catch {
            _cachedIsRunning = false;
            _cacheTimestamp = Date.now();
            return false;
        } finally {
            _ongoingCheckPromise = null;
        }
    })();

    return _ongoingCheckPromise;
}

export function clearOllamaCache() {
    _cachedIsRunning = null;
    _cacheTimestamp = 0;
    _penaltyTimestamp = 0;
    _ongoingCheckPromise = null;
    _ongoingEnsurePromise = null;
}

/**
 * Check if a specific model exists in Ollama.
 * @param {string} modelName
 * @returns {Promise<boolean>}
 */
async function doesModelExist(modelName) {
    try {
        const { stdout } = await execWithTimeout('ollama list', { encoding: 'utf8' });
        const lines = stdout.split('\n').map((l) => l.trim().split(/\s+/)[0]);
        return lines.some((l) => l === modelName || l.startsWith(`${modelName}:`));
    } catch {
        return false;
    }
}

/**
 * Start the Ollama service and ensure the model is loaded.
 * @returns {Promise<boolean>}
 */
export async function startOllama() {
    clearOllamaCache();

    try {
        const settings = await getSettings();
        const model = settings.llm?.local?.model || 'hermes3:8b';
        const skipModelOps =
            (process.env.VITEST === 'true' || process.env.NODE_ENV === 'test') &&
            process.env.ALLOW_OLLAMA_MODEL_OPS !== 'true';
        const baseUrl = await getOllamaBaseUrl();
        const ollamaCmd = resolveOllamaCommand();

        // Kill any existing processes first to prevent duplicates
        if (isOllamaProcessRunning()) {
            logger.info(`[OllamaManager] Existing process found, killing first...`);
            killOllamaProcesses();
            await new Promise((resolve) => setTimeout(resolve, 1000));
        }

        if (!isOllamaProcessRunning()) {
            logger.info(`[OllamaManager] Starting Ollama via command...`);

            // Use spawn-like approach with execSync for reliability
            try {
                execSync(`start /B "" ${ollamaCmd} serve`, { timeout: 10000, windowsHide: true });
                logger.info(`[OllamaManager] Started ollama serve`);
            } catch (serveErr) {
                logger.debug(`[OllamaManager] serve error: ${serveErr.message}`);
            }

            // Wait for process to start
            await new Promise((resolve) => setTimeout(resolve, 5000));
        }

        const apiReady = await waitForOllamaReady(baseUrl, 12, 2000);
        if (!apiReady) {
            logger.error('[OllamaManager] API failed to respond after start.');
            return false;
        }

        // Skip model ops if in test mode
        if (skipModelOps) {
            logger.info('[OllamaManager] Skipping model operations (test mode)');
            return true;
        }

        // Ensure Model exists (Pull if missing)
        if (!(await doesModelExist(model))) {
            logger.warn(`[OllamaManager] Model '${model}' not found. Pulling...`);
            const pulled = await execWithExitCode(`${ollamaCmd} pull ${model}`, {
                timeout: 300000,
            });
            if (!pulled) {
                logger.error(`[OllamaManager] Pull failed for ${model}`);
                return false;
            }
            logger.success(`[OllamaManager] Successfully pulled ${model}`);
        }

        return true;
    } catch (error) {
        logger.error(`[OllamaManager] Failed to start/prepare Ollama: ${error.message}`);
        return false;
    }
}

/**
 * Main entry point: Ensure Ollama is running and ready for use.
 * @returns {Promise<boolean>}
 */
export function ensureOllama() {
    if (_ongoingEnsurePromise) {
        return _ongoingEnsurePromise;
    }

    _ongoingEnsurePromise = (async () => {
        try {
            if (
                (process.env.VITEST === 'true' || process.env.NODE_ENV === 'test') &&
                process.env.ALLOW_OLLAMA_MODEL_OPS !== 'true'
            ) {
                if (!hasTestWarning) {
                    logger.warn('[OllamaManager] Ollama checks are disabled in test mode.');
                    hasTestWarning = true;
                }
                return false;
            }

            if (await isOllamaRunning()) {
                logger.debug('[OllamaManager] Ollama is already running.');
                return true;
            }

            logger.warn('[OllamaManager] Ollama not detected. Attempting to start...');
            await startOllama();

            // Verification loop - reduced from 10 to 4 attempts
            let attempts = 0;
            while (attempts < 4) {
                attempts++;
                if (await isOllamaRunning(true)) {
                    logger.success('[OllamaManager] Ollama is now ready.');
                    return true;
                }
                await new Promise((resolve) => setTimeout(resolve, 2000));
            }

            logger.error('[OllamaManager] Could not start Ollama service automatically.');
            _penaltyTimestamp = Date.now();
            return false;
        } finally {
            _ongoingEnsurePromise = null;
        }
    })();

    return _ongoingEnsurePromise;
}
