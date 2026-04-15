/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { readline } from "@api/utils/file.readline.js";

vi.mock("fs/promises", () => ({
  default: {
    readFile: vi.fn(),
  },
}));

import fs from "fs/promises";

describe("api/utils/file.readline.js", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("readline", () => {
    it("should return a random line from file", async () => {
      const fileContent = "line1\nline2\nline3";
      fs.readFile.mockResolvedValue(fileContent);

      const result = await readline("test.txt");

      expect(result).toMatch(/line[123]/);
    });

    it("should filter out empty lines", async () => {
      const fileContent = "line1\n\nline2\n\n\nline3";
      fs.readFile.mockResolvedValue(fileContent);

      const result = await readline("test.txt");

      expect(result).toMatch(/line[123]/);
      expect(result).not.toBe("");
    });

    it("should return null for empty file", async () => {
      const fileContent = "";
      fs.readFile.mockResolvedValue(fileContent);

      const result = await readline("test.txt");

      expect(result).toBeNull();
    });

    it("should return null for file with only whitespace", async () => {
      const fileContent = "   \n\n   \n";
      fs.readFile.mockResolvedValue(fileContent);

      const result = await readline("test.txt");

      expect(result).toBeNull();
    });

    it("should return null when file not found", async () => {
      const error = new Error("File not found");
      error.code = "ENOENT";
      fs.readFile.mockRejectedValue(error);

      const result = await readline("nonexistent.txt");

      expect(result).toBeNull();
    });

    it("should return null on other errors", async () => {
      fs.readFile.mockRejectedValue(new Error("Permission denied"));

      const result = await readline("test.txt");

      expect(result).toBeNull();
    });

    it("should handle Windows line endings", async () => {
      const fileContent = "line1\r\nline2\r\nline3";
      fs.readFile.mockResolvedValue(fileContent);

      const result = await readline("test.txt");

      expect(result).toMatch(/line[123]/);
    });

    it("should handle single line file", async () => {
      const fileContent = "single-line";
      fs.readFile.mockResolvedValue(fileContent);

      const result = await readline("test.txt");

      expect(result).toBe("single-line");
    });

    it("should return null for file with only newlines", async () => {
      const fileContent = "\n\n\n";
      fs.readFile.mockResolvedValue(fileContent);

      const result = await readline("test.txt");

      expect(result).toBeNull();
    });
  });
});
