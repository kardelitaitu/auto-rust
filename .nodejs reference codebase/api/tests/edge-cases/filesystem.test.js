/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Edge Case Tests: File System Operations
 *
 * Tests for handling file system edge cases:
 * - File read/write errors
 * - Concurrent file access
 * - Large file handling
 * - Permission issues
 * - File watching
 */

import { describe, it, expect, vi } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

describe("Edge Cases: File System", () => {
  describe("File Read/Write Operations", () => {
    it("should handle file not found errors", async () => {
      const mockFs = {
        readFile: vi.fn().mockRejectedValue({
          code: "ENOENT",
          message: "ENOENT: no such file or directory",
        }),
      };

      const safeRead = async (path) => {
        try {
          return await mockFs.readFile(path, "utf-8");
        } catch (error) {
          if (error.code === "ENOENT") {
            return null;
          }
          throw error;
        }
      };

      const result = await safeRead("/nonexistent/file.txt");
      expect(result).toBeNull();
    });

    it("should handle permission denied errors", async () => {
      const mockFs = {
        readFile: vi.fn().mockRejectedValue({
          code: "EACCES",
          message: "EACCES: permission denied",
        }),
      };

      const safeRead = async (path) => {
        try {
          return await mockFs.readFile(path, "utf-8");
        } catch (error) {
          if (error.code === "EACCES") {
            throw new Error(`Permission denied: ${path}`, { cause: error });
          }
          throw error;
        }
      };

      await expect(safeRead("/protected/file.txt")).rejects.toThrow(
        "Permission denied",
      );
    });

    it("should handle file write with atomic pattern", async () => {
      const mockFs = {
        writeFile: vi.fn().mockResolvedValue(undefined),
        rename: vi.fn().mockResolvedValue(undefined),
        unlink: vi.fn().mockResolvedValue(undefined),
      };

      const atomicWrite = async (path, content) => {
        const tempPath = `${path}.tmp.${Date.now()}`;
        await mockFs.writeFile(tempPath, content);
        await mockFs.rename(tempPath, path);
      };

      await atomicWrite("/data/config.json", '{"key": "value"}');

      expect(mockFs.writeFile).toHaveBeenCalledWith(
        expect.stringContaining(".tmp."),
        '{"key": "value"}',
      );
      expect(mockFs.rename).toHaveBeenCalled();
    });

    it("should handle write failures with cleanup", async () => {
      const mockFs = {
        writeFile: vi.fn().mockRejectedValue(new Error("Disk full")),
        unlink: vi.fn().mockResolvedValue(undefined),
      };

      const safeWrite = async (path, content) => {
        const tempPath = `${path}.tmp`;
        try {
          await mockFs.writeFile(tempPath, content);
        } catch (error) {
          // Cleanup temp file
          await mockFs.unlink(tempPath).catch(() => {});
          throw error;
        }
      };

      await expect(safeWrite("/data/file.txt", "content")).rejects.toThrow(
        "Disk full",
      );

      expect(mockFs.unlink).toHaveBeenCalled();
    });

    it("should handle concurrent read access", async () => {
      let readCount = 0;
      const fileContent = "file content";

      const mockFs = {
        readFile: vi.fn().mockImplementation(async () => {
          readCount++;
          return fileContent;
        }),
      };

      const results = await Promise.all([
        mockFs.readFile("/data/file.txt"),
        mockFs.readFile("/data/file.txt"),
        mockFs.readFile("/data/file.txt"),
      ]);

      expect(readCount).toBe(3);
      expect(results).toEqual([fileContent, fileContent, fileContent]);
    });

    it("should handle large file reading in chunks", async () => {
      const chunkSize = 1024;
      const totalSize = 10000;
      let position = 0;

      const mockFs = {
        read: vi.fn().mockImplementation(async (buffer, offset, length) => {
          const bytesToRead = Math.min(length, totalSize - position);
          if (bytesToRead <= 0) return { bytesRead: 0 };

          const chunk = Buffer.alloc(bytesToRead, "A");
          chunk.copy(buffer, offset);
          position += bytesToRead;

          return { bytesRead: bytesToRead };
        }),
      };

      const readLargeFile = async () => {
        const chunks = [];
        const buffer = Buffer.alloc(chunkSize);

        while (position < totalSize) {
          const { bytesRead } = await mockFs.read(buffer, 0, chunkSize);
          if (bytesRead === 0) break;
          chunks.push(buffer.slice(0, bytesRead).toString());
        }

        return chunks;
      };

      const chunks = await readLargeFile();
      const totalRead = chunks.reduce((sum, chunk) => sum + chunk.length, 0);

      expect(totalRead).toBe(totalSize);
    });

    it("should handle file truncation", () => {
      const truncate = (content, maxLength) => {
        if (!content) return content;
        if (content.length <= maxLength) return content;

        const suffix = "...";
        const truncatedLength = maxLength - suffix.length;
        return content.slice(0, Math.max(0, truncatedLength)) + suffix;
      };

      expect(truncate("Hello", 10)).toBe("Hello");
      expect(truncate("Hello World!", 10)).toBe("Hello W...");
      expect(truncate("", 10)).toBe("");
      expect(truncate(null, 10)).toBe(null);
    });
  });

  describe("Directory Operations", () => {
    it("should handle directory creation with recursive flag", async () => {
      const createdDirs = [];
      const mockFs = {
        mkdir: vi.fn().mockImplementation(async (path, options) => {
          if (options?.recursive) {
            createdDirs.push(path);
          }
          return undefined;
        }),
      };

      await mockFs.mkdir("/deep/nested/path", { recursive: true });

      expect(mockFs.mkdir).toHaveBeenCalledWith(
        "/deep/nested/path",
        expect.objectContaining({ recursive: true }),
      );
    });

    it("should handle directory listing with filtering", async () => {
      const mockDirContents = [
        { name: "file1.txt", isFile: () => true, isDirectory: () => false },
        { name: "file2.js", isFile: () => true, isDirectory: () => false },
        { name: "subdir", isFile: () => false, isDirectory: () => true },
        { name: "file3.json", isFile: () => true, isDirectory: () => false },
      ];

      const filterFiles = (entries, extension) => {
        return entries
          .filter((e) => e.isFile())
          .map((e) => e.name)
          .filter((name) => !extension || name.endsWith(extension));
      };

      expect(filterFiles(mockDirContents)).toHaveLength(3);
      expect(filterFiles(mockDirContents, ".js")).toEqual(["file2.js"]);
      expect(filterFiles(mockDirContents, ".json")).toEqual(["file3.json"]);
    });

    it("should handle recursive directory traversal", () => {
      const mockFs = {
        readdir: vi.fn(),
        stat: vi.fn(),
      };

      // Mock directory structure
      const mockStructure = {
        "/root": ["file1.txt", "subdir"],
        "/root/subdir": ["file2.txt", "nested"],
        "/root/subdir/nested": ["file3.txt"],
      };

      mockFs.readdir.mockImplementation(async (dir) => {
        return mockStructure[dir] || [];
      });

      mockFs.stat.mockImplementation(async (path) => {
        return {
          isFile: () => !path.includes("subdir") || path.endsWith(".txt"),
          isDirectory: () => path.includes("subdir") && !path.endsWith(".txt"),
        };
      });

      const traverse = async (dir, depth = 0, maxDepth = 3) => {
        if (depth > maxDepth) return [];

        const entries = await mockFs.readdir(dir);
        const results = [];

        for (const entry of entries) {
          const fullPath = `${dir}/${entry}`;
          const stat = await mockFs.stat(fullPath);

          if (stat.isFile()) {
            results.push(fullPath);
          } else if (stat.isDirectory()) {
            const nested = await traverse(fullPath, depth + 1, maxDepth);
            results.push(...nested);
          }
        }

        return results;
      };

      return traverse("/root").then((results) => {
        expect(results.length).toBe(3);
        expect(results).toContain("/root/file1.txt");
      });
    });

    it("should handle path normalization", () => {
      const normalizePath = (path) => {
        const parts = path.split(/[/\\]/).filter(Boolean);
        const result = [];

        for (const part of parts) {
          if (part === ".") continue;
          if (part === "..") {
            result.pop();
          } else {
            result.push(part);
          }
        }

        return result.join("/");
      };

      expect(normalizePath("a/b/c")).toBe("a/b/c");
      expect(normalizePath("a/b/../c")).toBe("a/c");
      expect(normalizePath("a/./b/./c")).toBe("a/b/c");
      expect(normalizePath("../a/b")).toBe("a/b");
    });

    it("should get file extension safely", () => {
      const getFileExtension = (filename) => {
        if (!filename) return "";
        const lastDot = filename.lastIndexOf(".");
        if (lastDot === -1 || lastDot === 0) return "";
        return filename.slice(lastDot + 1).toLowerCase();
      };

      expect(getFileExtension("file.txt")).toBe("txt");
      expect(getFileExtension("archive.tar.gz")).toBe("gz");
      expect(getFileExtension(".hidden")).toBe("");
      expect(getFileExtension("noextension")).toBe("");
      expect(getFileExtension("")).toBe("");
      expect(getFileExtension(null)).toBe("");
    });

    it("should generate unique filenames", () => {
      const crypto = require("crypto");

      const generateUniqueName = (baseName, extension) => {
        const timestamp = Date.now();
        const random = crypto.randomBytes(4).toString("hex");
        return `${baseName}_${timestamp}_${random}.${extension}`;
      };

      const name1 = generateUniqueName("backup", "json");
      const name2 = generateUniqueName("backup", "json");

      expect(name1).toMatch(/^backup_\d+_[a-f0-9]{8}\.json$/);
      expect(name2).toMatch(/^backup_\d+_[a-f0-9]{8}\.json$/);
      expect(name1).not.toBe(name2);
    });
  });

  describe("File Locking and Concurrency", () => {
    it("should implement file lock mechanism", () => {
      const locks = new Map();

      const acquireLock = async (filepath, timeout = 5000) => {
        if (locks.has(filepath)) {
          const lockInfo = locks.get(filepath);
          if (Date.now() - lockInfo.timestamp < timeout) {
            return false; // Lock held
          }
        }

        locks.set(filepath, {
          timestamp: Date.now(),
          holder: Math.random().toString(36).slice(2),
        });
        return true;
      };

      const releaseLock = (filepath) => {
        locks.delete(filepath);
      };

      return acquireLock("/data/file.txt").then((acquired) => {
        expect(acquired).toBe(true);

        return acquireLock("/data/file.txt").then((acquired2) => {
          expect(acquired2).toBe(false);

          releaseLock("/data/file.txt");
          return acquireLock("/data/file.txt").then((acquired3) => {
            expect(acquired3).toBe(true);
          });
        });
      });
    });

    it("should handle write conflicts with retry", async () => {
      let writeInProgress = false;
      let writeCount = 0;

      const safeWrite = async (content, maxRetries = 3) => {
        for (let attempt = 0; attempt < maxRetries; attempt++) {
          if (!writeInProgress) {
            writeInProgress = true;
            try {
              // Simulate write
              writeCount++;
              await new Promise((r) => setTimeout(r, 10));
              return true;
            } finally {
              writeInProgress = false;
            }
          }
          await new Promise((r) => setTimeout(r, 50));
        }
        return false;
      };

      // Concurrent writes
      const results = await Promise.all([
        safeWrite("content1"),
        safeWrite("content2"),
        safeWrite("content3"),
      ]);

      expect(results.every((r) => r === true)).toBe(true);
      expect(writeCount).toBe(3);
    });

    it("should implement file advisory locking", () => {
      class FileLock {
        constructor() {
          this.locks = new Map();
        }

        lock(path, mode = "exclusive") {
          const existing = this.locks.get(path);
          if (existing) {
            if (existing.mode === "exclusive" || mode === "exclusive") {
              return false;
            }
          }
          this.locks.set(path, { mode, timestamp: Date.now() });
          return true;
        }

        unlock(path) {
          return this.locks.delete(path);
        }

        isLocked(path) {
          return this.locks.has(path);
        }
      }

      const lockManager = new FileLock();

      expect(lockManager.lock("/file.txt", "exclusive")).toBe(true);
      expect(lockManager.lock("/file.txt", "shared")).toBe(false);
      expect(lockManager.isLocked("/file.txt")).toBe(true);

      lockManager.unlock("/file.txt");
      expect(lockManager.isLocked("/file.txt")).toBe(false);
    });
  });

  describe("File Watching", () => {
    it("should handle file change events", async () => {
      const watchers = new Map();

      const watchFile = (path, callback) => {
        const watcher = {
          path,
          callback,
          close: () => watchers.delete(path),
        };
        watchers.set(path, watcher);
        return watcher;
      };

      const emitChange = (path, eventType) => {
        const watcher = watchers.get(path);
        if (watcher) {
          watcher.callback(eventType, path);
        }
      };

      const changes = [];
      const watcher = watchFile("/data/config.json", (type, path) => {
        changes.push({ type, path });
      });

      emitChange("/data/config.json", "change");
      emitChange("/data/config.json", "rename");

      expect(changes).toEqual([
        { type: "change", path: "/data/config.json" },
        { type: "rename", path: "/data/config.json" },
      ]);

      watcher.close();
      expect(watchers.size).toBe(0);
    });

    it("should debounce rapid file changes", () => {
      const debounceFileChange = (callback, delay = 300) => {
        let timeoutId = null;

        return (path) => {
          if (timeoutId) clearTimeout(timeoutId);
          timeoutId = setTimeout(() => {
            callback(path);
            timeoutId = null;
          }, delay);
        };
      };

      const changes = [];
      const handler = debounceFileChange((path) => {
        changes.push(path);
      }, 100);

      // Rapid changes
      handler("/file1.txt");
      handler("/file2.txt");
      handler("/file3.txt");

      return new Promise((resolve) => {
        setTimeout(() => {
          expect(changes).toEqual(["/file3.txt"]);
          resolve();
        }, 200);
      });
    });

    it("should implement recursive directory watching", () => {
      const watchTree = {
        handlers: new Map(),

        watch(path, recursive, callback) {
          this.handlers.set(path, { recursive, callback });
          return { path, close: () => this.handlers.delete(path) };
        },

        emit(path, event) {
          for (const [watchPath, { recursive, callback }] of this.handlers) {
            if (
              path === watchPath ||
              (recursive && path.startsWith(watchPath + "/"))
            ) {
              callback(event, path);
            }
          }
        },
      };

      const events = [];
      watchTree.watch("/data", true, (event, path) => {
        events.push({ event, path });
      });

      watchTree.emit("/data/config.json", "change");
      watchTree.emit("/data/sub/nested.json", "change");
      watchTree.emit("/other/file.txt", "change");

      expect(events).toHaveLength(2);
    });
  });

  describe("File Format Detection", () => {
    it("should detect file type from content", () => {
      const detectFileType = (content) => {
        if (!content || typeof content !== "string") return "unknown";

        const trimmed = content.trim();

        // JSON
        if (
          (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
          (trimmed.startsWith("[") && trimmed.endsWith("]"))
        ) {
          try {
            JSON.parse(trimmed);
            return "json";
          } catch {
            // Not valid JSON
          }
        }

        // XML/HTML
        if (trimmed.startsWith("<") && trimmed.endsWith(">")) {
          if (trimmed.includes("<!DOCTYPE") || trimmed.includes("<html")) {
            return "html";
          }
          return "xml";
        }

        // CSV (simple detection)
        const lines = trimmed.split("\n");
        if (lines.length > 1) {
          const commaCount = (lines[0].match(/,/g) || []).length;
          if (commaCount > 0 && lines[1].match(/,/g)?.length === commaCount) {
            return "csv";
          }
        }

        return "text";
      };

      expect(detectFileType('{"key": "value"}')).toBe("json");
      expect(detectFileType("[1, 2, 3]")).toBe("json");
      expect(detectFileType("<html><body></body></html>")).toBe("html");
      expect(detectFileType("<root><child/></root>")).toBe("xml");
      expect(detectFileType("name,age\nJohn,30")).toBe("csv");
      expect(detectFileType("plain text")).toBe("text");
      expect(detectFileType("")).toBe("unknown");
    });

    it("should detect encoding from buffer", () => {
      const detectEncoding = (buffer) => {
        if (!buffer || buffer.length === 0) return "unknown";

        // Check for BOM
        if (buffer[0] === 0xfe && buffer[1] === 0xff) return "utf-16-be";
        if (buffer[0] === 0xff && buffer[1] === 0xfe) return "utf-16-le";
        if (buffer[0] === 0xef && buffer[1] === 0xbb && buffer[2] === 0xbf)
          return "utf-8-bom";

        // Check for valid UTF-8
        let isValidUtf8 = true;
        for (let i = 0; i < buffer.length; i++) {
          const byte = buffer[i];
          if (byte < 0x80) continue;
          if ((byte & 0xe0) === 0xc0) {
            if (i + 1 >= buffer.length || (buffer[i + 1] & 0xc0) !== 0x80) {
              isValidUtf8 = false;
              break;
            }
            i++;
          } else if ((byte & 0xf0) === 0xe0) {
            if (i + 2 >= buffer.length) {
              isValidUtf8 = false;
              break;
            }
            i += 2;
          }
        }

        return isValidUtf8 ? "utf-8" : "binary";
      };

      expect(detectEncoding(Buffer.from("Hello", "utf-8"))).toBe("utf-8");
      expect(detectEncoding(Buffer.from([]))).toBe("unknown");

      // With BOM
      const utf8Bom = Buffer.from([
        0xef, 0xbb, 0xbf, 0x48, 0x65, 0x6c, 0x6c, 0x6f,
      ]);
      expect(detectEncoding(utf8Bom)).toBe("utf-8-bom");
    });
  });

  describe("Temp File Management", () => {
    it("should create and cleanup temp files", async () => {
      const tempFiles = new Set();

      const createTempFile = async (prefix, content) => {
        const id = `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2)}`;
        const path = `/tmp/${id}`;
        tempFiles.add(path);
        return path;
      };

      const cleanupTempFile = async (path) => {
        tempFiles.delete(path);
      };

      const cleanupAllTempFiles = async () => {
        const files = Array.from(tempFiles);
        tempFiles.clear();
        return files;
      };

      const file1 = await createTempFile("backup", "data1");
      const file2 = await createTempFile("cache", "data2");

      expect(tempFiles.size).toBe(2);

      await cleanupTempFile(file1);
      expect(tempFiles.size).toBe(1);

      await cleanupAllTempFiles();
      expect(tempFiles.size).toBe(0);
    });

    it("should implement temp file rotation", () => {
      class TempFileRotator {
        constructor(maxFiles = 5) {
          this.maxFiles = maxFiles;
          this.files = [];
        }

        add(filename) {
          this.files.push({
            name: filename,
            created: Date.now(),
          });

          // Remove oldest if over limit
          while (this.files.length > this.maxFiles) {
            this.files.shift();
          }
        }

        getFiles() {
          return [...this.files];
        }

        get oldest() {
          return this.files[0];
        }

        get newest() {
          return this.files[this.files.length - 1];
        }
      }

      const rotator = new TempFileRotator(3);

      rotator.add("file1.tmp");
      rotator.add("file2.tmp");
      rotator.add("file3.tmp");

      expect(rotator.getFiles()).toHaveLength(3);

      rotator.add("file4.tmp");

      expect(rotator.getFiles()).toHaveLength(3);
      expect(rotator.oldest.name).toBe("file2.tmp");
      expect(rotator.newest.name).toBe("file4.tmp");
    });

    it("should safely write temp file and rename", async () => {
      const operations = [];

      const atomicWrite = async (targetPath, content) => {
        const tempPath = `${targetPath}.tmp.${process.pid}`;

        // Write to temp
        operations.push({ op: "write", path: tempPath });

        // Sync
        operations.push({ op: "sync" });

        // Rename
        operations.push({ op: "rename", from: tempPath, to: targetPath });

        return true;
      };

      await atomicWrite("/data/config.json", '{"version": 2}');

      expect(operations).toHaveLength(3);
      expect(operations[0].op).toBe("write");
      expect(operations[2].op).toBe("rename");
    });
  });

  describe("File Size and Limits", () => {
    it("should enforce file size limits", () => {
      const maxSizeBytes = 10 * 1024 * 1024; // 10MB

      const validateFileSize = (size) => {
        if (size < 0) throw new Error("Invalid file size");
        if (size > maxSizeBytes) {
          throw new Error(
            `File size ${size} exceeds limit of ${maxSizeBytes} bytes`,
          );
        }
        return true;
      };

      expect(() => validateFileSize(1024)).not.toThrow();
      expect(() => validateFileSize(10 * 1024 * 1024)).not.toThrow();
      expect(() => validateFileSize(11 * 1024 * 1024)).toThrow("exceeds limit");
      expect(() => validateFileSize(-1)).toThrow("Invalid file size");
    });

    it("should format file sizes", () => {
      const formatSize = (bytes) => {
        const units = ["B", "KB", "MB", "GB", "TB"];
        let size = bytes;
        let unitIndex = 0;

        while (size >= 1024 && unitIndex < units.length - 1) {
          size /= 1024;
          unitIndex++;
        }

        return `${size.toFixed(2)} ${units[unitIndex]}`;
      };

      expect(formatSize(0)).toBe("0.00 B");
      expect(formatSize(1024)).toBe("1.00 KB");
      expect(formatSize(1024 * 1024)).toBe("1.00 MB");
      expect(formatSize(1536)).toBe("1.50 KB");
      expect(formatSize(1024 * 1024 * 1024)).toBe("1.00 GB");
    });

    it("should detect disk space issues", () => {
      const checkDiskSpace = (requiredBytes, availableBytes) => {
        if (requiredBytes > availableBytes) {
          const needed = requiredBytes - availableBytes;
          throw new Error(
            `Insufficient disk space. Need ${needed} more bytes.`,
          );
        }
        return true;
      };

      expect(() => checkDiskSpace(1000, 2000)).not.toThrow();
      expect(() => checkDiskSpace(2000, 1000)).toThrow(
        "Insufficient disk space",
      );
    });
  });

  describe("File Permissions", () => {
    it("should parse Unix file permissions", () => {
      const parsePermissions = (mode) => {
        const perms = {
          owner: { read: false, write: false, execute: false },
          group: { read: false, write: false, execute: false },
          other: { read: false, write: false, execute: false },
        };

        if (mode & 0o400) perms.owner.read = true;
        if (mode & 0o200) perms.owner.write = true;
        if (mode & 0o100) perms.owner.execute = true;
        if (mode & 0o040) perms.group.read = true;
        if (mode & 0o020) perms.group.write = true;
        if (mode & 0o010) perms.group.execute = true;
        if (mode & 0o004) perms.other.read = true;
        if (mode & 0o002) perms.other.write = true;
        if (mode & 0o001) perms.other.execute = true;

        return perms;
      };

      const perms755 = parsePermissions(0o755);
      expect(perms755.owner).toEqual({
        read: true,
        write: true,
        execute: true,
      });
      expect(perms755.group).toEqual({
        read: true,
        write: false,
        execute: true,
      });
      expect(perms755.other).toEqual({
        read: true,
        write: false,
        execute: true,
      });

      const perms644 = parsePermissions(0o644);
      expect(perms644.owner).toEqual({
        read: true,
        write: true,
        execute: false,
      });
      expect(perms644.group).toEqual({
        read: true,
        write: false,
        execute: false,
      });
    });

    it("should format permission string", () => {
      const formatPermissions = (mode) => {
        const type = (mode & 0o170000) === 0o040000 ? "d" : "-";
        const chars = [
          mode & 0o400 ? "r" : "-",
          mode & 0o200 ? "w" : "-",
          mode & 0o100 ? "x" : "-",
          mode & 0o040 ? "r" : "-",
          mode & 0o020 ? "w" : "-",
          mode & 0o010 ? "x" : "-",
          mode & 0o004 ? "r" : "-",
          mode & 0o002 ? "w" : "-",
          mode & 0o001 ? "x" : "-",
        ];
        return type + chars.join("");
      };

      expect(formatPermissions(0o755)).toBe("-rwxr-xr-x");
      expect(formatPermissions(0o644)).toBe("-rw-r--r--");
      expect(formatPermissions(0o40755)).toBe("drwxr-xr-x");
    });
  });
});
