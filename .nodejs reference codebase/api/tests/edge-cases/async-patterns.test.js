import { describe, it, expect, vi } from "vitest";

/**
 * Edge Case Tests: Async Patterns and Promise Handling
 *
 * Tests for async/await edge cases, Promise handling, and
 * complex asynchronous patterns in browser automation.
 */
describe("Edge Cases: Async Patterns and Promise Handling", () => {
  describe("Promise Chaining and Composition", () => {
    it("should handle Promise.all with mixed results", async () => {
      const createAsyncTask = (shouldSucceed, delay, value) => {
        return new Promise((resolve, reject) => {
          setTimeout(() => {
            if (shouldSucceed) {
              resolve(value);
            } else {
              reject(new Error(`Failed: ${value}`));
            }
          }, delay);
        });
      };

      const tasks = [
        createAsyncTask(true, 10, "success1"),
        createAsyncTask(false, 20, "fail1"),
        createAsyncTask(true, 30, "success2"),
      ];

      const results = await Promise.allSettled(tasks);

      expect(results[0].status).toBe("fulfilled");
      expect(results[0].value).toBe("success1");
      expect(results[1].status).toBe("rejected");
      expect(results[1].reason.message).toBe("Failed: fail1");
      expect(results[2].status).toBe("fulfilled");
      expect(results[2].value).toBe("success2");
    });

    it("should handle Promise.race with timeout", async () => {
      const withTimeout = (promise, timeoutMs) => {
        return Promise.race([
          promise,
          new Promise((_, reject) =>
            setTimeout(() => reject(new Error("Timeout")), timeoutMs),
          ),
        ]);
      };

      const fastTask = new Promise((resolve) =>
        setTimeout(() => resolve("fast"), 10),
      );

      const slowTask = new Promise((resolve) =>
        setTimeout(() => resolve("slow"), 1000),
      );

      // Fast task should win
      const result1 = await withTimeout(fastTask, 100);
      expect(result1).toBe("fast");

      // Timeout should win
      await expect(withTimeout(slowTask, 50)).rejects.toThrow("Timeout");
    });

    it("should implement sequential Promise execution", async () => {
      const executeSequential = async (tasks) => {
        const results = [];
        for (const task of tasks) {
          const result = await task();
          results.push(result);
        }
        return results;
      };

      const order = [];
      const tasks = [
        () =>
          new Promise((resolve) => {
            setTimeout(() => {
              order.push(1);
              resolve("task1");
            }, 30);
          }),
        () =>
          new Promise((resolve) => {
            setTimeout(() => {
              order.push(2);
              resolve("task2");
            }, 10);
          }),
        () =>
          new Promise((resolve) => {
            setTimeout(() => {
              order.push(3);
              resolve("task3");
            }, 20);
          }),
      ];

      const results = await executeSequential(tasks);

      expect(results).toEqual(["task1", "task2", "task3"]);
      expect(order).toEqual([1, 2, 3]);
    });

    it("should handle Promise.all with concurrency limit", async () => {
      const parallelLimit = async (tasks, limit) => {
        const results = [];
        const executing = new Set();

        for (const task of tasks) {
          const promise = Promise.resolve().then(() => task());
          results.push(promise);
          executing.add(promise);

          const clean = () => executing.delete(promise);
          promise.then(clean, clean);

          if (executing.size >= limit) {
            await Promise.race(executing);
          }
        }

        return Promise.all(results);
      };

      let concurrent = 0;
      let maxConcurrent = 0;

      const createTask = (id, duration) => () => {
        return new Promise((resolve) => {
          concurrent++;
          maxConcurrent = Math.max(maxConcurrent, concurrent);
          setTimeout(() => {
            concurrent--;
            resolve(id);
          }, duration);
        });
      };

      const tasks = Array.from({ length: 10 }, (_, i) => createTask(i, 10));

      const results = await parallelLimit(tasks, 3);

      expect(results).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
      expect(maxConcurrent).toBe(3);
    });
  });

  describe("Error Handling in Async Code", () => {
    it("should handle async errors with proper stack traces", async () => {
      const asyncError = async () => {
        await Promise.resolve();
        throw new Error("Async error");
      };

      try {
        await asyncError();
        expect(true).toBe(false);
      } catch (error) {
        expect(error.message).toBe("Async error");
        expect(error.stack).toBeDefined();
      }
    });

    it("should handle unhandled rejection detection", async () => {
      const captureUnhandledRejections = () => {
        const rejections = [];
        const handler = (reason) => {
          rejections.push(reason);
        };
        process.on("unhandledRejection", handler);
        return {
          rejections,
          cleanup: () => process.off("unhandledRejection", handler),
        };
      };

      const capture = captureUnhandledRejections();

      // Create an unhandled rejection
      Promise.reject(new Error("Unhandled"));

      // Give it time to be captured
      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(capture.rejections.length).toBeGreaterThan(0);
      capture.cleanup();
    });

    it("should implement async retry with backoff", async () => {
      const retryWithBackoff = async (fn, maxRetries, initialDelay) => {
        let lastError;

        for (let i = 0; i < maxRetries; i++) {
          try {
            return await fn();
          } catch (error) {
            lastError = error;
            if (i < maxRetries - 1) {
              const delay = initialDelay * Math.pow(2, i);
              await new Promise((resolve) => setTimeout(resolve, delay));
            }
          }
        }

        throw lastError;
      };

      let attempts = 0;
      const flakyFn = async () => {
        attempts++;
        if (attempts < 3) {
          throw new Error(`Attempt ${attempts} failed`);
        }
        return "success";
      };

      const result = await retryWithBackoff(flakyFn, 5, 10);
      expect(result).toBe("success");
      expect(attempts).toBe(3);
    });

    it("should handle async generator patterns", async () => {
      const asyncGenerator = async function* () {
        for (let i = 0; i < 5; i++) {
          await new Promise((resolve) => setTimeout(resolve, 10));
          yield i;
        }
      };

      const results = [];
      for await (const value of asyncGenerator()) {
        results.push(value);
      }

      expect(results).toEqual([0, 1, 2, 3, 4]);
    });

    it("should handle async queue pattern", () => {
      const createAsyncQueue = () => {
        const queue = [];
        let processing = false;

        return {
          async enqueue(task) {
            return new Promise((resolve, reject) => {
              queue.push({ task, resolve, reject });
              this.process();
            });
          },

          async process() {
            if (processing) return;
            processing = true;

            while (queue.length > 0) {
              const { task, resolve, reject } = queue.shift();
              try {
                const result = await task();
                resolve(result);
              } catch (error) {
                reject(error);
              }
            }

            processing = false;
          },

          get pending() {
            return queue.length;
          },

          get isProcessing() {
            return processing;
          },
        };
      };

      const queue = createAsyncQueue();
      const results = [];

      queue.enqueue(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
        results.push(1);
        return 1;
      });

      queue.enqueue(async () => {
        results.push(2);
        return 2;
      });

      queue.enqueue(async () => {
        results.push(3);
        return 3;
      });

      // Queue should process in order
      return new Promise((resolve) => {
        setTimeout(() => {
          expect(results).toEqual([1, 2, 3]);
          resolve();
        }, 50);
      });
    });
  });

  describe("Async Context and Propagation", () => {
    it("should maintain async context through nested calls", async () => {
      const createContext = () => {
        let store = new Map();

        return {
          set(key, value) {
            store.set(key, value);
          },

          get(key) {
            return store.get(key);
          },

          async run(fn) {
            const previous = store;
            store = new Map(previous);
            try {
              return await fn();
            } finally {
              store = previous;
            }
          },
        };
      };

      const ctx = createContext();
      ctx.set("requestId", "123");

      const result = await ctx.run(async () => {
        expect(ctx.get("requestId")).toBe("123");
        ctx.set("nested", "value");
        return "done";
      });

      expect(result).toBe("done");
      expect(ctx.get("nested")).toBe(undefined);
      expect(ctx.get("requestId")).toBe("123");
    });

    it("should handle async local storage pattern", async () => {
      const createALS = () => {
        let currentStore = null;

        return {
          getStore() {
            return currentStore;
          },

          run(store, fn) {
            const previous = currentStore;
            currentStore = store;
            let result;
            let isAsync = false;
            try {
              result = fn();
              if (result && typeof result.then === "function") {
                isAsync = true;
                return result.finally(() => {
                  currentStore = previous;
                });
              }
              return result;
            } finally {
              if (!isAsync) {
                currentStore = previous;
              }
            }
          },
        };
      };

      const als = createALS();

      expect(als.getStore()).toBe(null);

      als.run({ id: 1 }, () => {
        expect(als.getStore()).toEqual({ id: 1 });

        als.run({ id: 2 }, () => {
          expect(als.getStore()).toEqual({ id: 2 });
        });

        expect(als.getStore()).toEqual({ id: 1 });
      });

      expect(als.getStore()).toBe(null);
    });

    it("should propagate cancellation tokens", async () => {
      const createCancellationToken = () => {
        let cancelled = false;
        const callbacks = [];

        return {
          get isCancelled() {
            return cancelled;
          },

          cancel() {
            cancelled = true;
            callbacks.forEach((cb) => cb());
            callbacks.length = 0;
          },

          onCancelled(callback) {
            if (cancelled) {
              callback();
            } else {
              callbacks.push(callback);
            }
          },

          throwIfCancelled() {
            if (cancelled) {
              throw new Error("Operation cancelled");
            }
          },
        };
      };

      const token = createCancellationToken();

      const longRunningTask = async () => {
        for (let i = 0; i < 100; i++) {
          token.throwIfCancelled();
          await new Promise((resolve) => setTimeout(resolve, 5));
        }
        return "completed";
      };

      // Cancel after a short delay
      setTimeout(() => token.cancel(), 20);

      await expect(longRunningTask()).rejects.toThrow("Operation cancelled");
    });
  });

  describe("Async Debouncing and Throttling", () => {
    it("should implement async debounce", async () => {
      const createAsyncDebounce = (fn, delay) => {
        let timeoutId = null;
        let lastArgs = null;

        return (...args) => {
          lastArgs = args;

          if (timeoutId) {
            clearTimeout(timeoutId);
          }

          return new Promise((resolve, reject) => {
            timeoutId = setTimeout(async () => {
              try {
                const result = await fn(...lastArgs);
                resolve(result);
              } catch (error) {
                reject(error);
              }
            }, delay);
          });
        };
      };

      const fn = vi.fn().mockResolvedValue("result");
      const debounced = createAsyncDebounce(fn, 50);

      // Call multiple times rapidly
      debounced();
      debounced();
      debounced();

      // Wait for debounce
      await new Promise((resolve) => setTimeout(resolve, 100));

      expect(fn).toHaveBeenCalledTimes(1);
    });

    it("should implement async throttle", async () => {
      const createAsyncThrottle = (fn, interval) => {
        let lastCall = 0;
        let pending = null;

        return async (...args) => {
          const now = Date.now();

          if (now - lastCall >= interval) {
            lastCall = now;
            return fn(...args);
          }

          if (!pending) {
            pending = new Promise((resolve) => {
              setTimeout(
                () => {
                  lastCall = Date.now();
                  resolve(fn(...args));
                  pending = null;
                },
                interval - (now - lastCall),
              );
            });
          }

          return pending;
        };
      };

      const fn = vi.fn().mockResolvedValue("result");
      const throttled = createAsyncThrottle(fn, 50);

      await throttled();
      expect(fn).toHaveBeenCalledTimes(1);

      // Immediate second call should be throttled
      const result = await throttled();
      expect(result).toBe("result");
    });

    it("should implement trailing edge debounce", async () => {
      const createTrailingDebounce = (fn, delay) => {
        let timeoutId = null;
        let lastArgs = null;

        return (...args) => {
          lastArgs = args;

          if (timeoutId) {
            clearTimeout(timeoutId);
          }

          return new Promise((resolve) => {
            timeoutId = setTimeout(async () => {
              const result = await fn(...lastArgs);
              timeoutId = null;
              resolve(result);
            }, delay);
          });
        };
      };

      const fn = vi.fn().mockImplementation((x) => x * 2);
      const debounced = createTrailingDebounce(fn, 30);

      debounced(1);
      debounced(2);
      const result = await debounced(3);

      expect(fn).toHaveBeenCalledWith(3);
      expect(result).toBe(6);
    });
  });

  describe("Async Event Emitter Patterns", () => {
    it("should emit async events with await", async () => {
      const createAsyncEmitter = () => {
        const listeners = new Map();

        return {
          on(event, listener) {
            if (!listeners.has(event)) {
              listeners.set(event, []);
            }
            listeners.get(event).push(listener);
            return () => this.off(event, listener);
          },

          off(event, listener) {
            const eventListeners = listeners.get(event);
            if (eventListeners) {
              const idx = eventListeners.indexOf(listener);
              if (idx !== -1) eventListeners.splice(idx, 1);
            }
          },

          async emit(event, ...args) {
            const eventListeners = listeners.get(event) || [];
            const results = [];

            for (const listener of eventListeners) {
              const result = await listener(...args);
              results.push(result);
            }

            return results;
          },
        };
      };

      const emitter = createAsyncEmitter();
      const results = [];

      emitter.on("data", async (value) => {
        await new Promise((resolve) => setTimeout(resolve, 10));
        results.push(value * 2);
        return value * 2;
      });

      emitter.on("data", async (value) => {
        results.push(value * 3);
        return value * 3;
      });

      const emitResults = await emitter.emit("data", 5);

      expect(results).toEqual([10, 15]);
      expect(emitResults).toEqual([10, 15]);
    });

    it("should handle async event error handling", async () => {
      const createSafeEmitter = () => {
        const listeners = new Map();
        const errorHandlers = [];

        return {
          on(event, listener) {
            if (!listeners.has(event)) {
              listeners.set(event, []);
            }
            listeners.get(event).push(listener);
          },

          onError(handler) {
            errorHandlers.push(handler);
          },

          async emit(event, ...args) {
            const eventListeners = listeners.get(event) || [];

            for (const listener of eventListeners) {
              try {
                await listener(...args);
              } catch (error) {
                errorHandlers.forEach((handler) => handler(error, event));
              }
            }
          },
        };
      };

      const emitter = createSafeEmitter();
      const errors = [];

      emitter.onError((error) => {
        errors.push(error.message);
      });

      emitter.on("event", async () => {
        throw new Error("Listener error 1");
      });

      emitter.on("event", async () => {
        throw new Error("Listener error 2");
      });

      emitter.on("event", async () => {
        return "success";
      });

      await emitter.emit("event");

      expect(errors).toEqual(["Listener error 1", "Listener error 2"]);
    });
  });

  describe("Async Iteration Patterns", () => {
    it("should handle async iterable streams", async () => {
      const createAsyncStream = (items, delay) => {
        return {
          [Symbol.asyncIterator]() {
            let index = 0;
            return {
              async next() {
                if (index >= items.length) {
                  return { done: true };
                }
                await new Promise((resolve) => setTimeout(resolve, delay));
                return { value: items[index++], done: false };
              },
            };
          },
        };
      };

      const stream = createAsyncStream([1, 2, 3, 4, 5], 10);
      const results = [];

      for await (const item of stream) {
        results.push(item);
      }

      expect(results).toEqual([1, 2, 3, 4, 5]);
    });

    it("should handle async iterator with break", async () => {
      const createInfiniteStream = () => {
        return {
          [Symbol.asyncIterator]() {
            let count = 0;
            return {
              async next() {
                return { value: count++, done: false };
              },
            };
          },
        };
      };

      const stream = createInfiniteStream();
      const results = [];

      for await (const item of stream) {
        results.push(item);
        if (item >= 4) break;
      }

      expect(results).toEqual([0, 1, 2, 3, 4]);
    });

    it("should implement async iterator with transform", async () => {
      const createTransformStream = async function* (source, transform) {
        for await (const item of source) {
          yield await transform(item);
        }
      };

      const source = {
        async *[Symbol.asyncIterator]() {
          yield 1;
          yield 2;
          yield 3;
        },
      };

      const transformed = createTransformStream(source, async (x) => x * 2);
      const results = [];

      for await (const item of transformed) {
        results.push(item);
      }

      expect(results).toEqual([2, 4, 6]);
    });

    it("should handle async iterator error recovery", async () => {
      const createFaultTolerantStream = (items) => {
        return {
          async *[Symbol.asyncIterator]() {
            for (const item of items) {
              if (item === null) {
                throw new Error("Null item");
              }
              yield item;
            }
          },
        };
      };

      const items = [1, 2, null, 4, 5];
      const stream = createFaultTolerantStream(items);
      const results = [];

      try {
        for await (const item of stream) {
          results.push(item);
        }
      } catch (error) {
        expect(error.message).toBe("Null item");
      }

      expect(results).toEqual([1, 2]);
    });
  });

  describe("Async Coordination Patterns", () => {
    it("should implement async barrier", async () => {
      const createBarrier = (count) => {
        let waiting = 0;
        let resolvers = [];

        return {
          async wait() {
            waiting++;

            if (waiting >= count) {
              // Release all waiting promises
              const toResolve = resolvers;
              resolvers = [];
              waiting = 0;
              toResolve.forEach((r) => r());
              return;
            }

            await new Promise((r) => {
              resolvers.push(r);
            });
          },

          get waitingCount() {
            return waiting;
          },
        };
      };

      const barrier = createBarrier(3);
      const results = [];

      const task1 = async () => {
        await new Promise((r) => setTimeout(r, 30));
        results.push("task1 done");
        await barrier.wait();
      };

      const task2 = async () => {
        await new Promise((r) => setTimeout(r, 20));
        results.push("task2 done");
        await barrier.wait();
      };

      const task3 = async () => {
        await new Promise((r) => setTimeout(r, 10));
        results.push("task3 done");
        await barrier.wait();
      };

      await Promise.all([task1(), task2(), task3()]);

      expect(results).toContain("task1 done");
      expect(results).toContain("task2 done");
      expect(results).toContain("task3 done");
    });

    it("should implement async semaphore", async () => {
      const createSemaphore = (maxPermits) => {
        let permits = maxPermits;
        const queue = [];

        return {
          async acquire() {
            if (permits > 0) {
              permits--;
              return;
            }

            await new Promise((resolve) => {
              queue.push(resolve);
            });
          },

          release() {
            if (queue.length > 0) {
              const resolve = queue.shift();
              resolve();
            } else {
              permits++;
            }
          },

          get availablePermits() {
            return permits;
          },

          get waitingCount() {
            return queue.length;
          },
        };
      };

      const semaphore = createSemaphore(2);

      expect(semaphore.availablePermits).toBe(2);

      await semaphore.acquire();
      await semaphore.acquire();
      expect(semaphore.availablePermits).toBe(0);

      const waitingPromise = semaphore.acquire();
      expect(semaphore.waitingCount).toBe(1);

      semaphore.release();
      expect(semaphore.availablePermits).toBe(0);

      await waitingPromise;
      expect(semaphore.waitingCount).toBe(0);
    });

    it("should implement async mutex", async () => {
      const createMutex = () => {
        let locked = false;
        const queue = [];

        return {
          async lock() {
            if (!locked) {
              locked = true;
              return;
            }

            await new Promise((resolve) => {
              queue.push(resolve);
            });
          },

          unlock() {
            if (queue.length > 0) {
              const resolve = queue.shift();
              resolve();
            } else {
              locked = false;
            }
          },

          get isLocked() {
            return locked;
          },
        };
      };

      const mutex = createMutex();
      const results = [];

      const task = async (id) => {
        await mutex.lock();
        results.push(`start-${id}`);
        await new Promise((r) => setTimeout(r, 10));
        results.push(`end-${id}`);
        mutex.unlock();
      };

      await Promise.all([task("a"), task("b"), task("c")]);

      expect(results).toEqual([
        "start-a",
        "end-a",
        "start-b",
        "end-b",
        "start-c",
        "end-c",
      ]);
    });
  });
});
