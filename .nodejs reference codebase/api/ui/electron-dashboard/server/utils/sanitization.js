/**
 * Data sanitization utilities for logging and security
 */

/**
 * Sanitize a string for safe logging.
 * Removes control characters and escapes HTML entities.
 * @param {string} str - String to sanitize
 * @param {number} maxLength - Maximum length (default: 1000)
 * @returns {string} - Sanitized string
 */
export function sanitizeLogString(str, maxLength = 1000) {
  if (typeof str !== "string") return str;
  return str
    .replace(/[\x00-\x1F\x7F]/g, "")
    .replace(/&/g, "&")
    .replace(/</g, "<")
    .replace(/>/g, ">")
    .replace(/"/g, '"')
    .replace(/'/g, "&#x27;")
    .replace(/\//g, "&#x2F;")
    .slice(0, maxLength);
}

/**
 * Sanitize a value for safe logging.
 * Handles all types including primitives, dates, regex, maps, sets.
 * @param {any} value - Value to sanitize
 * @param {WeakSet} visited - Set of visited objects for circular reference detection
 * @returns {any} - Sanitized value
 */
function sanitizeValue(value, visited) {
  if (value === null || value === undefined) {
    return value;
  }

  const type = typeof value;

  // Primitives pass through
  if (type === "boolean" || type === "number" || type === "bigint") {
    return value;
  }

  // Strings are sanitized
  if (type === "string") {
    return sanitizeLogString(value);
  }

  // Symbols are converted to string and sanitized
  if (type === "symbol") {
    return sanitizeLogString(value.toString());
  }

  // Functions are identified
  if (type === "function") {
    return `[Function: ${value.name || "anonymous"}]`;
  }

  // Objects require circular reference check
  if (type === "object") {
    // Check for circular reference
    if (visited.has(value)) {
      return "[Circular]";
    }
    visited.add(value);

    // Handle Date
    if (value instanceof Date) {
      return value.toISOString();
    }

    // Handle RegExp
    if (value instanceof RegExp) {
      return value.toString();
    }

    // Handle Map
    if (value instanceof Map) {
      const entries = [];
      for (const [k, v] of value) {
        entries.push([sanitizeValue(k, visited), sanitizeValue(v, visited)]);
      }
      return Object.fromEntries(entries);
    }

    // Handle Set
    if (value instanceof Set) {
      return Array.from(value).map((item) => sanitizeValue(item, visited));
    }

    // Handle Error
    if (value instanceof Error) {
      return {
        name: value.name,
        message: value.message,
        stack: value.stack,
      };
    }

    // Handle arrays - preserve array structure
    if (Array.isArray(value)) {
      return value.map((item) => sanitizeValue(item, visited));
    }

    // Handle plain objects
    const sanitized = {};
    for (const [key, val] of Object.entries(value)) {
      sanitized[key] = sanitizeValue(val, visited);
    }
    return sanitized;
  }

  return value;
}

/**
 * Sanitize an object recursively for safe logging.
 * Detects circular references and handles all JavaScript types.
 * @param {any} obj - Object to sanitize
 * @returns {any} - Sanitized object
 */
export function sanitizeObject(obj) {
  if (obj === null || obj === undefined) {
    return obj;
  }

  const type = typeof obj;

  // Primitives pass through directly
  if (
    type === "boolean" ||
    type === "number" ||
    type === "bigint" ||
    type === "string"
  ) {
    return type === "string" ? sanitizeLogString(obj) : obj;
  }

  // Objects and complex types use visited set for circular detection
  const visited = new WeakSet();
  return sanitizeValue(obj, visited);
}
