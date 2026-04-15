/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview A collection of validation functions for various data structures.
 * @module utils/validator
 */

import { z } from 'zod';
import { createLogger } from '../core/logger.js';

const logger = createLogger('validator.js');

/**
 * Converts a legacy schema object to a Zod schema.
 * @param {object} schema - The legacy schema object.
 * @returns {z.ZodObject} The Zod schema.
 */
function convertSchemaToZod(schema) {
    const shape = {};

    for (const [field, rules] of Object.entries(schema)) {
        let validator;

        switch (rules.type) {
            case 'string':
                validator = z.string({
                    invalid_type_error: `Field '${field}' must be of type string`,
                    required_error: `Required field '${field}' is missing`,
                });
                if (rules.minLength !== undefined) {
                    validator = validator.min(
                        rules.minLength,
                        `Field '${field}' must be at least ${rules.minLength} characters long`
                    );
                }
                if (rules.maxLength !== undefined) {
                    validator = validator.max(
                        rules.maxLength,
                        `Field '${field}' must be at most ${rules.maxLength} characters long`
                    );
                }
                if (rules.pattern) {
                    validator = validator.regex(
                        new RegExp(rules.pattern),
                        `Field '${field}' does not match required pattern`
                    );
                }
                break;

            case 'number':
                validator = z.number({
                    invalid_type_error: `Field '${field}' must be of type number`,
                    required_error: `Required field '${field}' is missing`,
                });
                if (rules.min !== undefined) {
                    validator = validator.min(
                        rules.min,
                        `Field '${field}' must be at least ${rules.min}`
                    );
                }
                if (rules.max !== undefined) {
                    validator = validator.max(
                        rules.max,
                        `Field '${field}' must be at most ${rules.max}`
                    );
                }
                break;

            case 'array': {
                const itemSchema = rules.itemSchema
                    ? convertSchemaToZod(rules.itemSchema)
                    : z.any();
                validator = z.array(itemSchema, {
                    invalid_type_error: `Field '${field}' must be of type array`,
                    required_error: `Required field '${field}' is missing`,
                });
                if (rules.nonEmpty) {
                    validator = validator.nonempty(`Field '${field}' must be a non-empty array`);
                }
                break;
            }

            case 'object':
                validator = z.object({}).passthrough();
                break;

            default:
                // If no type is specified, it matches anything.
                // However, if required is true, it must not be null or undefined.
                // We use z.unknown().refine(...) to enforce this for the required case.
                validator = z.unknown().refine((val) => val !== undefined && val !== null, {
                    message: `Required field '${field}' is missing`,
                });
        }

        if (!rules.required) {
            // Allow optional and nullable/undefined
            validator = validator.optional().nullable();
        }

        shape[field] = validator;
    }

    // Allow unknown keys (passthrough) to match legacy behavior which ignored extra fields
    return z.object(shape).passthrough();
}

/**
 * Validates an object against a schema using Zod.
 * @param {object} data - The object to validate.
 * @param {object} schema - The validation schema (legacy format).
 * @returns {{isValid: boolean, errors: string[]}} An object indicating whether the data is valid, and an array of any validation errors.
 * @private
 */
function validate(data, schema) {
    if (!data || typeof data !== 'object') {
        return { isValid: false, errors: ['Data must be a valid object'] };
    }

    // Note: convertSchemaToZod might throw if the schema is invalid (e.g. bad regex).
    // We let it throw to maintain legacy behavior where schema errors are fatal/exceptions.
    const zodSchema = convertSchemaToZod(schema);

    try {
        zodSchema.parse(data);
        return { isValid: true, errors: [] };
    } catch (error) {
        if (error instanceof z.ZodError) {
            // Compatibility: use error.issues if error.errors is undefined
            const issues = error.errors || error.issues || [];
            const errors = issues.map((err) => {
                // Enhance error message with path for nested errors if possible
                if (err.path && err.path.length > 1) {
                    return `${err.message} (at ${err.path.join('.')})`;
                }
                return err.message;
            });
            return { isValid: false, errors };
        }
        throw error;
    }
}

