/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Task validation utilities
 * Validates task payloads against schemas before execution
 * @module utils/task-validator
 */

/**
 * Task payload schema definition
 * @typedef {Object} TaskSchema
 * @property {Object} required - Required fields with types
 * @property {Object} optional - Optional fields with types
 * @property {Function} [validate] - Custom validation function
 */

/**
 * Supported field types
 */
const FIELD_TYPES = {
  string: (value) => typeof value === "string",
  number: (value) => typeof value === "number" && !isNaN(value),
  boolean: (value) => typeof value === "boolean",
  object: (value) => typeof value === "object" && value !== null,
  array: (value) => Array.isArray(value),
  url: (value) => {
    if (typeof value !== "string") return false;
    try {
      new URL(value);
      return true;
    } catch {
      return false;
    }
  },
  function: (value) => typeof value === "function",
};

/**
 * Task schemas registry
 */
const TASK_SCHEMAS = {
  pageview: {
    required: {
      // url can be in payload or loaded from file
    },
    optional: {
      url: "url",
      timeout: "number",
    },
    validate: (payload) => {
      const errors = [];
      // Either url in payload or will load from file
      if (payload.url && !FIELD_TYPES.url(payload.url)) {
        errors.push("url must be a valid URL (e.g., https://example.com)");
      }
      return errors;
    },
  },

  twitterFollow: {
    required: {},
    optional: {
      targetUrl: "url",
      url: "url",
      taskTimeoutMs: "number",
    },
    validate: (payload) => {
      const errors = [];
      const urlField = payload.targetUrl || payload.url;

      if (!urlField) {
        // Will use default TARGET_TWEET_URL
        return errors;
      }

      if (!FIELD_TYPES.url(urlField)) {
        errors.push("targetUrl/url must be a valid Twitter URL");
      }

      if (
        payload.taskTimeoutMs &&
        (typeof payload.taskTimeoutMs !== "number" ||
          payload.taskTimeoutMs < 1000)
      ) {
        errors.push("taskTimeoutMs must be a number >= 1000");
      }

      return errors;
    },
  },

  like: {
    required: {},
    optional: {
      tweetUrl: "url",
      url: "url",
    },
    validate: (payload) => {
      const errors = [];
      const urlField = payload.tweetUrl || payload.url;

      if (!urlField) {
        errors.push("tweetUrl or url is required");
      } else if (!FIELD_TYPES.url(urlField)) {
        errors.push("tweetUrl/url must be a valid Twitter URL");
      }

      return errors;
    },
  },

  retweet: {
    required: {},
    optional: {
      tweetUrl: "url",
      url: "url",
    },
    validate: (payload) => {
      const errors = [];
      const urlField = payload.tweetUrl || payload.url;

      if (!urlField) {
        errors.push("tweetUrl or url is required");
      } else if (!FIELD_TYPES.url(urlField)) {
        errors.push("tweetUrl/url must be a valid Twitter URL");
      }

      return errors;
    },
  },

  reply: {
    required: {},
    optional: {
      tweetUrl: "url",
      url: "url",
      message: "string",
    },
    validate: (payload) => {
      const errors = [];
      const urlField = payload.tweetUrl || payload.url;

      if (!urlField) {
        errors.push("tweetUrl or url is required");
      } else if (!FIELD_TYPES.url(urlField)) {
        errors.push("tweetUrl/url must be a valid Twitter URL");
      }

      if (payload.message && !FIELD_TYPES.string(payload.message)) {
        errors.push("message must be a string");
      }

      return errors;
    },
  },
};

/**
 * Validate a task payload against its schema
 *
 * @param {string} taskName - Name of the task
 * @param {Object} payload - Task payload to validate
 * @returns {Object} Validation result
 * @returns {boolean} result.isValid - Whether validation passed
 * @returns {string[]} result.errors - Array of error messages
 *
 * @example
 * const result = validateTaskPayload('twitterFollow', { targetUrl: 'https://x.com/user' });
 * if (!result.isValid) {
 *     console.error('Validation failed:', result.errors);
 * }
 */
export function validateTaskPayload(taskName, payload) {
  const schema = TASK_SCHEMAS[taskName];

  if (!schema) {
    // Unknown task - skip validation
    return {
      isValid: true,
      errors: [],
      warnings: [`Unknown task: ${taskName}`],
    };
  }

  const errors = [];
  const warnings = [];

  // Check required fields
  if (schema.required) {
    for (const [field, type] of Object.entries(schema.required)) {
      if (payload[field] === undefined || payload[field] === null) {
        errors.push(`Required field missing: ${field}`);
      } else if (type && !FIELD_TYPES[type](payload[field])) {
        errors.push(`Field ${field} must be of type ${type}`);
      }
    }
  }

  // Check optional fields types
  if (schema.optional && payload) {
    for (const [field, type] of Object.entries(schema.optional)) {
      if (payload[field] !== undefined && !FIELD_TYPES[type](payload[field])) {
        errors.push(`Optional field ${field} must be of type ${type}`);
      }
    }
  }

  // Run custom validation
  if (schema.validate && errors.length === 0) {
    const customErrors = schema.validate(payload);
    errors.push(...customErrors);
  }

  return {
    isValid: errors.length === 0,
    errors,
    warnings,
  };
}

/**
 * Register a new task schema
 *
 * @param {string} taskName - Name of the task
 * @param {TaskSchema} schema - Task schema definition
 *
 * @example
 * registerTaskSchema('myCustomTask', {
 *     required: { url: 'url' },
 *     optional: { timeout: 'number' }
 * });
 */
export function registerTaskSchema(taskName, schema) {
  TASK_SCHEMAS[taskName] = schema;
}

/**
 * Get schema for a task
 *
 * @param {string} taskName - Name of the task
 * @returns {TaskSchema|undefined} Task schema or undefined
 */
export function getTaskSchema(taskName) {
  return TASK_SCHEMAS[taskName];
}

/**
 * List all registered task schemas
 *
 * @returns {string[]} Array of task names
 */
export function listTaskSchemas() {
  return Object.keys(TASK_SCHEMAS);
}

export { FIELD_TYPES, TASK_SCHEMAS };

export default {
  validateTaskPayload,
  registerTaskSchema,
  getTaskSchema,
  listTaskSchemas,
  FIELD_TYPES,
  TASK_SCHEMAS,
};
