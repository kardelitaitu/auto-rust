/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Environment variable loader and configuration resolver.
 * @module utils/envLoader
 */

/**
 * Gets an environment variable with an optional default value.
 * @param {string} key - The environment variable name.
 * @param {any} [defaultValue] - The default value to use if the environment variable is not set.
 * @returns {string | undefined} The value of the environment variable, or the default value if provided.
 */
export function getEnv(key, defaultValue = undefined) {
    const value = process.env[key];

    if (value === undefined || value === '') {
        if (defaultValue !== undefined) {
            return defaultValue;
        }
        console.warn(`Environment variable ${key} not set and no default provided`);
        return undefined;
    }

    return value;
}

/**
 * Gets a required environment variable. Throws an error if the variable is not set.
 * @param {string} key - The environment variable name.
 * @returns {string} The value of the environment variable.
 * @throws {Error} If the environment variable is not set.
 */
export function getRequiredEnv(key) {
    const value = getEnv(key);

    if (value === undefined || value === '') {
        throw new Error(`Required environment variable ${key} is not set`);
    }

    return value;
}

/**
 * Resolves environment variable placeholders in a string (e.g., `${VAR_NAME}`).
 * @param {string} str - The string with potential placeholders.
 * @returns {string} The resolved string.
 */
export function resolveEnvVars(str) {
    if (typeof str !== 'string') return str;

    return str.replace(/\$\{([^}]+)\}/g, (match, varName) => {
        const value = process.env[varName];
        if (value === undefined) {
            console.warn(`Environment variable ${varName} referenced but not set`);
            return match; // Return original placeholder if not found
        }
        return value;
    });
}

/**
 * Recursively resolves environment variables in an object.
 * @param {object} obj - The object with potential environment variable placeholders.
 * @returns {object} The object with resolved values.
 */
export function resolveEnvVarsInObject(obj) {
    if (typeof obj === 'string') {
        return resolveEnvVars(obj);
    }

    if (typeof obj !== 'object' || obj === null) {
        return obj;
    }

    if (Array.isArray(obj)) {
        return obj.map((item) => resolveEnvVarsInObject(item));
    }

    const resolved = {};
    for (const [key, value] of Object.entries(obj)) {
        if (typeof value === 'string') {
            resolved[key] = resolveEnvVars(value);
        } else if (typeof value === 'object') {
            resolved[key] = resolveEnvVarsInObject(value);
        } else {
            resolved[key] = value;
        }
    }

    return resolved;
}

/**
 * Gets the current Node.js environment (e.g., 'development', 'production').
 * @returns {string} The current Node.js environment.
 */
export function getNodeEnv() {
    return getEnv('NODE_ENV', 'development');
}

/**
 * Checks if the current environment is 'production'.
 * @returns {boolean} True if the environment is 'production', false otherwise.
 */
export function isProduction() {
    return getNodeEnv() === 'production';
}

/**
 * Checks if the current environment is 'development'.
 * @returns {boolean} True if the environment is 'development', false otherwise.
 */
export function isDevelopment() {
    return getNodeEnv() === 'development';
}

/**
 * Gets the log level from the environment.
 * @returns {string} The log level.
 */
export function getLogLevel() {
    return getEnv('LOG_LEVEL', 'info');
}

/**
 * Validates that all required environment variables are set.
 * @param {string[]} requiredVars - An array of required environment variable names.
 * @throws {Error} If any of the required environment variables are missing.
 */
export function validateRequiredEnvVars(requiredVars) {
    const missing = [];

    for (const varName of requiredVars) {
        if (!process.env[varName] || process.env[varName] === '') {
            missing.push(varName);
        }
    }

    if (missing.length > 0) {
        throw new Error(
            `Missing required environment variables: ${missing.join(', ')}\n` +
                'Please check your .env file or environment configuration.'
        );
    }

    console.info('All required environment variables are set');
}

export default {
    getEnv,
    getRequiredEnv,
    resolveEnvVars,
    resolveEnvVarsInObject,
    getNodeEnv,
    isProduction,
    isDevelopment,
    getLogLevel,
    validateRequiredEnvVars,
};