/**
 * Validates the structure and types of a task payload.
 * @param {object} payload - The task payload to validate.
 * @param {object} [schema={}] - Additional validation schema to merge with the default schema.
 * @returns {{isValid: boolean, errors: string[]}} An object indicating whether the payload is valid, and an array of any validation errors.
 */
export function validatePayload(payload, schema = {}) {
    const defaultSchema = {
        browserInfo: { type: 'string', required: false },
        url: { type: 'string', required: false, pattern: '^(http|https)://' },
        duration: { type: 'number', required: false, min: 1, max: 3600 },
    };

    const validationSchema = { ...defaultSchema, ...schema };
    const result = validate(payload, validationSchema);

    if (!result.isValid) {
        logger.warn('Payload validation failed:', result.errors);
    }

    return result;
}

const apiResponseSchemas = {
    roxybrowser: {
        code: { type: 'number', required: true },
        msg: { type: 'string', required: false },
        data: {
            type: 'array',
            required: true,
            itemSchema: {
                ws: { type: 'string', required: false },
                http: { type: 'string', required: false },
                windowName: { type: 'string', required: false },
                sortNum: { type: 'number', required: false },
            },
        },
    },
    ixbrowser: {
        code: { type: 'number', required: true },
    },
    morelogin: {
        code: { type: 'number', required: true },
    },
    localChrome: {
        code: { type: 'number', required: true },
    },
};

/**
 * Validates the structure of an API response.
 * @param {object} response - The API response to validate.
 * @param {string} [apiType='unknown'] - The type of API (e.g., 'roxybrowser').
 * @returns {{isValid: boolean, errors: string[]}} An object indicating whether the response is valid, and an array of any validation errors.
 */
export function validateApiResponse(response, apiType = 'unknown') {
    const schema = apiResponseSchemas[apiType];

    if (!schema) {
        logger.warn(`No specific validation rules for API type: ${apiType}`);
        return { isValid: true, errors: [] };
    }

    const result = validate(response, schema);

    if (!result.isValid) {
        logger.error(`API response validation failed for ${apiType}:`, result.errors);
    } else {
        logger.debug(`API response validation passed for ${apiType}`);
    }

    return result;
}

/**
 * Validates browser connection parameters.
 * @param {string} wsEndpoint - The WebSocket endpoint URL.
 * @returns {{isValid: boolean, errors: string[]}} An object indicating whether the connection parameters are valid, and an array of any validation errors.
 */
export function validateBrowserConnection(wsEndpoint) {
    const schema = {
        wsEndpoint: { type: 'string', required: true, pattern: '^wss?://.+' },
    };
    const result = validate({ wsEndpoint }, schema);

    if (!result.isValid) {
        logger.warn('Browser connection validation failed:', result.errors);
    }

    return result;
}

/**
 * Validates the parameters for a task execution.
 * @param {object} browser - The Playwright browser instance.
 * @param {object} payload - The task payload.
 * @param {object} [schema={}] - Additional validation schema for the payload.
 * @returns {{isValid: boolean, errors: string[]}} An object indicating whether the parameters are valid, and an array of any validation errors.
 */
export function validateTaskExecution(instance, payload, schema = {}) {
    const errors = [];

    if (!instance || typeof instance !== 'object') {
        errors.push('Browser, Context, or Page instance is required');
    } else {
        const isBrowser = typeof instance.newContext === 'function';
        const isContext = typeof instance.newPage === 'function';
        const isPage = typeof instance.goto === 'function'; // Pages have a goto method

        if (!isBrowser && !isContext && !isPage) {
            errors.push('Invalid browser, context, or page instance provided');
        }
    }

    const payloadValidation = validatePayload(payload, schema);
    if (!payloadValidation.isValid) {
        errors.push(...payloadValidation.errors);
    }

    const isValid = errors.length === 0;

    if (!isValid) {
        logger.error('Task execution validation failed:', errors);
    }

    return { isValid, errors };
}

export default {
    validatePayload,
    validateApiResponse,
    validateBrowserConnection,
    validateTaskExecution,
};
