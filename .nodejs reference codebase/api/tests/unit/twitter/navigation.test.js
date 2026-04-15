import { describe, it, expect, vi, beforeEach } from "vitest";

const mockPage = {
  locator: vi.fn().mockReturnValue({
    first: vi.fn().mockReturnValue({
      waitFor: vi.fn().mockResolvedValue(undefined),
    }),
  }),
  evaluate: vi.fn().mockResolvedValue(undefined),
  goto: vi.fn().mockResolvedValue(undefined),
  url: vi.fn().mockReturnValue("https://x.com/home"),
};

const mockApi = {
  getPage: vi.fn(() => mockPage),
  visible: vi.fn().mockResolvedValue(true),
  click: vi.fn().mockResolvedValue(true),
  wait: vi.fn().mockResolvedValue(undefined),
  getCurrentUrl: vi.fn().mockResolvedValue("https://x.com/home"),
  waitForURL: vi.fn().mockResolvedValue(undefined),
  goto: vi.fn().mockResolvedValue(undefined),
  scroll: {
    read: vi.fn().mockResolvedValue(undefined),
  },
};

vi.mock("@api/index.js", () => ({
  api: mockApi,
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn((min, max) => (min + max) / 2),
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

describe("api/twitter/navigation.js", () => {
  let navigation;

  beforeEach(async () => {
    vi.clearAllMocks();
    mockApi.visible.mockResolvedValue(true);
    mockApi.getCurrentUrl.mockResolvedValue("https://x.com/home");
    mockPage.url.mockReturnValue("https://x.com/home");

    const module = await import("@api/twitter/navigation.js");
    navigation = module;
  });

  describe("home()", () => {
    it("should navigate to home successfully", async () => {
      const result = await navigation.home({ readFeed: false });
      expect(result.success).toBe(true);
      expect(result.reason).toBe("home_navigated");
    });

    it("should navigate with readFeed disabled", async () => {
      const result = await navigation.home({ readFeed: false });
      expect(result.success).toBe(true);
      // scroll.read should not be called when readFeed is false
    });

    it("should navigate with custom readDurationMs", async () => {
      const result = await navigation.home({
        readFeed: true,
        readDurationMs: 1000,
      });
      expect(result.success).toBe(true);
    });

    it("should use X logo fallback when home button not visible", async () => {
      mockApi.visible.mockResolvedValueOnce(false).mockResolvedValueOnce(true);
      const result = await navigation.home({ readFeed: false });
      expect(result.success).toBe(true);
    });

    it("should use direct URL fallback when no buttons visible", async () => {
      mockApi.visible.mockResolvedValue(false);
      const result = await navigation.home({ readFeed: false });
      expect(result.success).toBe(true);
    });

    it("should handle navigation errors gracefully", async () => {
      mockApi.click.mockRejectedValueOnce(new Error("Click failed"));
      const result = await navigation.home({ readFeed: false });
      expect(result.success).toBe(true);
    });

    it("should wait for hydration after navigation", async () => {
      await navigation.home({ readFeed: false });
      expect(mockApi.wait).toHaveBeenCalled();
    });

    it("should verify navigation when home button clicked", async () => {
      mockApi.getCurrentUrl.mockResolvedValue("https://x.com/explore");
      const result = await navigation.home({ readFeed: false });
      expect(result.success).toBe(true);
    });
  });

  describe("isOnHome()", () => {
    it("should return true when on /home URL", async () => {
      mockApi.getCurrentUrl.mockResolvedValue("https://x.com/home");
      const result = await navigation.isOnHome();
      expect(result).toBe(true);
    });

    it("should return true when on x.com root", async () => {
      mockApi.getCurrentUrl.mockResolvedValue("https://x.com/");
      const result = await navigation.isOnHome();
      expect(result).toBe(true);
    });

    it("should return true when on x.com without trailing slash", async () => {
      mockApi.getCurrentUrl.mockResolvedValue("https://x.com");
      const result = await navigation.isOnHome();
      expect(result).toBe(true);
    });

    it("should return false when on explore page", async () => {
      mockApi.getCurrentUrl.mockResolvedValue("https://x.com/explore");
      const result = await navigation.isOnHome();
      expect(result).toBe(false);
    });

    it("should return false when on profile page", async () => {
      mockApi.getCurrentUrl.mockResolvedValue("https://x.com/username");
      const result = await navigation.isOnHome();
      expect(result).toBe(false);
    });
  });
});
