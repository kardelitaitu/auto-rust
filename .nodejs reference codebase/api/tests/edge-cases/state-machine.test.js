/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Edge Case Tests: State Machine and Event-Driven Patterns
 *
 * Tests for handling state transitions and event systems:
 * - State machine transitions
 * - Invalid state handling
 * - Event emitter patterns
 * - Pub/sub systems
 * - Observable patterns
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

describe("Edge Cases: State Machine", () => {
  describe("Basic State Machine", () => {
    it("should implement a simple state machine", () => {
      const createStateMachine = (initialState, transitions) => {
        let state = initialState;
        const listeners = [];

        return {
          getState: () => state,

          transition(action) {
            const transition = transitions.find(
              (t) => t.from === state && t.action === action,
            );

            if (!transition) {
              throw new Error(
                `Invalid transition: ${action} from state ${state}`,
              );
            }

            const oldState = state;
            state = transition.to;

            listeners.forEach((listener) => listener(oldState, state, action));

            return state;
          },

          can(action) {
            return transitions.some(
              (t) => t.from === state && t.action === action,
            );
          },

          onTransition(listener) {
            listeners.push(listener);
            return () => {
              const idx = listeners.indexOf(listener);
              if (idx !== -1) listeners.splice(idx, 1);
            };
          },
        };
      };

      const machine = createStateMachine("idle", [
        { from: "idle", action: "start", to: "running" },
        { from: "running", action: "pause", to: "paused" },
        { from: "paused", action: "resume", to: "running" },
        { from: "running", action: "complete", to: "done" },
        { from: "paused", action: "stop", to: "idle" },
        { from: "running", action: "error", to: "failed" },
        { from: "failed", action: "reset", to: "idle" },
      ]);

      expect(machine.getState()).toBe("idle");
      expect(machine.can("start")).toBe(true);
      expect(machine.can("pause")).toBe(false);

      machine.transition("start");
      expect(machine.getState()).toBe("running");
      expect(machine.can("pause")).toBe(true);
      expect(machine.can("start")).toBe(false);
    });

    it("should handle invalid transitions", () => {
      const machine = {
        state: "idle",
        transitions: {
          idle: { start: "running" },
          running: { pause: "paused", complete: "done" },
        },

        transition(action) {
          const valid = this.transitions[this.state]?.[action];
          if (!valid) {
            throw new Error(`Cannot ${action} from ${this.state}`);
          }
          this.state = valid;
          return this.state;
        },
      };

      expect(() => machine.transition("pause")).toThrow(
        "Cannot pause from idle",
      );
      machine.transition("start");
      expect(machine.state).toBe("running");
    });

    it("should support hierarchical states", () => {
      const createHierarchicalSM = () => {
        const states = {
          idle: { parent: null },
          running: { parent: null },
          "running.active": { parent: "running" },
          "running.paused": { parent: "running" },
          done: { parent: null },
        };

        let currentState = "idle";

        return {
          getState: () => currentState,

          inState(state) {
            // Check if current state matches or is child of given state
            let check = currentState;
            while (check) {
              if (check === state) return true;
              check = states[check]?.parent;
            }
            return false;
          },

          transition(newState) {
            if (!states[newState]) {
              throw new Error(`Unknown state: ${newState}`);
            }
            currentState = newState;
          },
        };
      };

      const sm = createHierarchicalSM();

      expect(sm.getState()).toBe("idle");
      expect(sm.inState("idle")).toBe(true);

      sm.transition("running.active");
      expect(sm.inState("running")).toBe(true);
      expect(sm.inState("running.active")).toBe(true);
      expect(sm.inState("idle")).toBe(false);
    });

    it("should implement guard conditions", () => {
      const guardedMachine = {
        state: "idle",
        guards: {
          start: () => true,
          stop: () => false, // Cannot stop when already idle
        },

        can(action) {
          const guard = this.guards[action];
          return guard ? guard() : false;
        },

        transition(action) {
          if (!this.can(action)) {
            throw new Error(`Guard prevented: ${action}`);
          }
          this.state = action === "start" ? "running" : "idle";
        },
      };

      expect(guardedMachine.can("start")).toBe(true);
      expect(guardedMachine.can("stop")).toBe(false);
      expect(() => guardedMachine.transition("stop")).toThrow(
        "Guard prevented",
      );
    });
  });

  describe("Finite State Machine Patterns", () => {
    it("should implement traffic light FSM", () => {
      const createTrafficLight = () => {
        const states = ["red", "yellow", "green"];
        let currentIndex = 0;
        const timers = { red: 30, yellow: 5, green: 25 };
        const transitions = {
          red: ["green"],
          green: ["yellow"],
          yellow: ["red"],
        };

        return {
          get state() {
            return states[currentIndex];
          },

          get duration() {
            return timers[this.state];
          },

          next() {
            currentIndex = (currentIndex + 1) % states.length;
            return this.state;
          },

          canGo(state) {
            const current = this.state;
            return transitions[current]?.includes(state) || false;
          },
        };
      };

      const trafficLight = createTrafficLight();

      expect(trafficLight.state).toBe("red");
      expect(trafficLight.duration).toBe(30);
      expect(trafficLight.canGo("green")).toBe(true);
      expect(trafficLight.canGo("yellow")).toBe(false);

      trafficLight.next();
      expect(trafficLight.state).toBe("yellow");

      trafficLight.next();
      expect(trafficLight.state).toBe("green");

      trafficLight.next();
      expect(trafficLight.state).toBe("red");
    });

    it("should implement turnstile FSM", () => {
      const createTurnstile = () => {
        let state = "locked";
        const coins = [];

        return {
          getState: () => state,

          insertCoin(coin) {
            coins.push(coin);
            if (state === "locked") {
              state = "unlocked";
            }
            return { accepted: true, state };
          },

          push() {
            if (state === "locked") {
              return { opened: false, message: "Please insert coin" };
            }
            state = "locked";
            return { opened: true, message: "Welcome!" };
          },

          refund() {
            const returned = [...coins];
            coins.length = 0;
            state = "locked";
            return { coins: returned };
          },
        };
      };

      const turnstile = createTurnstile();

      expect(turnstile.getState()).toBe("locked");
      expect(turnstile.push().opened).toBe(false);

      turnstile.insertCoin("quarter");
      expect(turnstile.getState()).toBe("unlocked");
      expect(turnstile.push().opened).toBe(true);
      expect(turnstile.getState()).toBe("locked");
    });

    it("should implement workflow with pending states", () => {
      const createWorkflow = (steps) => {
        let currentStep = 0;
        const history = [];

        return {
          current: () => steps[currentStep],
          isComplete: () => currentStep >= steps.length,

          advance(result) {
            if (this.isComplete()) {
              throw new Error("Workflow already complete");
            }

            history.push({
              step: steps[currentStep],
              result,
              timestamp: Date.now(),
            });
            currentStep++;

            return {
              done: this.isComplete(),
              currentStep: this.isComplete() ? null : steps[currentStep],
            };
          },

          reset() {
            currentStep = 0;
            history.length = 0;
          },

          getHistory: () => [...history],
        };
      };

      const workflow = createWorkflow([
        "validate",
        "process",
        "save",
        "notify",
      ]);

      expect(workflow.current()).toBe("validate");
      expect(workflow.isComplete()).toBe(false);

      workflow.advance({ valid: true });
      expect(workflow.current()).toBe("process");

      workflow.advance({ processed: true });
      workflow.advance({ saved: true });
      workflow.advance({ notified: true });

      expect(workflow.isComplete()).toBe(true);
      expect(workflow.getHistory()).toHaveLength(4);
    });
  });

  describe("Context and History Tracking", () => {
    it("should track state history", () => {
      const createHistoricalSM = (initial) => {
        let state = initial;
        const history = [{ state: initial, timestamp: Date.now() }];

        return {
          getState: () => state,

          transition(newState) {
            state = newState;
            history.push({ state: newState, timestamp: Date.now() });
          },

          getHistory: () => [...history],

          getPrevious: () => history[history.length - 2]?.state,

          canUndo: () => history.length > 1,
        };
      };

      const sm = createHistoricalSM("start");

      sm.transition("middle");
      sm.transition("end");

      expect(sm.getHistory()).toHaveLength(3);
      expect(sm.getPrevious()).toBe("middle");
      expect(sm.canUndo()).toBe(true);
    });

    it("should implement state with context data", () => {
      const createContextSM = (initialState, initialContext) => {
        let state = initialState;
        let context = { ...initialContext };

        return {
          getState: () => state,
          getContext: () => ({ ...context }),

          transition(newState, newContext = {}) {
            state = newState;
            context = { ...context, ...newContext };
          },

          updateContext(updates) {
            context = { ...context, ...updates };
          },
        };
      };

      const sm = createContextSM("idle", { retries: 0, lastError: null });

      sm.transition("retrying", { retries: 1 });
      expect(sm.getState()).toBe("retrying");
      expect(sm.getContext().retries).toBe(1);

      sm.updateContext({ lastError: "timeout" });
      expect(sm.getContext().lastError).toBe("timeout");
      expect(sm.getContext().retries).toBe(1);
    });
  });
});

