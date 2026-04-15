import { describe, it, expect, vi } from "vitest";

/**
 * Edge Case Tests: Memory Leaks and Resource Cleanup
 *
 * Tests for patterns that can cause memory leaks, resource exhaustion,
 * and proper cleanup in browser automation scenarios.
 */
describe("Edge Cases: Memory Leaks and Resource Cleanup", () => {
  describe("Event Listener Cleanup", () => {
    it("should track and cleanup event listeners properly", () => {
      const createEventManager = () => {
        const listeners = new Map();
        let totalRegistered = 0;

        return {
          on(event, callback) {
            if (!listeners.has(event)) {
              listeners.set(event, new Set());
            }
            listeners.get(event).add(callback);
            totalRegistered++;
            return () => this.off(event, callback);
          },

          off(event, callback) {
            const eventListeners = listeners.get(event);
            if (eventListeners) {
              eventListeners.delete(callback);
              if (eventListeners.size === 0) {
                listeners.delete(event);
              }
            }
          },

          removeAll(event) {
            if (event) {
              listeners.delete(event);
            } else {
              listeners.clear();
            }
          },

          get listenerCount() {
            let count = 0;
            for (const eventListeners of listeners.values()) {
              count += eventListeners.size;
            }
            return count;
          },

          get registeredCount() {
            return totalRegistered;
          },
        };
      };

      const emitter = createEventManager();
      const cb1 = vi.fn();
      const cb2 = vi.fn();
      const cb3 = vi.fn();

      const unsub1 = emitter.on("click", cb1);
      emitter.on("click", cb2);
      emitter.on("hover", cb3);

      expect(emitter.listenerCount).toBe(3);
      expect(emitter.registeredCount).toBe(3);

      unsub1();
      expect(emitter.listenerCount).toBe(2);

      emitter.removeAll("click");
      expect(emitter.listenerCount).toBe(1);

      emitter.removeAll();
      expect(emitter.listenerCount).toBe(0);
    });

    it("should detect listener accumulation patterns", () => {
      const createLeakyTracker = () => {
        const listeners = [];

        return {
          add(listener) {
            listeners.push(listener);
          },

          cleanup() {
            listeners.length = 0;
          },

          get count() {
            return listeners.length;
          },

          // Simulates memory leak - adding without cleanup
          addWithoutTracking(listener) {
            listeners.push(listener);
          },
        };
      };

      const tracker = createLeakyTracker();

      // Add listeners
      for (let i = 0; i < 100; i++) {
        tracker.add(vi.fn());
      }
      expect(tracker.count).toBe(100);

      // Cleanup should reset
      tracker.cleanup();
      expect(tracker.count).toBe(0);
    });

    it("should handle circular reference cleanup", () => {
      const createCircularRefs = () => {
        const objects = [];
        const cleanup = [];

        return {
          create() {
            const obj = { id: objects.length, data: new Array(100).fill("x") };
            const ref = { parent: obj };
            obj.ref = ref;
            objects.push(obj);

            const cleanupFn = () => {
              obj.ref = null;
              ref.parent = null;
            };
            cleanup.push(cleanupFn);

            return obj;
          },

          cleanupAll() {
            cleanup.forEach((fn) => fn());
            objects.length = 0;
            cleanup.length = 0;
          },

          get count() {
            return objects.length;
          },
        };
      };

      const manager = createCircularRefs();

      for (let i = 0; i < 10; i++) {
        const obj = manager.create();
        expect(obj.ref.parent).toBe(obj);
      }
      expect(manager.count).toBe(10);

      manager.cleanupAll();
      expect(manager.count).toBe(0);
    });
  });

  describe("Timer and Interval Management", () => {
    it("should track and cleanup timers", () => {
      vi.useFakeTimers();

      const createTimerManager = () => {
        const timers = new Set();

        return {
          setTimeout(fn, delay) {
            const id = setTimeout(() => {
              timers.delete(id);
              fn();
            }, delay);
            timers.add(id);
            return id;
          },

          setInterval(fn, delay) {
            const id = setInterval(fn, delay);
            timers.add(id);
            return id;
          },

          clear(id) {
            clearTimeout(id);
            clearInterval(id);
            timers.delete(id);
          },

          clearAll() {
            for (const id of timers) {
              clearTimeout(id);
              clearInterval(id);
            }
            timers.clear();
          },

          get activeCount() {
            return timers.size;
          },
        };
      };

      const manager = createTimerManager();

      manager.setTimeout(vi.fn(), 100);
      manager.setTimeout(vi.fn(), 200);
      manager.setInterval(vi.fn(), 50);
      manager.setInterval(vi.fn(), 100);

      expect(manager.activeCount).toBe(4);

      manager.clearAll();
      expect(manager.activeCount).toBe(0);

      vi.useRealTimers();
    });

    it("should handle timer cleanup on abort", async () => {
      vi.useFakeTimers();

      const createAbortableTimer = (signal) => {
        return new Promise((resolve, reject) => {
          if (signal?.aborted) {
            reject(new Error("Aborted"));
            return;
          }

          const timerId = setTimeout(() => resolve("completed"), 1000);

          signal?.addEventListener(
            "abort",
            () => {
              clearTimeout(timerId);
              reject(new Error("Aborted"));
            },
            { once: true },
          );
        });
      };

      const controller = new AbortController();

      const promise = createAbortableTimer(controller.signal);

      // Abort before timer completes
      controller.abort();

      await expect(promise).rejects.toThrow("Aborted");

      vi.useRealTimers();
    });

    it("should detect timer accumulation", () => {
      vi.useFakeTimers();

      let timerCount = 0;
      const originalSetInterval = setInterval;

      const createAccumulationDetector = (threshold) => {
        const timers = new Map();

        return {
          track(id, label) {
            timers.set(id, { label, createdAt: Date.now() });
          },

          checkThreshold() {
            return timers.size > threshold;
          },

          remove(id) {
            timers.delete(id);
          },

          get count() {
            return timers.size;
          },

          get oldest() {
            let oldest = null;
            for (const [id, info] of timers) {
              if (!oldest || info.createdAt < oldest.createdAt) {
                oldest = { id, ...info };
              }
            }
            return oldest;
          },
        };
      };

      const detector = createAccumulationDetector(5);

      // Add timers below threshold
      for (let i = 0; i < 5; i++) {
        detector.track(i, `timer-${i}`);
      }
      expect(detector.checkThreshold()).toBe(false);

      // Add one more to exceed threshold
      detector.track(5, "timer-5");
      expect(detector.checkThreshold()).toBe(true);
      expect(detector.count).toBe(6);

      vi.useRealTimers();
    });
  });

  describe("Object Pool Management", () => {
    it("should implement object pool with proper cleanup", () => {
      const createObjectPool = (factory, maxSize = 10) => {
        const pool = [];
        let active = 0;

        return {
          acquire() {
            let obj;
            if (pool.length > 0) {
              obj = pool.pop();
            } else {
              obj = factory();
              active++;
            }
            obj._inUse = true;
            return obj;
          },

          release(obj) {
            if (obj._inUse) {
              obj._inUse = false;
              if (pool.length < maxSize) {
                pool.push(obj);
              }
            }
          },

          get poolSize() {
            return pool.length;
          },

          get activeCount() {
            return active - pool.length;
          },

          drain() {
            pool.length = 0;
          },
        };
      };

      const pool = createObjectPool(() => ({ data: [], _inUse: false }), 5);

      // Acquire objects
      const objects = [];
      for (let i = 0; i < 10; i++) {
        objects.push(pool.acquire());
      }
      expect(pool.activeCount).toBe(10);

      // Release some objects
      for (let i = 0; i < 5; i++) {
        pool.release(objects[i]);
      }
      expect(pool.poolSize).toBe(5);

      // Acquire again - should reuse from pool
      const reused = pool.acquire();
      expect(pool.poolSize).toBe(4);

      // Drain the pool
      pool.drain();
      expect(pool.poolSize).toBe(0);
    });

    it("should handle pool exhaustion gracefully", () => {
      const createLimitedPool = (maxActive) => {
        const queue = [];
        let activeCount = 0;

        return {
          tryAcquire() {
            if (activeCount >= maxActive) {
              return { success: false, reason: "pool_exhausted" };
            }
            activeCount++;
            return {
              success: true,
              release: () => {
                activeCount--;
              },
            };
          },

          async acquireWithQueue() {
            if (activeCount < maxActive) {
              activeCount++;
              return {
                release: () => {
                  activeCount--;
                },
              };
            }

            return new Promise((resolve) => {
              queue.push(resolve);
            });
          },

          get activeCount() {
            return activeCount;
          },

          get queueLength() {
            return queue.length;
          },
        };
      };

      const pool = createLimitedPool(3);

      // Fill the pool
      const acquired = [];
      for (let i = 0; i < 3; i++) {
        const result = pool.tryAcquire();
        expect(result.success).toBe(true);
        acquired.push(result);
      }

      // Pool should be exhausted
      const failed = pool.tryAcquire();
      expect(failed.success).toBe(false);
      expect(failed.reason).toBe("pool_exhausted");

      // Release one
      acquired[0].release();
      expect(pool.activeCount).toBe(2);

      // Should be able to acquire again
      const retry = pool.tryAcquire();
      expect(retry.success).toBe(true);
    });
  });

  describe("Cache Management", () => {
    it("should implement LRU cache with size limits", () => {
      const createLRUCache = (maxSize) => {
        const cache = new Map();

        return {
          get(key) {
            if (!cache.has(key)) return undefined;
            const value = cache.get(key);
            // Move to end (most recently used)
            cache.delete(key);
            cache.set(key, value);
            return value;
          },

          set(key, value) {
            if (cache.has(key)) {
              cache.delete(key);
            } else if (cache.size >= maxSize) {
              // Delete oldest (first in map)
              const oldest = cache.keys().next().value;
              cache.delete(oldest);
            }
            cache.set(key, value);
          },

          delete(key) {
            return cache.delete(key);
          },

          clear() {
            cache.clear();
          },

          get size() {
            return cache.size;
          },

          has(key) {
            return cache.has(key);
          },
        };
      };

      const cache = createLRUCache(3);

      cache.set("a", 1);
      cache.set("b", 2);
      cache.set("c", 3);
      expect(cache.size).toBe(3);

      // Access 'a' to make it recently used
      cache.get("a");

      // Adding 'd' should evict 'b' (oldest)
      cache.set("d", 4);
      expect(cache.size).toBe(3);
      expect(cache.has("b")).toBe(false);
      expect(cache.has("a")).toBe(true);
    });

    it("should implement TTL-based cache expiration", () => {
      const createTTLCache = (defaultTTL) => {
        const cache = new Map();

        return {
          set(key, value, ttl = defaultTTL) {
            cache.set(key, {
              value,
              expiresAt: Date.now() + ttl,
            });
          },

          get(key) {
            const entry = cache.get(key);
            if (!entry) return undefined;
            if (Date.now() > entry.expiresAt) {
              cache.delete(key);
              return undefined;
            }
            return entry.value;
          },

          cleanup() {
            const now = Date.now();
            for (const [key, entry] of cache) {
              if (now > entry.expiresAt) {
                cache.delete(key);
              }
            }
          },

          get size() {
            return cache.size;
          },
        };
      };

      const cache = createTTLCache(1000);

      cache.set("key1", "value1", 100);
      cache.set("key2", "value2", 5000);

      expect(cache.get("key1")).toBe("value1");
      expect(cache.get("key2")).toBe("value2");

      // Simulate time passing
      vi.useFakeTimers();
      vi.advanceTimersByTime(200);

      expect(cache.get("key1")).toBe(undefined);
      expect(cache.get("key2")).toBe("value2");

      vi.useRealTimers();
    });

    it("should handle cache stampede prevention", async () => {
      const createStampedeSafeCache = (fetchFn, ttl) => {
        const cache = new Map();
        const inProgress = new Map();

        return async function get(key) {
          // Check cache first
          const cached = cache.get(key);
          if (cached && Date.now() < cached.expiresAt) {
            return cached.value;
          }

          // Check if request is already in progress
          if (inProgress.has(key)) {
            return inProgress.get(key);
          }

          // Make new request
          const promise = fetchFn(key)
            .then((value) => {
              cache.set(key, {
                value,
                expiresAt: Date.now() + ttl,
              });
              inProgress.delete(key);
              return value;
            })
            .catch((err) => {
              inProgress.delete(key);
              throw err;
            });

          inProgress.set(key, promise);
          return promise;
        };
      };

      const fetchFn = vi.fn().mockResolvedValue("fetched-value");
      const get = createStampedeSafeCache(fetchFn, 1000);

      // Multiple concurrent calls should only fetch once
      const results = await Promise.all([
        get("key1"),
        get("key1"),
        get("key1"),
      ]);

      expect(results).toEqual([
        "fetched-value",
        "fetched-value",
        "fetched-value",
      ]);
      expect(fetchFn).toHaveBeenCalledTimes(1);
    });
  });

  describe("WeakRef and Finalization Patterns", () => {
    it("should track weak references properly", () => {
      const createWeakTracker = () => {
        const refs = new Map();
        const registry = new FinalizationRegistry((key) => {
          refs.delete(key);
        });

        return {
          track(key, obj) {
            refs.set(key, new WeakRef(obj));
            registry.register(obj, key);
          },

          get(key) {
            const ref = refs.get(key);
            if (!ref) return undefined;

            const obj = ref.deref();
            if (!obj) {
              refs.delete(key);
              return undefined;
            }
            return obj;
          },

          get trackedCount() {
            let count = 0;
            for (const ref of refs.values()) {
              if (ref.deref()) count++;
            }
            return count;
          },

          cleanup() {
            for (const [key, ref] of refs) {
              if (!ref.deref()) {
                refs.delete(key);
              }
            }
          },
        };
      };

      const tracker = createWeakTracker();

      let obj1 = { data: "test1" };
      let obj2 = { data: "test2" };

      tracker.track("key1", obj1);
      tracker.track("key2", obj2);

      expect(tracker.get("key1")).toBe(obj1);
      expect(tracker.get("key2")).toBe(obj2);

      // Clear reference
      obj1 = null; // eslint-disable-line no-useless-assignment

      // Manually trigger cleanup check
      tracker.cleanup();

      // After deref check, key1 should be gone if obj1 was collected
      // In test environment, we just verify the tracking mechanism works
      expect(tracker.get("key2")).toBe(obj2);
    });

    it("should handle weak map cache patterns", () => {
      const createWeakCache = () => {
        const cache = new WeakMap();
        const stats = { hits: 0, misses: 0 };

        return {
          get(obj) {
            const value = cache.get(obj);
            if (value !== undefined) {
              stats.hits++;
            } else {
              stats.misses++;
            }
            return value;
          },

          set(obj, value) {
            cache.set(obj, value);
          },

          has(obj) {
            return cache.has(obj);
          },

          get stats() {
            return { ...stats };
          },
        };
      };

      const cache = createWeakCache();
      const obj = { id: 1 };

      expect(cache.has(obj)).toBe(false);
      expect(cache.get(obj)).toBe(undefined);
      expect(cache.stats.misses).toBe(1);

      cache.set(obj, "cached-value");
      expect(cache.has(obj)).toBe(true);
      expect(cache.get(obj)).toBe("cached-value");
      expect(cache.stats.hits).toBe(1);
    });
  });

  describe("Resource Limit Tracking", () => {
    it("should track memory usage patterns", () => {
      const createMemoryTracker = (limitMB) => {
        const allocations = new Map();
        let totalBytes = 0;

        return {
          allocate(key, sizeBytes) {
            if (totalBytes + sizeBytes > limitMB * 1024 * 1024) {
              return { success: false, reason: "limit_exceeded" };
            }
            allocations.set(key, sizeBytes);
            totalBytes += sizeBytes;
            return { success: true };
          },

          free(key) {
            const size = allocations.get(key);
            if (size) {
              allocations.delete(key);
              totalBytes -= size;
            }
          },

          get usageBytes() {
            return totalBytes;
          },

          get usagePercent() {
            return (totalBytes / (limitMB * 1024 * 1024)) * 100;
          },

          get isNearLimit() {
            return this.usagePercent > 80;
          },
        };
      };

      const tracker = createMemoryTracker(1); // 1MB limit

      // Allocate some memory
      const result1 = tracker.allocate("buffer1", 1024 * 512); // 512KB
      expect(result1.success).toBe(true);
      expect(tracker.usagePercent).toBe(50);

      const result2 = tracker.allocate("buffer2", 1024 * 400); // 400KB
      expect(result2.success).toBe(true);
      expect(tracker.isNearLimit).toBe(true);

      // This should fail - exceeds limit
      const result3 = tracker.allocate("buffer3", 1024 * 200);
      expect(result3.success).toBe(false);
      expect(result3.reason).toBe("limit_exceeded");

      // Free some and retry
      tracker.free("buffer1");
      const result4 = tracker.allocate("buffer3", 1024 * 200);
      expect(result4.success).toBe(true);
    });

    it("should implement connection pool limits", async () => {
      const createConnectionPool = (maxConnections) => {
        const active = new Set();
        const waiting = [];

        return {
          async acquire() {
            if (active.size < maxConnections) {
              const id = Symbol("connection");
              active.add(id);
              return {
                id,
                release: () => {
                  active.delete(id);
                  if (waiting.length > 0) {
                    const resolve = waiting.shift();
                    resolve();
                  }
                },
              };
            }

            return new Promise((resolve) => {
              waiting.push(async () => {
                const conn = await this.acquire();
                resolve(conn);
              });
            });
          },

          get activeCount() {
            return active.size;
          },

          get waitingCount() {
            return waiting.length;
          },

          get maxConnections() {
            return maxConnections;
          },
        };
      };

      const pool = createConnectionPool(2);

      const conn1 = await pool.acquire();
      const conn2 = await pool.acquire();

      expect(pool.activeCount).toBe(2);

      // Start waiting for a connection
      const waitingPromise = pool.acquire();
      expect(pool.waitingCount).toBe(1);

      // Release a connection
      conn1.release();

      // Waiting should now acquire
      const conn3 = await waitingPromise;
      expect(pool.activeCount).toBe(2);

      conn2.release();
      conn3.release();
    });
  });

  describe("Cleanup Strategies", () => {
    it("should implement cleanup registry pattern", async () => {
      const createCleanupRegistry = () => {
        const cleanups = [];

        return {
          register(cleanupFn) {
            cleanups.push(cleanupFn);
            return () => {
              const idx = cleanups.indexOf(cleanupFn);
              if (idx !== -1) cleanups.splice(idx, 1);
            };
          },

          async cleanupAll() {
            const errors = [];
            for (const fn of cleanups.reverse()) {
              try {
                await fn();
              } catch (err) {
                errors.push(err);
              }
            }
            cleanups.length = 0;
            return errors;
          },

          get pendingCount() {
            return cleanups.length;
          },
        };
      };

      const registry = createCleanupRegistry();

      const cleanup1 = vi.fn();
      const cleanup2 = vi.fn().mockImplementation(() => {
        throw new Error("cleanup failed");
      });
      const cleanup3 = vi.fn();

      registry.register(cleanup1);
      registry.register(cleanup2);
      registry.register(cleanup3);

      expect(registry.pendingCount).toBe(3);

      const errors = await registry.cleanupAll();

      expect(cleanup1).toHaveBeenCalled();
      expect(cleanup2).toHaveBeenCalled();
      expect(cleanup3).toHaveBeenCalled();
      expect(registry.pendingCount).toBe(0);
    });

    it("should handle graceful shutdown sequence", async () => {
      const createGracefulShutdown = () => {
        const handlers = [];
        let isShuttingDown = false;

        return {
          register(handler) {
            handlers.push(handler);
          },

          async shutdown() {
            if (isShuttingDown) return;
            isShuttingDown = true;

            const results = [];
            for (const handler of handlers) {
              try {
                await handler();
                results.push({ status: "success" });
              } catch (err) {
                results.push({ status: "error", error: err.message });
              }
            }
            return results;
          },

          get isShuttingDown() {
            return isShuttingDown;
          },
        };
      };

      const shutdown = createGracefulShutdown();

      const handler1 = vi.fn().mockResolvedValue();
      const handler2 = vi.fn().mockRejectedValue(new Error("failed"));
      const handler3 = vi.fn().mockResolvedValue();

      shutdown.register(handler1);
      shutdown.register(handler2);
      shutdown.register(handler3);

      expect(shutdown.isShuttingDown).toBe(false);

      await shutdown.shutdown();

      expect(shutdown.isShuttingDown).toBe(true);
      expect(handler1).toHaveBeenCalled();
      expect(handler2).toHaveBeenCalled();
      expect(handler3).toHaveBeenCalled();
    });
  });
});
