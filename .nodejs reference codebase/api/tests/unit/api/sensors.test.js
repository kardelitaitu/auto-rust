/**
 * Auto-AI Framework - Sensors Tests
 * @module tests/unit/api/sensors.test
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

const mockPage = {
  addInitScript: vi.fn().mockResolvedValue(undefined),
};

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(() => mockPage),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    gaussian: vi.fn(() => 0.85),
    randomInRange: vi.fn(() => 50),
  },
}));

describe("api/utils/sensors.js", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
  });

  describe("injectSensors", () => {
    it("should call addInitScript on page", async () => {
      const { injectSensors } = await import("../../../utils/sensors.js");
      await injectSensors();
      expect(mockPage.addInitScript).toHaveBeenCalled();
    });

    it("should call addInitScript exactly once", async () => {
      const { injectSensors } = await import("../../../utils/sensors.js");
      await injectSensors();
      expect(mockPage.addInitScript).toHaveBeenCalledTimes(1);
    });

    it("should call addInitScript with function", async () => {
      const { injectSensors } = await import("../../../utils/sensors.js");
      await injectSensors();
      const call = mockPage.addInitScript.mock.calls[0];
      expect(typeof call[0]).toBe("function");
    });
  });
});
