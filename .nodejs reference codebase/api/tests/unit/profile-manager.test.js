/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import fs from "fs";
import { execSync } from "child_process";
let profileManager;

// Mock dependencies
vi.mock("fs");
vi.mock("child_process", () => ({
  exec: vi.fn((cmd, opts, callback) =>
    callback(null, { stdout: "", stderr: "" }),
  ),
  execSync: vi.fn(),
}));

describe("profileManager", () => {
  const mockProfiles = [
    {
      id: "fast",
      type: "engagement",
      timings: { scrollPause: { mean: 2000 } },
      probabilities: { dive: 50, like: 30 },
    },
    {
      id: "slow",
      type: "engagement",
      timings: { scrollPause: { mean: 5000 } },
      probabilities: { dive: 50, like: 30 },
    },
  ];

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.resetModules();
    vi.mocked(fs.existsSync).mockReturnValue(true);
    vi.mocked(fs.readFileSync).mockReturnValue(JSON.stringify(mockProfiles));
    const module = await import("../../utils/profileManager.js");
    profileManager = module.profileManager;
    profileManager.reset();
    profileManager.reload();
  });

  const clearProfiles = () => {
    vi.mocked(fs.existsSync).mockReturnValue(true);
    vi.mocked(fs.readFileSync).mockReturnValue("[]");
    profileManager.reload();
  };

  describe("loadProfiles", () => {
    it("should load profiles from disk", () => {
      const result = profileManager.reload();
      expect(result).toBe(true);
      expect(profileManager.getCount()).toBe(2);
    });

    it("should return false if file does not exist", () => {
      vi.mocked(fs.existsSync).mockReturnValue(false);
      const result = profileManager.reload();
      expect(result).toBe(false);
    });

    it("should return false and log error on invalid JSON", () => {
      vi.mocked(fs.readFileSync).mockReturnValue("invalid-json");
      const consoleSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});
      const result = profileManager.reload();
      expect(result).toBe(false);
      expect(consoleSpy).toHaveBeenCalled();
      consoleSpy.mockRestore();
    });
  });

  describe("Selection Logic (Sync)", () => {
    it("should get a starter profile", () => {
      const profile = profileManager.getStarter();
      expect(profile).toBeDefined();
      expect(["fast", "slow"]).toContain(profile.id);
    });

    it("should get a fatigued variant if available", () => {
      const profile = profileManager.getFatiguedVariant(2000);
      expect(profile.id).toBe("slow");
    });

    it("should return null if no fatigued variant found", () => {
      const profile = profileManager.getFatiguedVariant(10000);
      expect(profile).toBeNull();
    });

    it("should return null if profiles cannot be loaded during fatigue check", () => {
      clearProfiles();
      vi.mocked(fs.existsSync).mockReturnValue(false);

      const result = profileManager.getFatiguedVariant(2000);
      expect(result).toBeNull();
    });

    it("should get profile by ID", () => {
      const profile = profileManager.getById("fast");
      expect(profile.id).toBe("fast");
    });

    it("should throw error for non-existent ID", () => {
      expect(() => profileManager.getById("missing")).toThrow(
        'Profile "missing" not found',
      );
    });

    it("should throw error if no profiles loaded", () => {
      clearProfiles();
      vi.mocked(fs.existsSync).mockReturnValue(false);
      expect(() => profileManager.getStarter()).toThrow();
    });
  });

  describe("Selection Logic (Async)", () => {
    it("should get a starter profile async", async () => {
      const profile = await profileManager.getStarterAsync();
      expect(profile).toBeDefined();
    });

    it("should get profile by ID async", async () => {
      const profile = await profileManager.getByIdAsync("slow");
      expect(profile.id).toBe("slow");
    });

    it("should throw error for non-existent ID async", async () => {
      await expect(profileManager.getByIdAsync("none")).rejects.toThrow(
        'Profile "none" not found',
      );
    });

    it("should reload async", async () => {
      const result = await profileManager.reloadAsync();
      expect(result).toBe(true);
    });

    it("should throw error in getStarterAsync if load fails", async () => {
      clearProfiles();
      vi.mocked(fs.existsSync).mockReturnValue(false);
      vi.mocked(execSync).mockImplementationOnce(() => {
        throw new Error("fail");
      });

      await expect(profileManager.getStarterAsync()).rejects.toThrow();
    });

    it("should throw error in getByIdAsync if load fails", async () => {
      clearProfiles();
      vi.mocked(fs.existsSync).mockReturnValue(false);
      vi.mocked(execSync).mockImplementationOnce(() => {
        throw new Error("fail");
      });

      await expect(profileManager.getByIdAsync("id")).rejects.toThrow();
    });
  });

  describe("Generation Fallback", () => {
    it("should trigger generation if loading fails", async () => {
      clearProfiles();
      // 1. sync check fails (empty)
      // 2. ensureProfilesLoadedAsync -> loadProfiles() fails (mock exists false once)
      // 3. generateProfilesAsync() called -> exec()
      // 4. retry loadProfiles() -> success (mock exists true, read mockProfiles)
      vi.mocked(fs.existsSync).mockReturnValueOnce(false).mockReturnValue(true);
      vi.mocked(fs.readFileSync).mockReturnValue(JSON.stringify(mockProfiles));

      const profile = await profileManager.getStarterAsync();
      expect(profile).toBeDefined();
      expect(execSync).toHaveBeenCalled();
    });

    it("should return false if generation fails", async () => {
      clearProfiles();
      vi.mocked(fs.existsSync).mockReturnValue(false);
      vi.mocked(execSync).mockImplementationOnce(() => {
        throw new Error("Exec failed");
      });

      // getStarterAsync throws if ensureProfilesLoaded returns false (which it does if generation fails)
      await expect(profileManager.getStarterAsync()).rejects.toThrow();
    });
  });

  describe("Validation", () => {
    it("should warn on invalid profiles during reload", () => {
      const badProfiles = [{ id: "bad" }];
      vi.mocked(fs.readFileSync).mockReturnValue(JSON.stringify(badProfiles));
      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

      profileManager.reload();
      expect(consoleSpy).toHaveBeenCalledWith(
        expect.stringContaining("Profile validation issues"),
      );
      consoleSpy.mockRestore();
    });

    it("should handle validation failures with specific errors", () => {
      const badProfiles = [
        {
          id: "bad",
          type: "e",
          timings: { scrollPause: {} },
          probabilities: { dive: 150 },
        },
      ];
      vi.mocked(fs.readFileSync).mockReturnValue(JSON.stringify(badProfiles));
      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

      profileManager.reload();
      expect(consoleSpy).toHaveBeenCalledWith(
        expect.stringContaining("Profile validation issues"),
      );
      consoleSpy.mockRestore();
    });

    it("should handle null profile input in validation", () => {
      vi.mocked(fs.readFileSync).mockReturnValue("[null]");
      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
      profileManager.reload();
      expect(consoleSpy).toHaveBeenCalled();
      consoleSpy.mockRestore();
    });
  });

  describe("Edge Cases & 100% Coverage", () => {
    it("should throw if ensureProfilesLoaded fails in getById", () => {
      clearProfiles();
      vi.mocked(fs.existsSync).mockReturnValue(false);
      expect(() => profileManager.getById("any")).toThrow("No profiles loaded");
    });
  });
});
