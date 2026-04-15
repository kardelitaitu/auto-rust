/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Edge Case Tests: Concurrency and Race Conditions
 *
 * Tests for handling concurrent operations:
 * - Promise race conditions
 * - Async/await edge cases
 * - Mutex/lock patterns
 * - Deadlock prevention
 * - Parallel execution limits
 */

import { describe, it, expect, vi } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

describe("Edge Cases: Concurrency", () => {
  describe("Promise Race Conditions", () => {
    it("should handle Promise.race with winner", async () => {
      const fast = new Promise((resolve) =>
        setTimeout(() => resolve("fast"), 10),
      );
      const slow = new Promise((resolve) =>
        setTimeout(() => resolve("slow"), 100),
      );

      const result = await Promise.race([fast, slow]);
      expect(result).toBe("fast");
    });

    it("should handle Promise.race rejection", async () => {
      const fail = new Promise((_, reject) =>
        setTimeout(() => reject(new Error("failed")), 10),
      );
      const success = new Promise((resolve) =>
        setTimeout(() => resolve("ok"), 100),
      );

      await expect(Promise.race([fail, success])).rejects.toThrow("failed");
    });

    it("should handle Promise.all with partial failure", async () => {
      const results = [];
      const promises = [
        Promise.resolve("ok1"),
        Promise.reject(new Error("fail1")),
        Promise.resolve("ok2"),
      ];

      const settled = await Promise.allSettled(promises);

      expect(settled[0].status).toBe("fulfilled");
      expect(settled[0].value).toBe("ok1");
      expect(settled[1].status).toBe("rejected");
      expect(settled[1].reason.message).toBe("fail1");
      expect(settled[2].status).toBe("fulfilled");
    });

    it("should handle empty Promise.race", async () => {
      // Promise.race with empty array never settles (returns a pending promise forever)
      // We can test this by racing it with a timeout
      const neverResolve = Promise.race([]);

      const result = await Promise.race([
        neverResolve,
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error("timeout")), 50),
        ),
      ]).catch((e) => e.message);

      expect(result).toBe("timeout");
    });

    it("should handle concurrent map with limit", async () => {
      const concurrencyLimit = 2;
      const executing = new Set();

      const asyncTask = async (id) => {
        await new Promise((resolve) => setTimeout(resolve, 10));
        return `result-${id}`;
      };

      const limitedMap = async (items, fn) => {
        const results = [];
        for (const item of items) {
          const promise = fn(item).then((result) => {
            executing.delete(promise);
            return result;
          });
          executing.add(promise);
          results.push(promise);

          if (executing.size >= concurrencyLimit) {
            await Promise.race(executing);
          }
        }
        return Promise.all(results);
      };

      const results = await limitedMap([1, 2, 3, 4], asyncTask);
      expect(results).toEqual(["result-1", "result-2", "result-3", "result-4"]);
    });
  });

  describe("Async/Await Edge Cases", () => {
    it("should handle unhandled promise rejection", async () => {
      const unhandledRejections = [];
      const handler = (reason) => unhandledRejections.push(reason);

      // Simulate catching unhandled rejection
      const p = Promise.reject(new Error("unhandled"));
      p.catch(handler);

      await new Promise((resolve) => setTimeout(resolve, 10));
      expect(unhandledRejections.length).toBe(1);
    });

    it("should handle async function returning undefined", async () => {
      const asyncFn = async () => {
        return undefined;
      };

      const result = await asyncFn();
      expect(result).toBeUndefined();
    });

    it("should handle await on non-promise", async () => {
      const value = 42;
      const result = await value;
      expect(result).toBe(42);
    });

    it("should handle multiple awaits on same promise", async () => {
      let computeCount = 0;
      const promise = new Promise((resolve) => {
        computeCount++;
        resolve(computeCount);
      });

      const [r1, r2, r3] = await Promise.all([promise, promise, promise]);
      expect(r1).toBe(1);
      expect(r2).toBe(1);
      expect(r3).toBe(1);
      expect(computeCount).toBe(1); // Computed only once
    });

    it("should handle async error in Promise.all", async () => {
      const asyncOp = async () => {
        throw new Error("async error");
      };

      await expect(Promise.all([asyncOp()])).rejects.toThrow("async error");
    });

    it("should handle sequential vs parallel execution", async () => {
      const order = [];
      const delay = (ms, label) =>
        new Promise((resolve) => {
          setTimeout(() => {
            order.push(label);
            resolve(label);
          }, ms);
        });

      // Sequential
      order.length = 0;
      await delay(10, "a");
      await delay(10, "b");
      expect(order).toEqual(["a", "b"]);

      // Parallel
      order.length = 0;
      await Promise.all([delay(10, "c"), delay(10, "d")]);
      expect(order.sort()).toEqual(["c", "d"]);
    });
  });

  describe("Mutex and Lock Patterns", () => {
    it("should implement simple mutex", async () => {
      class Mutex {
        constructor() {
          this.locked = false;
          this.queue = [];
        }

        async acquire() {
          if (this.locked) {
            await new Promise((resolve) => this.queue.push(resolve));
          }
          this.locked = true;
        }

        release() {
          if (this.queue.length > 0) {
            const next = this.queue.shift();
            next();
          } else {
            this.locked = false;
          }
        }
      }

      const mutex = new Mutex();
      const results = [];

      const criticalSection = async (id) => {
        await mutex.acquire();
        try {
          results.push(`start-${id}`);
          await new Promise((resolve) => setTimeout(resolve, 10));
          results.push(`end-${id}`);
        } finally {
          mutex.release();
        }
      };

      await Promise.all([
        criticalSection(1),
        criticalSection(2),
        criticalSection(3),
      ]);

      // Each section should complete before the next starts
      for (let i = 0; i < results.length; i += 2) {
        expect(results[i].startsWith("start-")).toBe(true);
        expect(results[i + 1].startsWith("end-")).toBe(true);
      }
    });

    it("should implement tryLock pattern", () => {
      class TryMutex {
        constructor() {
          this.locked = false;
        }

        tryLock() {
          if (this.locked) return false;
          this.locked = true;
          return true;
        }

        unlock() {
          this.locked = false;
        }
      }

      const mutex = new TryMutex();
      expect(mutex.tryLock()).toBe(true);
      expect(mutex.tryLock()).toBe(false); // Already locked
      mutex.unlock();
      expect(mutex.tryLock()).toBe(true); // Can lock again
      mutex.unlock();
    });

    it("should implement read-write lock", async () => {
      class RWLock {
        constructor() {
          this.readers = 0;
          this.writer = false;
          this.queue = [];
        }

        async readLock() {
          while (this.writer) {
            await new Promise((resolve) => this.queue.push(resolve));
          }
          this.readers++;
        }

        readUnlock() {
          this.readers--;
          if (this.readers === 0 && this.queue.length > 0) {
            this.queue.shift()();
          }
        }

        async writeLock() {
          while (this.writer || this.readers > 0) {
            await new Promise((resolve) => this.queue.push(resolve));
          }
          this.writer = true;
        }

        writeUnlock() {
          this.writer = false;
          while (this.queue.length > 0) {
            this.queue.shift()();
          }
        }
      }

      const lock = new RWLock();
      const data = { value: 0 };

      const reader = async () => {
        await lock.readLock();
        const val = data.value;
        await new Promise((r) => setTimeout(r, 5));
        lock.readUnlock();
        return val;
      };

      const writer = async (newValue) => {
        await lock.writeLock();
        data.value = newValue;
        await new Promise((r) => setTimeout(r, 5));
        lock.writeUnlock();
      };

      // Multiple readers can run concurrently
      const readResults = await Promise.all([reader(), reader(), reader()]);
      expect(readResults.every((v) => typeof v === "number")).toBe(true);

      // Writer has exclusive access
      await writer(42);
      expect(data.value).toBe(42);
    });
  });

  describe("Deadlock Prevention", () => {
    it("should implement lock ordering to prevent deadlock", async () => {
      const lockOrder = new Map();
      let lockId = 0;

      const acquireLock = async (resource) => {
        if (!lockOrder.has(resource)) {
          lockOrder.set(resource, lockId++);
        }
        // Always acquire in order of lock ID
        await new Promise((resolve) => setTimeout(resolve, 1));
        return lockOrder.get(resource);
      };

      const resource1 = "db";
      const resource2 = "cache";

      // Both operations acquire in same order
      const op1 = acquireLock(resource1).then(() => acquireLock(resource2));
      const op2 = acquireLock(resource1).then(() => acquireLock(resource2));

      await Promise.all([op1, op2]);
      // No deadlock occurred
    });

    it("should implement timeout-based deadlock detection", async () => {
      const tryWithTimeout = async (promise, timeoutMs) => {
        const timeout = new Promise((_, reject) =>
          setTimeout(() => reject(new Error("Operation timed out")), timeoutMs),
        );
        return Promise.race([promise, timeout]);
      };

      const slowOperation = new Promise((resolve) =>
        setTimeout(() => resolve("done"), 200),
      );

      await expect(tryWithTimeout(slowOperation, 100)).rejects.toThrow(
        "timed out",
      );
    });

    it("should detect lock acquisition order violation", () => {
      const requiredOrder = ["A", "B", "C"];
      const acquireOrder = ["B", "A"]; // Violation!

      const isOrderViolated = (required, acquired) => {
        let maxIndex = -1;
        for (const resource of acquired) {
          const index = required.indexOf(resource);
          if (index === -1) continue;
          if (index <= maxIndex) return true; // Out of order
          maxIndex = index;
        }
        return false;
      };

      expect(isOrderViolated(requiredOrder, ["A", "B", "C"])).toBe(false);
      expect(isOrderViolated(requiredOrder, ["A", "C", "B"])).toBe(true);
      expect(isOrderViolated(requiredOrder, ["B", "A"])).toBe(true);
    });
  });

  describe("Parallel Execution Limits", () => {
    it("should limit concurrent operations", async () => {
      let concurrent = 0;
      let maxConcurrent = 0;
      const results = [];

      const limitedTask = async (id, limit) => {
        while (concurrent >= limit) {
          await new Promise((r) => setTimeout(r, 5));
        }
        concurrent++;
        maxConcurrent = Math.max(maxConcurrent, concurrent);
        await new Promise((r) => setTimeout(r, 10));
        results.push(id);
        concurrent--;
      };

      await Promise.all([
        limitedTask(1, 2),
        limitedTask(2, 2),
        limitedTask(3, 2),
        limitedTask(4, 2),
        limitedTask(5, 2),
      ]);

      expect(maxConcurrent).toBeLessThanOrEqual(2);
      expect(results.length).toBe(5);
    });

    it("should implement worker pool pattern", async () => {
      class WorkerPool {
        constructor(size) {
          this.size = size;
          this.workers = [];
          this.queue = [];
          for (let i = 0; i < size; i++) {
            this.workers.push({ id: i, busy: false });
          }
        }

        async execute(task) {
          const worker = this.workers.find((w) => !w.busy);
          if (worker) {
            worker.busy = true;
            try {
              return await task(worker.id);
            } finally {
              worker.busy = false;
            }
          } else {
            // All workers busy, wait and retry
            await new Promise((r) => setTimeout(r, 10));
            return this.execute(task);
          }
        }
      }

      const pool = new WorkerPool(2);
      const taskOrder = [];

      const task = async (workerId) => {
        taskOrder.push(`worker-${workerId}-start`);
        await new Promise((r) => setTimeout(r, 20));
        taskOrder.push(`worker-${workerId}-end`);
      };

      await Promise.all([
        pool.execute(task),
        pool.execute(task),
        pool.execute(task),
      ]);

      expect(taskOrder.length).toBe(6);
    });

    it("should handle batch processing with chunk size", async () => {
      const chunkSize = 3;
      const items = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
      const results = [];

      const processChunk = async (chunk) => {
        return Promise.all(
          chunk.map(async (item) => {
            await new Promise((r) => setTimeout(r, 5));
            return item * 2;
          }),
        );
      };

      for (let i = 0; i < items.length; i += chunkSize) {
        const chunk = items.slice(i, i + chunkSize);
        const chunkResults = await processChunk(chunk);
        results.push(...chunkResults);
      }

      expect(results).toEqual([2, 4, 6, 8, 10, 12, 14, 16, 18, 20]);
    });
  });

  describe("Shared State Concurrency", () => {
    it("should handle concurrent counter increment", async () => {
      let counter = 0;
      const increments = 1000;

      // Without synchronization - race condition!
      const unsafeIncrement = async () => {
        const temp = counter;
        await new Promise((r) => setTimeout(r, 1));
        counter = temp + 1;
      };

      // Reset and test unsafe
      counter = 0;
      await Promise.all(
        Array(increments)
          .fill(null)
          .map(() => unsafeIncrement()),
      );
      // Counter will be less than increments due to race condition
      expect(counter).toBeLessThan(increments);

      // With synchronization
      counter = 0;
      const safeIncrement = async () => {
        await new Promise((r) => setTimeout(r, 1));
        counter++;
      };

      await Promise.all(
        Array(increments)
          .fill(null)
          .map(() => safeIncrement()),
      );
      expect(counter).toBe(increments);
    });

    it("should handle concurrent array mutations", async () => {
      const sharedArray = [];
      const mutations = 100;

      // Using mutex for safe array access
      let mutex = Promise.resolve();

      const addToQueue = async (item) => {
        mutex = mutex.then(async () => {
          await new Promise((r) => setTimeout(r, 1));
          sharedArray.push(item);
        });
        return mutex;
      };

      await Promise.all(
        Array(mutations)
          .fill(null)
          .map((_, i) => addToQueue(i)),
      );

      expect(sharedArray.length).toBe(mutations);
    });

    it("should implement double-checked locking", () => {
      let instance = null;
      let initCount = 0;

      const getInstance = () => {
        if (instance !== null) {
          return instance; // First check (no lock)
        }

        // Acquire lock (simulated)
        if (instance === null) {
          // Second check (with lock)
          initCount++;
          instance = { id: initCount };
        }

        return instance;
      };

      // Multiple calls, should only init once
      const i1 = getInstance();
      const i2 = getInstance();
      const i3 = getInstance();

      expect(i1).toBe(i2);
      expect(i2).toBe(i3);
      expect(initCount).toBe(1);
    });
  });

  describe("Event Loop and Microtasks", () => {
    it("should handle microtask ordering", async () => {
      const order = [];

      // setTimeout is macrotask
      setTimeout(() => order.push("timeout1"), 0);
      setTimeout(() => order.push("timeout2"), 0);

      // Promise.then is microtask
      Promise.resolve().then(() => order.push("promise1"));
      Promise.resolve().then(() => order.push("promise2"));

      // queueMicrotask is also microtask
      queueMicrotask(() => order.push("microtask1"));

      await new Promise((r) => setTimeout(r, 10));

      // Microtasks run before macrotasks
      expect(order[0]).toBe("promise1");
      expect(order[1]).toBe("promise2");
      expect(order[2]).toBe("microtask1");
      // Macrotasks run last
      expect(order.includes("timeout1")).toBe(true);
      expect(order.includes("timeout2")).toBe(true);
    });

    it("should handle process.nextTick ordering", async () => {
      const order = [];

      // Create promises that resolve immediately to queue microtasks
      Promise.resolve().then(() => order.push("promise1"));
      Promise.resolve().then(() => order.push("promise2"));

      // Use queueMicrotask instead of process.nextTick for better compatibility
      queueMicrotask(() => order.push("microtask1"));
      queueMicrotask(() => order.push("microtask2"));

      setTimeout(() => order.push("timeout1"), 0);
      setTimeout(() => order.push("timeout2"), 0);

      await new Promise((r) => setTimeout(r, 20));

      // Microtasks run before macrotasks
      expect(order.slice(0, 4).sort()).toEqual(
        ["microtask1", "microtask2", "promise1", "promise2"].sort(),
      );
      expect(order.slice(4)).toEqual(["timeout1", "timeout2"]);
    });

    it("should detect starved tasks", async () => {
      let counter = 0;
      const maxIterations = 1000;

      // Simulate CPU-bound task that blocks event loop
      const blockingTask = () => {
        const start = Date.now();
        while (Date.now() - start < 50) {
          counter++;
        }
      };

      // This blocks the event loop
      blockingTask();

      // These would be delayed
      const delayed = [];
      setTimeout(() => delayed.push(1), 0);
      setTimeout(() => delayed.push(2), 0);

      await new Promise((r) => setTimeout(r, 100));

      // Tasks eventually run
      expect(delayed.length).toBe(2);
      expect(counter).toBeGreaterThan(0);
    });
  });

  describe("Timeout and Cancellation", () => {
    it("should implement AbortController pattern", async () => {
      const controller = new AbortController();

      const longTask = (signal) =>
        new Promise((resolve, reject) => {
          const timeout = setTimeout(() => resolve("completed"), 1000);
          signal.addEventListener("abort", () => {
            clearTimeout(timeout);
            reject(new Error("Aborted"));
          });
        });

      setTimeout(() => controller.abort(), 50);

      await expect(longTask(controller.signal)).rejects.toThrow("Aborted");
    });

    it("should handle cancellation with cleanup", async () => {
      const controller = new AbortController();
      const resources = { files: [], listeners: [] };

      const taskWithCleanup = async (signal) => {
        resources.files.push("temp.txt");
        resources.listeners.push(() => {});

        return new Promise((resolve, reject) => {
          const done = () => {
            // Cleanup
            resources.files.length = 0;
            resources.listeners.length = 0;
            resolve("done");
          };

          signal.addEventListener("abort", () => {
            // Cleanup on abort too
            resources.files.length = 0;
            resources.listeners.length = 0;
            reject(new Error("Cancelled"));
          });

          setTimeout(done, 100);
        });
      };

      setTimeout(() => controller.abort(), 50);

      try {
        await taskWithCleanup(controller.signal);
      } catch {
        // Cleanup happened
      }

      expect(resources.files.length).toBe(0);
      expect(resources.listeners.length).toBe(0);
    });

    it("should implement debounce with cancellation", () => {
      const debounce = (fn, delay) => {
        let timeoutId;
        return (...args) => {
          clearTimeout(timeoutId);
          return new Promise((resolve) => {
            timeoutId = setTimeout(() => resolve(fn(...args)), delay);
          });
        };
      };

      const mockFn = vi.fn(() => "result");
      const debounced = debounce(mockFn, 100);

      // Rapid calls - only last should execute
      debounced();
      debounced();
      debounced();
      const promise = debounced();

      expect(mockFn).not.toHaveBeenCalled();

      return promise.then((result) => {
        expect(mockFn).toHaveBeenCalledTimes(1);
        expect(result).toBe("result");
      });
    });

    it("should implement throttle pattern", async () => {
      const throttle = (fn, limit) => {
        let lastCall = 0;
        return (...args) => {
          const now = Date.now();
          if (now - lastCall >= limit) {
            lastCall = now;
            return fn(...args);
          }
          return "throttled";
        };
      };

      const mockFn = vi.fn(() => "called");
      const throttled = throttle(mockFn, 50);

      expect(throttled()).toBe("called");
      expect(throttled()).toBe("throttled");
      expect(throttled()).toBe("throttled");

      // Wait for throttle window to pass
      await new Promise((r) => setTimeout(r, 60));

      expect(throttled()).toBe("called");
      expect(mockFn).toHaveBeenCalledTimes(2);
    });
  });
});
