/**
 * @fileoverview Comprehensive tests for wait module
 */

import { describe, it, expect, vi } from "vitest";

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(() => mockPage),
}));

vi.mock("@api/utils/locator.js", () => ({
  getLocator: vi.fn((selector) => mockPage.locator(selector)),
}));

const mockPage = {
  locator: vi.fn(() => ({
    waitFor: vi.fn().mockResolvedValue(undefined),
    isVisible: vi.fn().mockResolvedValue(true),
    isHidden: vi.fn().mockResolvedValue(false),
  })),
};

describe("Wait Module", () => {
  describe("wait function", () => {
    it("should be exported", async () => {
      const { wait } = await import("../../../interactions/wait.js");
      expect(typeof wait).toBe("function");
    });

    it("should throw ValidationError for non-number input", async () => {
      const { wait } = await import("../../../interactions/wait.js");
      await expect(wait("invalid")).rejects.toThrow();
    });

    it("should throw ValidationError for negative number", async () => {
      const { wait } = await import("../../../interactions/wait.js");
      await expect(wait(-100)).rejects.toThrow();
    });

    it("should resolve for valid positive number", async () => {
      const { wait } = await import("../../../interactions/wait.js");
      await expect(wait(1)).resolves.toBeUndefined();
    });
  });

  describe("waitWithAbort function", () => {
    it("should be exported", async () => {
      const { waitWithAbort } = await import("../../../interactions/wait.js");
      expect(typeof waitWithAbort).toBe("function");
    });

    it("should throw ValidationError for non-number input", async () => {
      const { waitWithAbort } = await import("../../../interactions/wait.js");
      await expect(waitWithAbort("invalid")).rejects.toThrow();
    });

    it("should resolve for valid positive number", async () => {
      const { waitWithAbort } = await import("../../../interactions/wait.js");
      await expect(waitWithAbort(1)).resolves.toBeUndefined();
    });

    it("should reject when signal is already aborted", async () => {
      const { waitWithAbort } = await import("../../../interactions/wait.js");
      const controller = new AbortController();
      controller.abort();
      await expect(waitWithAbort(1000, controller.signal)).rejects.toThrow();
    });
  });

  describe("waitFor function", () => {
    it("should be exported", async () => {
      const { waitFor } = await import("../../../interactions/wait.js");
      expect(typeof waitFor).toBe("function");
    });
  });

  describe("waitVisible function", () => {
    it("should be exported", async () => {
      const { waitVisible } = await import("../../../interactions/wait.js");
      expect(typeof waitVisible).toBe("function");
    });

    it("should throw ValidationError for empty selector", async () => {
      const { waitVisible } = await import("../../../interactions/wait.js");
      await expect(waitVisible("")).rejects.toThrow();
    });
  });

  describe("waitHidden function", () => {
    it("should be exported", async () => {
      const { waitHidden } = await import("../../../interactions/wait.js");
      expect(typeof waitHidden).toBe("function");
    });
  });

  describe("waitForLoadState function", () => {
    it("should be exported", async () => {
      const { waitForLoadState } =
        await import("../../../interactions/wait.js");
      expect(typeof waitForLoadState).toBe("function");
    });
  });

  describe("waitForURL function", () => {
    it("should be exported", async () => {
      const { waitForURL } = await import("../../../interactions/wait.js");
      expect(typeof waitForURL).toBe("function");
    });
  });
});

describe("ClickAt Module", () => {
  it("should export a function", async () => {
    const mod = await import("../../../interactions/clickAt.js");
    expect(mod.default || mod.clickAt).toBeDefined();
  });
});

describe("Drag Module", () => {
  it("should export a function", async () => {
    const mod = await import("../../../interactions/drag.js");
    expect(mod.default || mod.drag).toBeDefined();
  });
});

describe("Scroll Module", () => {
  it("should export scroll functions", async () => {
    const mod = await import("../../../interactions/scroll.js");
    expect(Object.keys(mod).length).toBeGreaterThan(0);
  });
});

describe("Keys Module", () => {
  it("should export key functions", async () => {
    const mod = await import("../../../interactions/keys.js");
    expect(Object.keys(mod).length).toBeGreaterThan(0);
  });
});

describe("MultiSelect Module", () => {
  it("should export a function", async () => {
    const mod = await import("../../../interactions/multiSelect.js");
    expect(mod.default || mod.multiSelect).toBeDefined();
  });
});

describe("Navigation Module", () => {
  it("should export navigation functions", async () => {
    const mod = await import("../../../interactions/navigation.js");
    expect(Object.keys(mod).length).toBeGreaterThan(0);
  });
});

describe("Queries Module", () => {
  it("should export query functions", async () => {
    const mod = await import("../../../interactions/queries.js");
    expect(Object.keys(mod).length).toBeGreaterThan(0);
  });
});

describe("Banners Module", () => {
  it("should export banner functions", async () => {
    const mod = await import("../../../interactions/banners.js");
    expect(Object.keys(mod).length).toBeGreaterThan(0);
  });
});

describe("Game Modules", () => {
  it("game-units should export functions", async () => {
    const mod = await import("../../../interactions/game-units.js");
    expect(Object.keys(mod).length).toBeGreaterThan(0);
  });

  it("gameMenus should export functions", async () => {
    const mod = await import("../../../interactions/gameMenus.js");
    expect(Object.keys(mod).length).toBeGreaterThan(0);
  });

  it("gameState should export functions", async () => {
    const mod = await import("../../../interactions/gameState.js");
    expect(Object.keys(mod).length).toBeGreaterThan(0);
  });

  it("resourceTracker should export functions", async () => {
    const mod = await import("../../../interactions/resourceTracker.js");
    expect(Object.keys(mod).length).toBeGreaterThan(0);
  });
});

describe("Actions Module", () => {
  it("should export action functions", async () => {
    const mod = await import("../../../interactions/actions.js");
    expect(Object.keys(mod).length).toBeGreaterThan(0);
  });
});

describe("Cursor Module", () => {
  it("should export cursor functions", async () => {
    const mod = await import("../../../interactions/cursor.js");
    expect(Object.keys(mod).length).toBeGreaterThan(0);
  });
});
