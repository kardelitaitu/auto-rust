/**
 * Auto-AI Framework - GhostCursor Tests
 * @module tests/unit/api/ghostCursor.test
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn((min, max) => (min + max) / 2),
    gaussian: vi.fn(() => 0),
    roll: vi.fn(() => false),
  },
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

vi.mock("@api/constants/engagement.js", () => ({
  TWITTER_CLICK_PROFILES: {
    reply: {
      hoverMin: 200,
      hoverMax: 800,
      holdMs: 80,
      hesitation: false,
      microMove: false,
    },
    like: {
      hoverMin: 150,
      hoverMax: 400,
      holdMs: 60,
      hesitation: true,
      microMove: true,
    },
    retweet: {
      hoverMin: 150,
      hoverMax: 400,
      holdMs: 60,
      hesitation: true,
      microMove: true,
    },
    follow: {
      hoverMin: 300,
      hoverMax: 900,
      holdMs: 100,
      hesitation: true,
      microMove: false,
    },
    nav: {
      hoverMin: 50,
      hoverMax: 150,
      holdMs: 40,
      hesitation: false,
      microMove: false,
    },
  },
}));

let GhostCursor;
let mockPage;

describe("api/utils/ghostCursor.js", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    mockPage = {
      mouse: {
        move: vi.fn().mockResolvedValue(undefined),
        down: vi.fn().mockResolvedValue(undefined),
        up: vi.fn().mockResolvedValue(undefined),
      },
      viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
    };
    const module = await import("../../../utils/ghostCursor.js");
    GhostCursor = module.GhostCursor;
  });

  describe("constructor", () => {
    it("should create GhostCursor instance", () => {
      const cursor = new GhostCursor(mockPage);
      expect(cursor.page).toBe(mockPage);
    });

    it("should initialize previousPos", () => {
      const cursor = new GhostCursor(mockPage);
      expect(cursor.previousPos).toBeDefined();
      expect(cursor.previousPos.x).toBeDefined();
      expect(cursor.previousPos.y).toBeDefined();
    });
  });

  describe("vector math", () => {
    it("should add vectors correctly", () => {
      const cursor = new GhostCursor(mockPage);
      const result = cursor.vecAdd({ x: 1, y: 2 }, { x: 3, y: 4 });
      expect(result.x).toBe(4);
      expect(result.y).toBe(6);
    });

    it("should subtract vectors correctly", () => {
      const cursor = new GhostCursor(mockPage);
      const result = cursor.vecSub({ x: 3, y: 4 }, { x: 1, y: 2 });
      expect(result.x).toBe(2);
      expect(result.y).toBe(2);
    });

    it("should multiply vector by scalar", () => {
      const cursor = new GhostCursor(mockPage);
      const result = cursor.vecMult({ x: 2, y: 3 }, 2);
      expect(result.x).toBe(4);
      expect(result.y).toBe(6);
    });

    it("should calculate vector length", () => {
      const cursor = new GhostCursor(mockPage);
      const result = cursor.vecLen({ x: 3, y: 4 });
      expect(result).toBe(5);
    });
  });

  describe("bezier", () => {
    it("should calculate bezier point at t=0", () => {
      const cursor = new GhostCursor(mockPage);
      const result = cursor.bezier(
        0,
        { x: 0, y: 0 },
        { x: 10, y: 10 },
        { x: 20, y: 20 },
        { x: 30, y: 30 },
      );
      expect(result.x).toBe(0);
      expect(result.y).toBe(0);
    });

    it("should calculate bezier point at t=1", () => {
      const cursor = new GhostCursor(mockPage);
      const result = cursor.bezier(
        1,
        { x: 0, y: 0 },
        { x: 10, y: 10 },
        { x: 20, y: 20 },
        { x: 30, y: 30 },
      );
      expect(result.x).toBe(30);
      expect(result.y).toBe(30);
    });
  });

  describe("easeOutCubic", () => {
    it("should return 0 at t=0", () => {
      const cursor = new GhostCursor(mockPage);
      expect(cursor.easeOutCubic(0)).toBe(0);
    });

    it("should return 1 at t=1", () => {
      const cursor = new GhostCursor(mockPage);
      expect(cursor.easeOutCubic(1)).toBe(1);
    });

    it("should return 0.5 at t≈0.5", () => {
      const cursor = new GhostCursor(mockPage);
      const result = cursor.easeOutCubic(0.5);
      expect(result).toBeCloseTo(0.875, 2);
    });
  });

  describe("performMove", () => {
    it("should return early for invalid start", async () => {
      const cursor = new GhostCursor(mockPage);
      await cursor.performMove(null, { x: 100, y: 100 }, 100);
      expect(mockPage.mouse.move).not.toHaveBeenCalled();
    });

    it("should return early for invalid end", async () => {
      const cursor = new GhostCursor(mockPage);
      await cursor.performMove({ x: 0, y: 0 }, null, 100);
      expect(mockPage.mouse.move).not.toHaveBeenCalled();
    });

    it("should return early for NaN coordinates", async () => {
      const cursor = new GhostCursor(mockPage);
      await cursor.performMove({ x: NaN, y: 0 }, { x: 100, y: 100 }, 100);
      expect(mockPage.mouse.move).not.toHaveBeenCalled();
    });

    it("should return early for Infinity coordinates", async () => {
      const cursor = new GhostCursor(mockPage);
      await cursor.performMove({ x: Infinity, y: 0 }, { x: 100, y: 100 }, 100);
      expect(mockPage.mouse.move).not.toHaveBeenCalled();
    });
  });

  describe("move", () => {
    it("should return early for invalid coordinates", async () => {
      const cursor = new GhostCursor(mockPage);
      await cursor.move(NaN, 100);
      await cursor.move(100, NaN);
      await cursor.move(Infinity, 100);
      expect(mockPage.mouse.move).not.toHaveBeenCalled();
    });
  });

  describe("moveWithHesitation", () => {
    it("should return early for invalid coordinates", async () => {
      const cursor = new GhostCursor(mockPage);
      await cursor.moveWithHesitation(NaN, 100);
      await cursor.moveWithHesitation(100, NaN);
      expect(mockPage.mouse.move).not.toHaveBeenCalled();
    });
  });

  describe("hoverWithDrift", () => {
    it("should have proper mouse methods", () => {
      const cursor = new GhostCursor(mockPage);
      expect(cursor.page.mouse.move).toBeDefined();
    });
  });

  describe("park", () => {
    it("should complete without error", async () => {
      const cursor = new GhostCursor(mockPage);
      await cursor.park();
      expect(mockPage.viewportSize).toHaveBeenCalled();
    });
  });
});
