/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { GhostCursor } from "@api/utils/ghostCursor.js";
import { TWITTER_CLICK_PROFILES } from "@api/constants/engagement.js";
import { mathUtils } from "@api/utils/math.js";

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  }),
}));

vi.mock("@api/interactions/queries.js", () => ({
  visible: vi.fn().mockResolvedValue(true),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn((min, max) => (min + max) / 2),
    gaussian: vi.fn((mean) => mean),
    roll: vi.fn(() => false),
  },
}));

describe("api/utils/ghostCursor.js", () => {
  let mockPage;
  let ghostCursor;

  beforeEach(() => {
    vi.clearAllMocks();

    mockPage = {
      mouse: {
        move: vi.fn().mockResolvedValue(),
        down: vi.fn().mockResolvedValue(),
        up: vi.fn().mockResolvedValue(),
      },
      locator: vi.fn().mockReturnValue({
        boundingBox: vi
          .fn()
          .mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
        isVisible: true,
        click: vi.fn().mockResolvedValue(),
      }),
      viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
    };

    ghostCursor = new GhostCursor(mockPage);
  });

  describe("GhostCursor constructor", () => {
    it("should create GhostCursor instance with page", () => {
      expect(ghostCursor.page).toBe(mockPage);
    });

    it("should initialize previousPos", () => {
      expect(ghostCursor.previousPos).toBeDefined();
      expect(ghostCursor.previousPos.x).toBeDefined();
      expect(ghostCursor.previousPos.y).toBeDefined();
    });

    it("should use provided logger", () => {
      const customLogger = { info: vi.fn() };
      const gc = new GhostCursor(mockPage, customLogger);
      expect(gc.logger).toBe(customLogger);
    });

    it("should initialize current position via init()", () => {
      expect(typeof ghostCursor.init).toBe("function");
    });
  });

  describe("TWITTER_CLICK_PROFILES", () => {
    it("should have like profile", () => {
      expect(TWITTER_CLICK_PROFILES.like).toBeDefined();
      expect(TWITTER_CLICK_PROFILES.like.hoverMin).toBe(800);
      expect(TWITTER_CLICK_PROFILES.like.hoverMax).toBe(2000);
    });

    it("should have reply profile", () => {
      expect(TWITTER_CLICK_PROFILES.reply).toBeDefined();
      expect(TWITTER_CLICK_PROFILES.reply.hoverMin).toBe(1500);
    });

    it("should have retweet profile", () => {
      expect(TWITTER_CLICK_PROFILES.retweet).toBeDefined();
    });

    it("should have follow profile", () => {
      expect(TWITTER_CLICK_PROFILES.follow).toBeDefined();
    });

    it("should have bookmark profile", () => {
      expect(TWITTER_CLICK_PROFILES.bookmark).toBeDefined();
    });

    it("should have nav profile", () => {
      expect(TWITTER_CLICK_PROFILES.nav).toBeDefined();
    });
  });

  describe("vector helpers", () => {
    it("should add vectors correctly", () => {
      const result = ghostCursor.vecAdd({ x: 1, y: 2 }, { x: 3, y: 4 });
      expect(result).toEqual({ x: 4, y: 6 });
    });

    it("should subtract vectors correctly", () => {
      const result = ghostCursor.vecSub({ x: 3, y: 4 }, { x: 1, y: 2 });
      expect(result).toEqual({ x: 2, y: 2 });
    });

    it("should multiply vector by scalar", () => {
      const result = ghostCursor.vecMult({ x: 2, y: 3 }, 2);
      expect(result).toEqual({ x: 4, y: 6 });
    });

    it("should calculate vector length", () => {
      const result = ghostCursor.vecLen({ x: 3, y: 4 });
      expect(result).toBe(5);
    });
  });

  describe("bezier", () => {
    it("should calculate bezier point at t=0", () => {
      const result = ghostCursor.bezier(
        0,
        { x: 0, y: 0 },
        { x: 1, y: 1 },
        { x: 2, y: 2 },
        { x: 3, y: 3 },
      );
      expect(result.x).toBeCloseTo(0);
      expect(result.y).toBeCloseTo(0);
    });

    it("should calculate bezier point at t=1", () => {
      const result = ghostCursor.bezier(
        1,
        { x: 0, y: 0 },
        { x: 1, y: 1 },
        { x: 2, y: 2 },
        { x: 3, y: 3 },
      );
      expect(result.x).toBeCloseTo(3);
      expect(result.y).toBeCloseTo(3);
    });

    it("should calculate bezier point at t=0.5", () => {
      const result = ghostCursor.bezier(
        0.5,
        { x: 0, y: 0 },
        { x: 1, y: 1 },
        { x: 2, y: 2 },
        { x: 3, y: 3 },
      );
      expect(result.x).toBeCloseTo(1.5);
      expect(result.y).toBeCloseTo(1.5);
    });
  });

  describe("easeOutCubic", () => {
    it("should return 0 at t=0", () => {
      expect(ghostCursor.easeOutCubic(0)).toBe(0);
    });

    it("should return 1 at t=1", () => {
      expect(ghostCursor.easeOutCubic(1)).toBe(1);
    });

    it("should be between 0 and 1 for t in (0,1)", () => {
      const result = ghostCursor.easeOutCubic(0.5);
      expect(result).toBeGreaterThan(0);
      expect(result).toBeLessThan(1);
    });
  });

  describe("performMove", () => {
    it("should handle invalid start position", async () => {
      await expect(
        ghostCursor.performMove(null, { x: 100, y: 100 }, 100),
      ).resolves.toBeUndefined();
    });

    it("should handle invalid end position", async () => {
      await expect(
        ghostCursor.performMove({ x: 0, y: 0 }, null, 100),
      ).resolves.toBeUndefined();
    });

    it("should handle NaN coordinates", async () => {
      await expect(
        ghostCursor.performMove({ x: NaN, y: 0 }, { x: 100, y: 100 }, 100),
      ).resolves.toBeUndefined();
    });

    it("should handle Infinity coordinates", async () => {
      await expect(
        ghostCursor.performMove({ x: Infinity, y: 0 }, { x: 100, y: 100 }, 100),
      ).resolves.toBeUndefined();
    });
  });

  describe("move", () => {
    it("should handle invalid target coordinates", async () => {
      await expect(ghostCursor.move(NaN, 100)).resolves.toBeUndefined();
      await expect(ghostCursor.move(100, NaN)).resolves.toBeUndefined();
      await expect(ghostCursor.move(Infinity, 100)).resolves.toBeUndefined();
    });

    it("should perform a direct move for a short distance", async () => {
      ghostCursor.previousPos = { x: 10, y: 10 };
      const performSpy = vi.spyOn(ghostCursor, "performMove");

      await ghostCursor.move(20, 20);

      expect(performSpy).toHaveBeenCalledWith(
        { x: 10, y: 10 },
        { x: 20, y: 20 },
        expect.any(Number),
      );
    });

    it("should take the overshoot path for long distances when rolling true", async () => {
      ghostCursor.previousPos = { x: 0, y: 0 };
      mathUtils.roll.mockReturnValue(true);
      const performSpy = vi
        .spyOn(ghostCursor, "performMove")
        .mockResolvedValue();

      await ghostCursor.move(1000, 1000);

      expect(performSpy).toHaveBeenCalledTimes(2);
    });
  });

  describe("moveWithHesitation", () => {
    it("should handle NaN coordinates", async () => {
      await expect(
        ghostCursor.moveWithHesitation(NaN, 100),
      ).resolves.toBeUndefined();
    });

    it("should fall back to move for short distances", async () => {
      ghostCursor.previousPos = { x: 0, y: 0 };
      const moveSpy = vi.spyOn(ghostCursor, "move").mockResolvedValue();

      await ghostCursor.moveWithHesitation(10, 10);

      expect(moveSpy).toHaveBeenCalledWith(10, 10);
    });
  });

  describe("park", () => {
    it("should handle missing viewport", async () => {
      mockPage.viewportSize.mockReturnValue(null);
      await expect(ghostCursor.park()).resolves.toBeUndefined();
    });

    it("should park on the left when roll returns true", async () => {
      mathUtils.roll.mockReturnValue(true);
      const performSpy = vi
        .spyOn(ghostCursor, "performMove")
        .mockResolvedValue();

      await ghostCursor.park();

      expect(performSpy).toHaveBeenCalled();
    });
  });

  describe("click", () => {
    it("should return success false when no bounding box and no fallback", async () => {
      const mockLocator = {
        boundingBox: vi.fn().mockResolvedValue(null),
      };

      const result = await ghostCursor.click(mockLocator, {
        allowNativeFallback: false,
      });
      expect(result.success).toBe(false);
    });

    it("should use native fallback when allowed and no bbox", async () => {
      const mockLocator = {
        boundingBox: vi.fn().mockResolvedValue(null),
        click: vi.fn().mockResolvedValue(),
      };

      const result = await ghostCursor.click(mockLocator, {
        allowNativeFallback: true,
      });
      expect(result.usedFallback).toBe(true);
    });

    it("should click with hover before click when enabled", async () => {
      const mockLocator = {
        boundingBox: vi
          .fn()
          .mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
        click: vi.fn(),
      };
      const hoverSpy = vi
        .spyOn(ghostCursor, "hoverWithDrift")
        .mockResolvedValue();
      const moveSpy = vi.spyOn(mockPage.mouse, "move").mockResolvedValue();
      const downSpy = vi.spyOn(mockPage.mouse, "down").mockResolvedValue();
      const upSpy = vi.spyOn(mockPage.mouse, "up").mockResolvedValue();

      const result = await ghostCursor.click(mockLocator, {
        hoverBeforeClick: true,
        allowNativeFallback: false,
      });

      expect(hoverSpy).toHaveBeenCalled();
      expect(moveSpy).toHaveBeenCalled();
      expect(downSpy).toHaveBeenCalled();
      expect(upSpy).toHaveBeenCalled();
      expect(result.success).toBe(true);
    });

    it("should fall back to native click after tracking failure", async () => {
      const mockLocator = {
        boundingBox: vi.fn().mockResolvedValue(null),
        click: vi.fn().mockResolvedValue(),
      };

      const result = await ghostCursor.click(mockLocator, {
        allowNativeFallback: true,
        label: "fallback",
      });

      expect(result.usedFallback).toBe(true);
      expect(mockLocator.click).toHaveBeenCalledWith({
        force: true,
        button: "left",
      });
    });
  });

  describe("twitterClick", () => {
    it("should fall back to nav profile when action type is unknown", async () => {
      const profileSpy = vi
        .spyOn(ghostCursor, "profiledClick")
        .mockResolvedValue();
      const mockLocator = {
        boundingBox: vi.fn().mockResolvedValue(null),
        click: vi.fn().mockResolvedValue(),
      };

      await ghostCursor.twitterClick(mockLocator, "unknown-action", 1);

      expect(profileSpy).toHaveBeenCalledWith(
        mockLocator,
        TWITTER_CLICK_PROFILES.nav,
        1,
      );
    });

    it("should use like profile when action type is like", async () => {
      const profileSpy = vi
        .spyOn(ghostCursor, "profiledClick")
        .mockResolvedValue();
      const mockLocator = {
        boundingBox: vi
          .fn()
          .mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
      };

      await ghostCursor.twitterClick(mockLocator, "like", 2);

      expect(profileSpy).toHaveBeenCalledWith(
        mockLocator,
        TWITTER_CLICK_PROFILES.like,
        2,
      );
    });

    it("should use retweet profile when action type is retweet", async () => {
      const profileSpy = vi
        .spyOn(ghostCursor, "profiledClick")
        .mockResolvedValue();
      const mockLocator = {
        boundingBox: vi
          .fn()
          .mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
      };

      await ghostCursor.twitterClick(mockLocator, "retweet", 1);

      expect(profileSpy).toHaveBeenCalledWith(
        mockLocator,
        TWITTER_CLICK_PROFILES.retweet,
        1,
      );
    });

    it("should use follow profile when action type is follow", async () => {
      const profileSpy = vi
        .spyOn(ghostCursor, "profiledClick")
        .mockResolvedValue();
      const mockLocator = {
        boundingBox: vi
          .fn()
          .mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
      };

      await ghostCursor.twitterClick(mockLocator, "follow", 3);

      expect(profileSpy).toHaveBeenCalledWith(
        mockLocator,
        TWITTER_CLICK_PROFILES.follow,
        3,
      );
    });
  });

  describe("hoverWithDrift", () => {
    it("should perform hover for specified duration", async () => {
      const mockMove = vi.spyOn(mockPage.mouse, "move").mockResolvedValue();
      const startTime = Date.now();

      await ghostCursor.hoverWithDrift(100, 100, 50, 50);

      const duration = Date.now() - startTime;
      expect(duration).toBeGreaterThanOrEqual(40);
      expect(mockMove).toHaveBeenCalled();
    });

    it("should handle invalid coordinates", async () => {
      await expect(
        ghostCursor.hoverWithDrift(NaN, 100, 50, 50),
      ).resolves.toBeUndefined();
      await expect(
        ghostCursor.hoverWithDrift(100, NaN, 50, 50),
      ).resolves.toBeUndefined();
    });
  });

  describe("waitForStableElement", () => {
    it("should return null when locator returns null", async () => {
      const mockLocator = {
        boundingBox: vi.fn().mockResolvedValue(null),
      };

      const result = await ghostCursor.waitForStableElement(mockLocator, 1000);
      expect(result).toBeNull();
    });

    it("should return bbox when element is stable", async () => {
      const stableBox = { x: 100, y: 100, width: 50, height: 50 };
      const mockLocator = {
        boundingBox: vi.fn().mockResolvedValue(stableBox),
      };

      const result = await ghostCursor.waitForStableElement(mockLocator, 500);
      expect(result).toEqual(stableBox);
    });

    it("should return last bbox when element never stabilizes", async () => {
      let callCount = 0;
      const mockLocator = {
        boundingBox: vi.fn().mockImplementation(() => {
          callCount++;
          return { x: callCount * 10, y: 0, width: 50, height: 50 };
        }),
      };

      const result = await ghostCursor.waitForStableElement(mockLocator, 200);
      expect(result).not.toBeNull();
      expect(result.width).toBe(50);
    });
  });

  describe("profiledClick", () => {
    it("should call waitForStableElement", async () => {
      const waitSpy = vi
        .spyOn(ghostCursor, "waitForStableElement")
        .mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 });
      const moveSpy = vi
        .spyOn(ghostCursor, "moveWithHesitation")
        .mockResolvedValue();
      const hoverSpy = vi
        .spyOn(ghostCursor, "hoverWithDrift")
        .mockResolvedValue();

      const mockLocator = {
        boundingBox: vi
          .fn()
          .mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
        click: vi.fn().mockResolvedValue(),
      };

      await ghostCursor.profiledClick(
        mockLocator,
        {
          hoverMin: 100,
          hoverMax: 200,
          holdMs: 50,
          hesitation: false,
          microMove: false,
        },
        1,
      );

      expect(waitSpy).toHaveBeenCalled();
    });

    it("should handle null bbox and use fallback", async () => {
      const waitSpy = vi
        .spyOn(ghostCursor, "waitForStableElement")
        .mockResolvedValue(null);
      const moveSpy = vi
        .spyOn(ghostCursor, "moveWithHesitation")
        .mockResolvedValue();

      const mockLocator = {
        boundingBox: vi.fn().mockResolvedValue(null),
        click: vi.fn().mockResolvedValue(),
      };

      await ghostCursor.profiledClick(
        mockLocator,
        { hoverMin: 100, hoverMax: 200, holdMs: 50 },
        0,
      );

      expect(waitSpy).toHaveBeenCalled();
    });
  });

  describe("park", () => {
    it("should park on the right when roll returns false", async () => {
      mathUtils.roll.mockReturnValue(false);
      const performSpy = vi
        .spyOn(ghostCursor, "performMove")
        .mockResolvedValue();

      await ghostCursor.park();

      expect(performSpy).toHaveBeenCalled();
    });
  });
});
