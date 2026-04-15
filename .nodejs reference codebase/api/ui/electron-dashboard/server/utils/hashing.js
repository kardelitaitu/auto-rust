/**
 * Hashing utilities for change detection
 *
 * WARNING: These functions use non-cryptographic hashes (DJB2).
 * Do NOT use for security purposes (passwords, tokens, signatures).
 * Suitable for: change detection, cache keys, deduplication.
 */

/**
 * Safely stringify an object, handling circular references.
 * @param {any} obj - Object to stringify
 * @returns {string} - JSON string or fallback representation
 */
function safeStringify(obj) {
  const seen = new WeakSet();

  return JSON.stringify(obj, (key, value) => {
    if (typeof value === "object" && value !== null) {
      if (seen.has(value)) {
        return "[Circular]";
      }
      seen.add(value);
    }
    return value;
  });
}

/**
 * Generate a quick hash for change detection.
 * Uses a simple DJB2-like hash for performance (not cryptographic).
 *
 * Note: DJB2 has known collision patterns but is suitable for
 * change detection where occasional collisions are acceptable.
 *
 * @param {any} obj - Object to hash
 * @returns {string} - Hash string in base-36
 */
export function quickHash(obj) {
  try {
    const str = safeStringify(obj);
    let hash = 5381;
    for (let i = 0; i < str.length; i++) {
      hash = (hash << 5) + hash + str.charCodeAt(i);
      hash = hash | 0; // Convert to 32-bit integer
    }
    return hash.toString(36);
  } catch (e) {
    // Fallback for unexpected errors
    return "hash_error";
  }
}

/**
 * Generate a hash for a string directly.
 * Useful for hashing URLs, IDs, or other string values.
 *
 * @param {string} str - String to hash
 * @returns {string} - Hash string in base-36
 */
export function quickHashString(str) {
  if (typeof str !== "string") {
    return quickHash(str);
  }

  let hash = 5381;
  for (let i = 0; i < str.length; i++) {
    hash = (hash << 5) + hash + str.charCodeAt(i);
    hash = hash | 0; // Convert to 32-bit integer
  }
  return hash.toString(36);
}
