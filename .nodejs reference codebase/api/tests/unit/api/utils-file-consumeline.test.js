/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { consumeline } from "@api/utils/file.consumeline.js";

vi.mock("fs/promises", () => ({
  default: {
    writeFile: vi.fn(),
    readFile: vi.fn(),
    unlink: vi.fn(),
  },
}));

import fs from "fs/promises";

describe("api/utils/file.consumeline.js", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("consumeline", () => {
    it("should return a random line and remove it from file", async () => {
      fs.writeFile.mockResolvedValueOnce();
      const fileContent = "line1\nline2\nline3";
      fs.readFile.mockResolvedValueOnce(fileContent);
      fs.writeFile.mockResolvedValueOnce();
      fs.unlink.mockResolvedValue();

      const result = await consumeline("test.txt");

      expect(result).toMatch(/line[123]/);
      expect(fs.writeFile).toHaveBeenCalledTimes(2);
    });

    it("should acquire lock before reading", async () => {
      fs.writeFile.mockResolvedValueOnce();
      fs.readFile.mockResolvedValueOnce("line1\nline2");
      fs.writeFile.mockResolvedValueOnce();
      fs.unlink.mockResolvedValue();

      await consumeline("test.txt");

      expect(fs.writeFile).toHaveBeenCalledWith(
        "test.txt.lock",
        expect.any(String),
        {
          flag: "wx",
        },
      );
    });

    it("should return null if file is empty", async () => {
      fs.writeFile.mockResolvedValueOnce();
      fs.readFile.mockResolvedValueOnce("");
      fs.unlink.mockResolvedValue();

      const result = await consumeline("test.txt");

      expect(result).toBeNull();
    });

    it("should return null if file has only empty lines", async () => {
      fs.writeFile.mockResolvedValueOnce();
      fs.readFile.mockResolvedValueOnce("\n\n\n");
      fs.unlink.mockResolvedValue();

      const result = await consumeline("test.txt");

      expect(result).toBeNull();
    });

    it("should call writeFile to acquire lock", async () => {
      fs.writeFile.mockResolvedValueOnce();
      fs.readFile.mockResolvedValueOnce("line1");
      fs.writeFile.mockResolvedValueOnce();
      fs.unlink.mockResolvedValue();

      await consumeline("test.txt");

      expect(fs.writeFile).toHaveBeenCalledWith(
        expect.stringContaining("lock"),
        expect.any(String),
        { flag: "wx" },
      );
    });

    it("should return null on read error", async () => {
      fs.writeFile.mockResolvedValueOnce();
      fs.readFile.mockRejectedValueOnce(new Error("Read error"));
      fs.unlink.mockResolvedValue();

      const result = await consumeline("test.txt");

      expect(result).toBeNull();
    });

    it("should cleanup lock file on error", async () => {
      fs.writeFile.mockResolvedValueOnce();
      fs.readFile.mockRejectedValueOnce(new Error("Read error"));
      fs.unlink.mockResolvedValue();

      await consumeline("test.txt");

      expect(fs.unlink).toHaveBeenCalled();
    });

    it("should filter out empty lines before selecting", async () => {
      fs.writeFile.mockResolvedValueOnce();
      fs.readFile.mockResolvedValueOnce("line1\n\nline2\n\nline3");
      fs.writeFile.mockResolvedValueOnce();
      fs.unlink.mockResolvedValue();

      const result = await consumeline("test.txt");

      expect(result).toMatch(/line[123]/);
    });

    it("should update file with remaining lines", async () => {
      fs.writeFile.mockResolvedValueOnce();
      const fileContent = "line1\nline2\nline3";
      fs.readFile.mockResolvedValueOnce(fileContent);
      fs.writeFile.mockResolvedValueOnce();
      fs.unlink.mockResolvedValue();

      await consumeline("test.txt");

      const writeCall = fs.writeFile.mock.calls[1];
      expect(writeCall[0]).toContain("test.txt");
      expect(writeCall[1]).toMatch(/line/);
    });

    it("should use custom maxRetries", async () => {
      const error = new Error("Lock exists");
      error.code = "EEXIST";
      fs.writeFile.mockRejectedValue(error);

      await consumeline("test.txt", { maxRetries: 5, retryDelay: 1 });

      expect(fs.writeFile).toHaveBeenCalledTimes(5);
    });

    it("should use custom retryDelay", async () => {
      fs.writeFile.mockResolvedValueOnce();
      fs.readFile.mockResolvedValueOnce("line1");
      fs.writeFile.mockResolvedValueOnce();
      fs.unlink.mockResolvedValue();

      await consumeline("test.txt", { retryDelay: 100 });

      expect(fs.writeFile).toHaveBeenCalledTimes(2);
    });

    it("should handle Windows line endings", async () => {
      fs.writeFile.mockResolvedValueOnce();
      fs.readFile.mockResolvedValueOnce("line1\r\nline2\r\nline3");
      fs.writeFile.mockResolvedValueOnce();
      fs.unlink.mockResolvedValue();

      const result = await consumeline("test.txt");

      expect(result).toMatch(/line[123]/);
    });
  });
});