describe("Edge Cases: Event-Driven", () => {
  describe("Event Emitter", () => {
    it("should implement event emitter", () => {
      const createEventEmitter = () => {
        const events = new Map();

        return {
          on(event, handler) {
            if (!events.has(event)) {
              events.set(event, []);
            }
            events.get(event).push(handler);

            return () => {
              const handlers = events.get(event);
              const idx = handlers.indexOf(handler);
              if (idx !== -1) handlers.splice(idx, 1);
            };
          },

          emit(event, data) {
            const handlers = events.get(event) || [];
            handlers.forEach((handler) => handler(data));
          },

          off(event, handler) {
            const handlers = events.get(event);
            if (handlers) {
              const idx = handlers.indexOf(handler);
              if (idx !== -1) handlers.splice(idx, 1);
            }
          },

          removeAllListeners(event) {
            if (event) {
              events.delete(event);
            } else {
              events.clear();
            }
          },

          listenerCount(event) {
            return (events.get(event) || []).length;
          },
        };
      };

      const emitter = createEventEmitter();
      const results = [];

      emitter.on("data", (d) => results.push(d));
      emitter.emit("data", 1);
      emitter.emit("data", 2);

      expect(results).toEqual([1, 2]);
      expect(emitter.listenerCount("data")).toBe(1);
    });

    it("should handle once listeners", () => {
      const createEventEmitter = () => {
        const events = new Map();

        return {
          on(event, handler) {
            if (!events.has(event)) {
              events.set(event, []);
            }
            events.get(event).push({ handler, once: false });
          },

          once(event, handler) {
            if (!events.has(event)) {
              events.set(event, []);
            }
            events.get(event).push({ handler, once: true });
          },

          emit(event, data) {
            const handlers = events.get(event) || [];
            const remaining = [];

            for (const { handler, once } of handlers) {
              handler(data);
              if (!once) {
                remaining.push({ handler, once });
              }
            }

            events.set(event, remaining);
          },
        };
      };

      const emitter = createEventEmitter();
      let count = 0;

      emitter.once("tick", () => count++);
      emitter.emit("tick");
      emitter.emit("tick");

      expect(count).toBe(1);
    });

    it("should emit error events", () => {
      const emitter = {
        handlers: new Map(),

        on(event, handler) {
          if (!this.handlers.has(event)) {
            this.handlers.set(event, []);
          }
          this.handlers.get(event).push(handler);
        },

        emit(event, data) {
          const handlers = this.handlers.get(event) || [];
          for (const handler of handlers) {
            try {
              handler(data);
            } catch (e) {
              // Error in handler shouldn't stop others
              if (event !== "error") {
                this.emit("error", e);
              }
            }
          }
        },
      };

      const errors = [];
      const results = [];

      emitter.on("error", (e) => errors.push(e.message));
      emitter.on("data", () => {
        throw new Error("handler failed");
      });
      emitter.on("data", () => results.push("ok"));

      emitter.emit("data");

      expect(results).toEqual(["ok"]);
      expect(errors).toContain("handler failed");
    });
  });

  describe("Pub/Sub Pattern", () => {
    it("should implement pub/sub system", () => {
      const createPubSub = () => {
        const subscriptions = new Map();

        return {
          subscribe(topic, callback) {
            if (!subscriptions.has(topic)) {
              subscriptions.set(topic, []);
            }
            const subs = subscriptions.get(topic);
            subs.push(callback);

            return () => {
              const idx = subs.indexOf(callback);
              if (idx !== -1) subs.splice(idx, 1);
            };
          },

          publish(topic, data) {
            const subs = subscriptions.get(topic) || [];
            subs.forEach((callback) => callback(data));
          },

          unsubscribe(topic, callback) {
            const subs = subscriptions.get(topic);
            if (subs) {
              const idx = subs.indexOf(callback);
              if (idx !== -1) subs.splice(idx, 1);
            }
          },
        };
      };

      const pubsub = createPubSub();
      const messages = [];

      const unsub = pubsub.subscribe("news", (data) => messages.push(data));

      pubsub.publish("news", "headline 1");
      pubsub.publish("news", "headline 2");

      unsub();
      pubsub.publish("news", "headline 3");

      expect(messages).toEqual(["headline 1", "headline 2"]);
    });

    it("should support wildcard subscriptions", () => {
      const createWildcardPubSub = () => {
        const subscriptions = new Map();

        return {
          subscribe(pattern, callback) {
            if (!subscriptions.has(pattern)) {
              subscriptions.set(pattern, []);
            }
            subscriptions.get(pattern).push(callback);
          },

          publish(topic, data) {
            for (const [pattern, callbacks] of subscriptions) {
              if (this._matches(topic, pattern)) {
                callbacks.forEach((cb) => cb(data, topic));
              }
            }
          },

          _matches(topic, pattern) {
            if (pattern === "*") return true;
            if (pattern === topic) return true;

            const patternParts = pattern.split(".");
            const topicParts = topic.split(".");

            if (patternParts.length !== topicParts.length) return false;

            return patternParts.every(
              (part, i) => part === "*" || part === topicParts[i],
            );
          },
        };
      };

      const pubsub = createWildcardPubSub();
      const received = [];

      pubsub.subscribe("user.*.login", (data, topic) => {
        received.push({ topic, data });
      });

      pubsub.publish("user.admin.login", "admin data");
      pubsub.publish("user.guest.login", "guest data");
      pubsub.publish("user.admin.logout", "logout data");
      pubsub.publish("system.start", "system data");

      expect(received).toHaveLength(2);
      expect(received[0].topic).toBe("user.admin.login");
      expect(received[1].topic).toBe("user.guest.login");
    });

    it("should handle pub/sub errors gracefully", () => {
      const pubsub = {
        handlers: new Map(),
        errors: [],

        subscribe(topic, handler) {
          if (!this.handlers.has(topic)) {
            this.handlers.set(topic, []);
          }
          this.handlers.get(topic).push(handler);
        },

        publish(topic, data) {
          const handlers = this.handlers.get(topic) || [];
          for (const handler of handlers) {
            try {
              handler(data);
            } catch (e) {
              this.errors.push({ topic, error: e.message });
            }
          }
        },
      };

      pubsub.subscribe("events", () => {
        throw new Error("Handler failed");
      });
      pubsub.subscribe("events", () => {}); // This should still execute

      pubsub.publish("events", "data");

      expect(pubsub.errors).toHaveLength(1);
    });
  });

  describe("Observable Pattern", () => {
    it("should implement observable stream", () => {
      const createObservable = (subscriber) => {
        return {
          subscribe: subscriber,

          pipe(...operators) {
            return operators.reduce((obs, op) => op(obs), this);
          },

          map(fn) {
            return createObservable((observer) => {
              return this.subscribe({
                next: (val) => observer.next(fn(val)),
                error: (err) => observer.error(err),
                complete: () => observer.complete(),
              });
            });
          },

          filter(predicate) {
            return createObservable((observer) => {
              return this.subscribe({
                next: (val) => {
                  if (predicate(val)) observer.next(val);
                },
                error: (err) => observer.error(err),
                complete: () => observer.complete(),
              });
            });
          },

          take(count) {
            return createObservable((observer) => {
              let taken = 0;
              const sub = this.subscribe({
                next: (val) => {
                  if (taken < count) {
                    taken++;
                    observer.next(val);
                    if (taken === count) {
                      observer.complete();
                    }
                  }
                },
                error: (err) => observer.error(err),
                complete: () => {
                  if (taken < count) {
                    observer.complete();
                  }
                },
              });
              return sub;
            });
          },
        };
      };

      const results = [];
      const observable = createObservable((observer) => {
        observer.next(1);
        observer.next(2);
        observer.next(3);
        observer.next(4);
        observer.complete();
        return { unsubscribe: () => {} };
      });

      observable
        .filter((x) => x % 2 === 0)
        .map((x) => x * 10)
        .take(2)
        .subscribe({
          next: (val) => results.push(val),
          complete: () => results.push("done"),
        });

      // Only expect one 'done' at the end
      expect(results).toEqual([20, 40, "done"]);
    });

    it("should handle observable errors", () => {
      const createObservable = (subscriber) => ({
        subscribe: subscriber,
      });

      const errors = [];
      const observable = createObservable((observer) => {
        observer.next(1);
        observer.error(new Error("stream error"));
        return { unsubscribe: () => {} };
      });

      observable.subscribe({
        next: () => {},
        error: (err) => errors.push(err.message),
      });

      expect(errors).toEqual(["stream error"]);
    });
  });

  describe("Async Event Patterns", () => {
    it("should implement async event queue", async () => {
      const createAsyncQueue = () => {
        const queue = [];
        let processing = false;

        return {
          enqueue(item) {
            return new Promise((resolve, reject) => {
              queue.push({ item, resolve, reject });
              if (!processing) {
                this.process();
              }
            });
          },

          async process() {
            processing = true;
            while (queue.length > 0) {
              const { item, resolve, reject } = queue.shift();
              try {
                const result = await item();
                resolve(result);
              } catch (error) {
                reject(error);
              }
            }
            processing = false;
          },

          get size() {
            return queue.length;
          },
        };
      };

      const queue = createAsyncQueue();
      const results = [];

      await Promise.all([
        queue.enqueue(async () => {
          await new Promise((r) => setTimeout(r, 30));
          results.push(1);
          return 1;
        }),
        queue.enqueue(async () => {
          await new Promise((r) => setTimeout(r, 20));
          results.push(2);
          return 2;
        }),
        queue.enqueue(async () => {
          await new Promise((r) => setTimeout(r, 10));
          results.push(3);
          return 3;
        }),
      ]);

      // Queue processes in order
      expect(results).toEqual([1, 2, 3]);
    });

    it("should implement event debouncing", () => {
      const debounce = (fn, delay) => {
        let timeoutId;
        return (...args) => {
          clearTimeout(timeoutId);
          return new Promise((resolve) => {
            timeoutId = setTimeout(() => {
              resolve(fn(...args));
            }, delay);
          });
        };
      };

      const mockFn = vi.fn((x) => x * 2);
      const debounced = debounce(mockFn, 100);

      debounced(1);
      debounced(2);
      debounced(3);

      return new Promise((resolve) => {
        setTimeout(() => {
          expect(mockFn).toHaveBeenCalledTimes(1);
          expect(mockFn).toHaveBeenCalledWith(3);
          resolve();
        }, 150);
      });
    });

    it("should implement event throttling", () => {
      const throttle = (fn, limit) => {
        let lastCall = 0;
        let pending = null;

        return (...args) => {
          const now = Date.now();
          if (now - lastCall >= limit) {
            lastCall = now;
            return fn(...args);
          }
          if (!pending) {
            pending = new Promise((resolve) => {
              setTimeout(
                () => {
                  lastCall = Date.now();
                  pending = null;
                  resolve(fn(...args));
                },
                limit - (now - lastCall),
              );
            });
          }
          return pending;
        };
      };

      const mockFn = vi.fn((x) => x);
      const throttled = throttle(mockFn, 50);

      throttled(1);
      throttled(2);
      throttled(3);

      expect(mockFn).toHaveBeenCalledTimes(1);

      return new Promise((resolve) => {
        setTimeout(() => {
          expect(mockFn).toHaveBeenCalledTimes(2);
          resolve();
        }, 60);
      });
    });
  });

  describe("State Change Events", () => {
    it("should emit events on state change", () => {
      const createStateEmitter = (initialState) => {
        let state = initialState;
        const listeners = [];

        return {
          getState: () => state,

          setState(newState) {
            if (state !== newState) {
              const oldState = state;
              state = newState;
              listeners.forEach((l) => l({ oldState, newState }));
            }
          },

          onChange(listener) {
            listeners.push(listener);
            return () => {
              const idx = listeners.indexOf(listener);
              if (idx !== -1) listeners.splice(idx, 1);
            };
          },
        };
      };

      const emitter = createStateEmitter("idle");
      const changes = [];

      emitter.onChange((change) => changes.push(change));

      emitter.setState("running");
      emitter.setState("running"); // No change
      emitter.setState("done");

      expect(changes).toHaveLength(2);
      expect(changes[0]).toEqual({ oldState: "idle", newState: "running" });
      expect(changes[1]).toEqual({ oldState: "running", newState: "done" });
    });

    it("should implement state persistence", () => {
      const createPersistableSM = (initialState) => {
        let state = initialState;
        let saveFn = null;

        return {
          getState: () => state,

          setState(newState) {
            state = newState;
            if (saveFn) {
              saveFn(state);
            }
          },

          setPersistence(fn) {
            saveFn = fn;
          },

          load(savedState) {
            state = savedState;
          },
        };
      };

      const saved = [];
      const sm = createPersistableSM("idle");
      sm.setPersistence((s) => saved.push(s));

      sm.setState("running");
      sm.setState("done");

      expect(saved).toEqual(["running", "done"]);

      const sm2 = createPersistableSM("idle");
      sm2.load("restored");
      expect(sm2.getState()).toBe("restored");
    });
  });
});
