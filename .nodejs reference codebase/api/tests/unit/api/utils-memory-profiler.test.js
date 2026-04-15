/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { MemoryProfiler, memory } from "@api/utils/memory-profiler.js";

describe("api/utils/memory-profiler.js", () => {
  let profiler;

  beforeEach(() => {
    profiler = new MemoryProfiler();
  });

  afterEach(() => {
    profiler.dispose();
  });

  describe("MemoryProfiler", () => {
    describe("constructor", () => {
      it("should initialize with default values", () => {
        const p = new MemoryProfiler();
        expect(p.isTracking).toBe(false);
        p.dispose();
      });
    });

    describe("getUsage", () => {
      it("should return memory usage object with required fields", () => {
        const usage = profiler.getUsage();
        expect(usage).toHaveProperty("heapUsed");
        expect(usage).toHaveProperty("heapTotal");
        expect(usage).toHaveProperty("external");
        expect(usage).toHaveProperty("rss");
        expect(usage).toHaveProperty("timestamp");
        expect(typeof usage.heapUsed).toBe("number");
        expect(typeof usage.heapTotal).toBe("number");
      });

      it("should return values in MB", () => {
        const usage = profiler.getUsage();
        expect(usage.heapUsed).toBeGreaterThan(0);
      });
    });

    describe("getSnapshot", () => {
      it("should return snapshot with current and delta", () => {
        const snapshot = profiler.getSnapshot();
        expect(snapshot).toHaveProperty("current");
        expect(snapshot).toHaveProperty("delta");
        expect(snapshot).toHaveProperty("timestamp");
      });

      it("should track multiple snapshots with limit", () => {
        for (let i = 0; i < 15; i++) {
          profiler.getSnapshot();
        }
        const snapshots = profiler.getSnapshots();
        expect(snapshots.length).toBeLessThanOrEqual(10);
      });

      it("should keep exactly 10 snapshots at limit", () => {
        for (let i = 0; i < 10; i++) {
          profiler.getSnapshot();
        }
        expect(profiler.getSnapshots().length).toBe(10);

        profiler.getSnapshot();
        expect(profiler.getSnapshots().length).toBe(10);
      });
    });

    describe("startTracking", () => {
      it("should set isTracking to true", () => {
        profiler.startTracking(100);
        expect(profiler.isTracking).toBe(true);
        profiler.stopTracking();
      });

      it("should not start multiple tracking intervals", () => {
        profiler.startTracking(100);
        profiler.startTracking(100);
        expect(profiler.isTracking).toBe(true);
        profiler.stopTracking();
      });

      it("should accept custom interval", () => {
        profiler.startTracking(5000);
        expect(profiler.isTracking).toBe(true);
        profiler.stopTracking();
      });
    });

    describe("stopTracking", () => {
      it("should set isTracking to false", () => {
        profiler.startTracking(100);
        profiler.stopTracking();
        expect(profiler.isTracking).toBe(false);
      });

      it("should handle multiple stop calls", () => {
        profiler.startTracking(100);
        profiler.stopTracking();
        profiler.stopTracking();
        expect(profiler.isTracking).toBe(false);
      });
    });

    describe("getSnapshots", () => {
      it("should return array of snapshots", () => {
        profiler.getSnapshot();
        profiler.getSnapshot();
        const snapshots = profiler.getSnapshots();
        expect(Array.isArray(snapshots)).toBe(true);
        expect(snapshots.length).toBe(2);
      });

      it("should return copy of internal array", () => {
        profiler.getSnapshot();
        const snapshots1 = profiler.getSnapshots();
        const snapshots2 = profiler.getSnapshots();
        expect(snapshots1).not.toBe(snapshots2);
        expect(snapshots1).toEqual(snapshots2);
      });
    });

    describe("resetBaseline", () => {
      it("should reset baseline without throwing", () => {
        profiler.getSnapshot();
        expect(() => profiler.resetBaseline()).not.toThrow();
      });

      it("should update baseline after reset", () => {
        const before = profiler.getSnapshot();
        profiler.resetBaseline();
        const after = profiler.getSnapshot();
        expect(after.delta.heapUsed).toBe(0);
      });
    });

    describe("detectLeaks", () => {
      it("should return hasLeak false with insufficient snapshots", () => {
        const result = profiler.detectLeaks(50);
        expect(result.hasLeak).toBe(false);
        expect(result.reason).toBe("Insufficient snapshots");
      });

      it("should detect leak when growth exceeds threshold", () => {
        vi.spyOn(profiler, "getUsage")
          .mockReturnValueOnce({
            heapUsed: 100,
            heapTotal: 200,
            external: 50,
            rss: 300,
            timestamp: Date.now(),
          })
          .mockReturnValueOnce({
            heapUsed: 200,
            heapTotal: 200,
            external: 50,
            rss: 300,
            timestamp: Date.now(),
          })
          .mockReturnValueOnce({
            heapUsed: 300,
            heapTotal: 200,
            external: 50,
            rss: 300,
            timestamp: Date.now(),
          });

        profiler.getSnapshot();
        profiler.getSnapshot();
        profiler.getSnapshot();

        const result = profiler.detectLeaks(50);
        expect(result.hasLeak).toBe(true);
        expect(result.growthMB).toBe(200);
      });

      it("should return no leak when growth is below threshold", () => {
        vi.spyOn(profiler, "getUsage")
          .mockReturnValueOnce({
            heapUsed: 100,
            heapTotal: 200,
            external: 50,
            rss: 300,
            timestamp: Date.now(),
          })
          .mockReturnValueOnce({
            heapUsed: 110,
            heapTotal: 200,
            external: 50,
            rss: 300,
            timestamp: Date.now(),
          });

        profiler.getSnapshot();
        profiler.getSnapshot();

        const result = profiler.detectLeaks(50);
        expect(result.hasLeak).toBe(false);
      });

      it("should return growthMB in non-leak case", () => {
        vi.spyOn(profiler, "getUsage")
          .mockReturnValueOnce({
            heapUsed: 100,
            heapTotal: 200,
            external: 50,
            rss: 300,
            timestamp: Date.now(),
          })
          .mockReturnValueOnce({
            heapUsed: 120,
            heapTotal: 200,
            external: 50,
            rss: 300,
            timestamp: Date.now(),
          });

        profiler.getSnapshot();
        profiler.getSnapshot();

        const result = profiler.detectLeaks(50);
        expect(result.hasLeak).toBe(false);
        expect(result.growthMB).toBe(20);
      });
    });

    describe("dispose", () => {
      it("should stop tracking and clear data", () => {
        profiler.startTracking(100);
        profiler.getSnapshot();
        profiler.dispose();
        expect(profiler.isTracking).toBe(false);
        expect(profiler.getSnapshots().length).toBe(0);
      });
    });
  });

  describe("memory exports", () => {
    it("should have all required methods", () => {
      expect(typeof memory.getUsage).toBe("function");
      expect(typeof memory.getSnapshot).toBe("function");
      expect(typeof memory.startTracking).toBe("function");
      expect(typeof memory.stopTracking).toBe("function");
      expect(typeof memory.getSnapshots).toBe("function");
      expect(typeof memory.resetBaseline).toBe("function");
      expect(typeof memory.detectLeaks).toBe("function");
      expect(typeof memory.dispose).toBe("function");
    });

    it("should have isTracking getter", () => {
      expect(memory.isTracking).toBeDefined();
    });

    it("should start and stop global tracking", () => {
      memory.startTracking(50);
      expect(memory.isTracking).toBe(true);
      memory.stopTracking();
      expect(memory.isTracking).toBe(false);
    });

    it("should get global snapshots", () => {
      memory.getSnapshot();
      const snapshots = memory.getSnapshots();
      expect(Array.isArray(snapshots)).toBe(true);
      expect(snapshots.length).toBeGreaterThan(0);
    });

    it("should reset global baseline", () => {
      memory.getSnapshot();
      expect(() => memory.resetBaseline()).not.toThrow();
    });

    it("should detect leaks globally", () => {
      const result = memory.detectLeaks(100);
      expect(result).toHaveProperty("hasLeak");
    });

    it("should dispose global profiler", () => {
      memory.startTracking(50);
      memory.getSnapshot();
      memory.dispose();
      expect(memory.isTracking).toBe(false);
      expect(memory.getSnapshots().length).toBe(0);
    });
  });
});
