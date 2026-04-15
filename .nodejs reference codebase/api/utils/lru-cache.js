/**
 * LRU Cache Module
 *
 * Least Recently Used cache for query results and expensive operations.
 * Provides automatic eviction when max size is reached.
 *
 * @module api/utils/lru-cache
 */

/**
 * LRU Cache Node
 * @private
 */
class CacheNode {
  constructor(key, value) {
    this.key = key;
    this.value = value;
    this.prev = null;
    this.next = null;
    this.createdAt = Date.now();
    this.lastAccessed = Date.now();
  }
}

/**
 * LRU Cache Implementation
 *
 * @example
 * import { createCache } from './utils/lru-cache.js';
 *
 * const cache = createCache({
 *     maxSize: 100,
 *     ttl: 60000  // 1 minute
 * });
 *
 * cache.set('query-1', result);
 * const value = cache.get('query-1');
 */
export class LRUCache {
  /**
   * Create LRU Cache
   * @param {object} options - Cache options
   * @param {number} options.maxSize - Maximum number of items
   * @param {number} options.ttl - Time to live in ms (0 = no expiry)
   */
  constructor(options = {}) {
    this.maxSize = options.maxSize || 100;
    this.ttl = options.ttl || 0;

    /** @private */
    this.cache = new Map();
    this.head = null;
    this.tail = null;
    this.size = 0;
    this.hits = 0;
    this.misses = 0;
  }

  /**
   * Get value by key
   * @param {string} key - Cache key
   * @returns {any|null} Value or null if not found/expired
   */
  get(key) {
    const node = this.cache.get(key);

    if (!node) {
      this.misses++;
      return null;
    }

    // Check TTL
    if (this.ttl > 0 && Date.now() - node.createdAt > this.ttl) {
      this.delete(key);
      this.misses++;
      return null;
    }

    // Move to head (most recently used)
    this.moveToHead(node);
    node.lastAccessed = Date.now();
    this.hits++;

    return node.value;
  }

  /**
   * Set value in cache
   * @param {string} key - Cache key
   * @param {any} value - Value to cache
   */
  set(key, value) {
    let node = this.cache.get(key);

    if (node) {
      // Update existing
      node.value = value;
      node.lastAccessed = Date.now();
      this.moveToHead(node);
    } else {
      // Create new
      node = new CacheNode(key, value);
      this.cache.set(key, node);
      this.addToHead(node);
      this.size++;

      // Evict if over max size
      while (this.size > this.maxSize) {
        this.evictOldest();
      }
    }
  }

  /**
   * Delete key from cache
   * @param {string} key - Cache key
   * @returns {boolean} True if deleted
   */
  delete(key) {
    const node = this.cache.get(key);
    if (!node) return false;

    this.removeNode(node);
    this.cache.delete(key);
    this.size--;
    return true;
  }

  /**
   * Check if key exists (without updating access time)
   * @param {string} key - Cache key
   * @returns {boolean}
   */
  has(key) {
    const node = this.cache.get(key);
    if (!node) return false;

    // Check TTL
    if (this.ttl > 0 && Date.now() - node.createdAt > this.ttl) {
      this.delete(key);
      return false;
    }

    return true;
  }

  /**
   * Clear all entries
   */
  clear() {
    this.cache.clear();
    this.head = null;
    this.tail = null;
    this.size = 0;
    this.hits = 0;
    this.misses = 0;
  }

  /**
   * Get cache statistics
   * @returns {object}
   */
  stats() {
    const total = this.hits + this.misses;
    return {
      size: this.size,
      maxSize: this.maxSize,
      hits: this.hits,
      misses: this.misses,
      hitRate: total > 0 ? ((this.hits / total) * 100).toFixed(2) + "%" : "0%",
      ttl: this.ttl,
      keys: Array.from(this.cache.keys()),
    };
  }

  /**
   * Get all values (for debugging)
   * @returns {Array}
   */
  values() {
    return Array.from(this.cache.values()).map((n) => ({
      key: n.key,
      value: n.value,
      age: Date.now() - n.createdAt,
      lastAccessed: Date.now() - n.lastAccessed,
    }));
  }

  /** @private */
  addToHead(node) {
    if (!this.head) {
      this.head = node;
      this.tail = node;
    } else {
      node.next = this.head;
      this.head.prev = node;
      this.head = node;
    }
  }

  /** @private */
  removeNode(node) {
    if (node.prev) {
      node.prev.next = node.next;
    } else {
      this.head = node.next;
    }

    if (node.next) {
      node.next.prev = node.prev;
    } else {
      this.tail = node.prev;
    }

    node.prev = null;
    node.next = null;
  }

  /** @private */
  moveToHead(node) {
    this.removeNode(node);
    this.addToHead(node);
  }

  /** @private */
  evictOldest() {
    if (!this.tail) return;

    const key = this.tail.key;
    this.removeNode(this.tail);
    this.cache.delete(key);
    this.size--;
  }
}

/**
 * Create LRU Cache with options
 *
 * @param {object} options - Cache options
 * @param {number} options.maxSize - Maximum items (default: 100)
 * @param {number} options.ttl - Time to live in ms (default: 0 = no expiry)
 * @returns {LRUCache}
 *
 * @example
 * const cache = createCache({ maxSize: 50, ttl: 30000 });
 */
export function createCache(options = {}) {
  return new LRUCache(options);
}

/**
 * Create cached version of async function
 *
 * @param {Function} fn - Async function to cache
 * @param {object} options - Cache options
 * @param {Function} options.keyFn - Function to generate cache key from args
 * @returns {Function} Cached function
 *
 * @example
 * const cachedQuery = createCachedFunction(
 *     async (selector) => await page.$(selector),
 *     { maxSize: 100, ttl: 5000 }
 * );
 *
 * const el1 = await cachedQuery('.button');  // Executes
 * const el2 = await cachedQuery('.button');  // From cache
 */
export function createCachedFunction(fn, options = {}) {
  const cache = createCache(options);
  const keyFn = options.keyFn || ((...args) => JSON.stringify(args));

  return async (...args) => {
    const key = keyFn(...args);

    // Try cache first
    const cached = cache.get(key);
    if (cached !== null) {
      return cached;
    }

    // Execute and cache
    const result = await fn(...args);
    cache.set(key, result);
    return result;
  };
}

/**
 * Query result cache (singleton)
 * For caching DOM query results
 */
export const queryCache = createCache({
  maxSize: 200,
  ttl: 10000, // 10 seconds
});

/**
 * Selector validation cache
 * For caching selector parse results
 */
export const selectorCache = createCache({
  maxSize: 500,
  ttl: 0, // No expiry
});

/**
 * Content cache
 * For caching extracted content
 */
export const contentCache = createCache({
  maxSize: 100,
  ttl: 30000, // 30 seconds
});

export default {
  LRUCache,
  createCache,
  createCachedFunction,
  queryCache,
  selectorCache,
  contentCache,
};
