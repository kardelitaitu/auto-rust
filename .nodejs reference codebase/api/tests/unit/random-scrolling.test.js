/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { createRandomScroller } from "@api/utils/randomScrolling.js";

vi.mock("@api/index.js", () => ({
  api: {
    scroll: {
      read: vi.fn().mockResolvedValue(undefined),
    },
    wait: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn().mockImplementation((min, max) => (min + max) / 2),
  },
}));

import { api } from "@api/index.js";

describe("api/utils/randomScrolling.js", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should create a random scroller function", () => {
    const scroller = createRandomScroller({});
    expect(typeof scroller).toBe("function");
  });

  it("should call api.scroll.read when scroller is invoked", async () => {
    const scroller = createRandomScroller({});
    await scroller(0.1); // very short duration

    expect(api.scroll.read).toHaveBeenCalled();
  });

  it("should call api.wait between scroll cycles", async () => {
    const scroller = createRandomScroller({});
    await scroller(0.1);

    expect(api.wait).toHaveBeenCalled();
  });

  it("should pass scroll options to api.scroll.read", async () => {
    const scroller = createRandomScroller({});
    await scroller(0.1);

    expect(api.scroll.read).toHaveBeenCalledWith(
      undefined,
      expect.objectContaining({
        pauses: 1,
        variableSpeed: true,
        backScroll: expect.any(Boolean),
      }),
    );
  });

  it("should use scrollAmount from randomInRange", async () => {
    const { mathUtils } = await import("@api/utils/math.js");
    mathUtils.randomInRange.mockReturnValueOnce(500).mockReturnValueOnce(1000);

    const scroller = createRandomScroller({});
    await scroller(0.1);

    expect(api.scroll.read).toHaveBeenCalledWith(
      undefined,
      expect.objectContaining({
        scrollAmount: 500,
      }),
    );
  });
});
