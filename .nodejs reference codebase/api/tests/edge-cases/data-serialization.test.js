/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Edge Case Tests: Data Serialization and Encoding
 *
 * Tests for handling data conversion edge cases:
 * - JSON serialization quirks
 * - Encoding/decoding issues
 * - Buffer handling
 * - Data transformation edge cases
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

describe("Edge Cases: Data Serialization", () => {
  describe("JSON Serialization Edge Cases", () => {
    it("should handle circular references in JSON.stringify", () => {
      const obj = { name: "test" };
      obj.self = obj;

      expect(() => JSON.stringify(obj)).toThrow("circular");

      // Use replacement function to handle circular refs
      const seen = new WeakSet();
      const safeStringify = (value) => {
        return JSON.stringify(value, (key, val) => {
          if (typeof val === "object" && val !== null) {
            if (seen.has(val)) return "[Circular]";
            seen.add(val);
          }
          return val;
        });
      };

      const result = safeStringify(obj);
      expect(result).toContain("[Circular]");
    });

    it("should handle BigInt serialization", () => {
      const bigValue = BigInt(9007199254740991);
      const obj = { value: bigValue };

      // BigInt cannot be serialized directly
      expect(() => JSON.stringify(obj)).toThrow();

      // Custom serializer
      const serializeWithBigInt = (value) => {
        return JSON.stringify(value, (key, val) => {
          if (typeof val === "bigint") {
            return { $bigint: val.toString() };
          }
          return val;
        });
      };

      const serialized = serializeWithBigInt(obj);
      const parsed = JSON.parse(serialized);
      expect(parsed.value.$bigint).toBe("9007199254740991");
    });

    it("should handle undefined values in JSON", () => {
      const obj = {
        defined: "value",
        undefined: undefined,
        null: null,
      };

      const serialized = JSON.stringify(obj);
      // undefined values are omitted
      expect(serialized).not.toContain("undefined");
      expect(serialized).toContain("null");
    });

    it("should handle special number values", () => {
      const obj = {
        inf: Infinity,
        negInf: -Infinity,
        nan: NaN,
      };

      const serialized = JSON.stringify(obj);
      // Special numbers become null
      expect(serialized).toBe('{"inf":null,"negInf":null,"nan":null}');
    });

    it("should handle Date serialization", () => {
      const date = new Date("2024-01-15T10:30:00.000Z");
      const obj = { timestamp: date };

      const serialized = JSON.stringify(obj);
      const parsed = JSON.parse(serialized);

      // Date becomes string
      expect(typeof parsed.timestamp).toBe("string");
      expect(parsed.timestamp).toBe("2024-01-15T10:30:00.000Z");

      // Restore Date
      parsed.timestamp = new Date(parsed.timestamp);
      expect(parsed.timestamp instanceof Date).toBe(true);
    });

    it("should handle Function serialization", () => {
      const obj = {
        name: "test",
        fn: function () {
          return 42;
        },
        arrow: () => 42,
      };

      const serialized = JSON.stringify(obj);
      const parsed = JSON.parse(serialized);

      // Functions are omitted
      expect(parsed.name).toBe("test");
      expect(parsed.fn).toBeUndefined();
      expect(parsed.arrow).toBeUndefined();
    });

    it("should handle Symbol serialization", () => {
      const sym = Symbol("test");
      const obj = {
        [sym]: "symbol value",
        regular: "regular value",
      };

      const serialized = JSON.stringify(obj);
      const parsed = JSON.parse(serialized);

      // Symbol keys are omitted
      expect(parsed.regular).toBe("regular value");
      expect(Object.keys(parsed).length).toBe(1);
    });

    it("should handle Map and Set serialization", () => {
      const map = new Map([
        ["a", 1],
        ["b", 2],
      ]);
      const set = new Set([1, 2, 3]);
      const obj = { map, set };

      // Both Maps and Sets become empty objects in JSON
      const serialized = JSON.stringify(obj);
      const parsed = JSON.parse(serialized);

      expect(parsed.map).toEqual({});
      expect(parsed.set).toEqual({});

      // Custom serializer
      const serializeMapSet = (value) => {
        return JSON.stringify(value, (key, val) => {
          if (val instanceof Map) {
            return { $map: Array.from(val.entries()) };
          }
          if (val instanceof Set) {
            return { $set: Array.from(val) };
          }
          return val;
        });
      };

      const customSerialized = serializeMapSet(obj);
      const customParsed = JSON.parse(customSerialized);
      expect(customParsed.map.$map).toEqual([
        ["a", 1],
        ["b", 2],
      ]);
      expect(customParsed.set.$set).toEqual([1, 2, 3]);
    });

    it("should handle very large JSON objects", () => {
      const largeArray = Array(10000)
        .fill(null)
        .map((_, i) => ({
          id: i,
          data: `item-${i}`,
        }));

      const serialized = JSON.stringify(largeArray);
      expect(serialized.length).toBeGreaterThan(100000);

      const parsed = JSON.parse(serialized);
      expect(parsed.length).toBe(10000);
      expect(parsed[9999].id).toBe(9999);
    });

    it("should handle JSON parsing errors gracefully", () => {
      const invalidJsonStrings = [
        "",
        "{",
        "}",
        "{invalid}",
        "undefined",
        "NaN",
        '{"key": }',
        '{"key": "value",}',
      ];

      invalidJsonStrings.forEach((str) => {
        expect(() => JSON.parse(str)).toThrow();
      });

      // Safe parse wrapper
      const safeParse = (str) => {
        try {
          return { success: true, data: JSON.parse(str) };
        } catch (error) {
          return { success: false, error: error.message };
        }
      };

      const result = safeParse("{invalid}");
      expect(result.success).toBe(false);
    });
  });

  describe("Encoding and Decoding", () => {
    it("should handle Base64 encoding edge cases", () => {
      const testCases = [
        { input: "Hello", expected: "SGVsbG8=" },
        { input: "", expected: "" },
        { input: "a", expected: "YQ==" },
        { input: "ab", expected: "YWI=" },
        { input: "abc", expected: "YWJj" },
      ];

      testCases.forEach(({ input, expected }) => {
        const encoded = Buffer.from(input).toString("base64");
        expect(encoded).toBe(expected);

        const decoded = Buffer.from(encoded, "base64").toString("utf-8");
        expect(decoded).toBe(input);
      });
    });

    it("should handle URL encoding edge cases", () => {
      const specialChars = "hello world!@#$%^&*()";
      const encoded = encodeURIComponent(specialChars);
      const decoded = decodeURIComponent(encoded);

      expect(encoded).not.toBe(specialChars);
      expect(decoded).toBe(specialChars);

      // Edge cases
      expect(encodeURIComponent(" ")).toBe("%20");
      expect(encodeURIComponent("\n")).toBe("%0A");
      expect(encodeURIComponent("\t")).toBe("%09");
    });

    it("should handle Unicode encoding", () => {
      const unicodeStrings = [
        "Hello 世界", // Chinese
        "Привет мир", // Russian
        "مرحبا بالعالم", // Arabic
        "🌍🌎🌏", // Emojis
        "Hello \u0000 World", // Null character
      ];

      unicodeStrings.forEach((str) => {
        const encoded = encodeURIComponent(str);
        const decoded = decodeURIComponent(encoded);
        expect(decoded).toBe(str);
      });
    });

    it("should handle Hex encoding", () => {
      const input = "Hello";
      const hex = Buffer.from(input).toString("hex");
      expect(hex).toBe("48656c6c6f");

      const decoded = Buffer.from(hex, "hex").toString("utf-8");
      expect(decoded).toBe(input);
    });

    it("should handle Binary data", () => {
      const bytes = new Uint8Array([72, 101, 108, 108, 111]); // "Hello"
      const buffer = Buffer.from(bytes);
      const text = buffer.toString("utf-8");
      expect(text).toBe("Hello");
    });

    it("should handle mixed encoding scenarios", () => {
      const obj = {
        text: "Hello 世界",
        special: "a=b&c=d",
        emoji: "😀😁😂",
      };

      // URL-safe Base64
      const jsonString = JSON.stringify(obj);
      const base64 = Buffer.from(jsonString).toString("base64url");

      const decoded = Buffer.from(base64, "base64url").toString("utf-8");
      const parsed = JSON.parse(decoded);

      expect(parsed).toEqual(obj);
    });
  });

  describe("Buffer and Stream Handling", () => {
    it("should handle empty buffer", () => {
      const empty = Buffer.alloc(0);
      expect(empty.length).toBe(0);
      expect(empty.toString()).toBe("");
    });

    it("should handle buffer overflow", () => {
      const buf = Buffer.alloc(5);
      // Writing more than allocated
      buf.write("Hello World");
      // Only first 5 bytes written
      expect(buf.toString()).toBe("Hello");
    });

    it("should handle buffer slicing", () => {
      const original = Buffer.from("Hello World");
      const slice = original.slice(0, 5);

      expect(slice.toString()).toBe("Hello");

      // Slice is a view, not a copy
      original[0] = "J".charCodeAt(0);
      expect(slice[0]).toBe("J".charCodeAt(0));
    });

    it("should handle buffer concatenation", () => {
      const buf1 = Buffer.from("Hello");
      const buf2 = Buffer.from(" ");
      const buf3 = Buffer.from("World");

      const combined = Buffer.concat([buf1, buf2, buf3]);
      expect(combined.toString()).toBe("Hello World");
    });

    it("should handle TypedArray conversions", () => {
      const uint8 = new Uint8Array([1, 2, 3, 4, 5]);
      const buffer = Buffer.from(uint8);

      expect(buffer.length).toBe(5);
      expect(buffer[0]).toBe(1);
      expect(buffer[4]).toBe(5);

      // Convert back
      const backToUint8 = new Uint8Array(buffer);
      expect(backToUint8).toEqual(uint8);
    });
  });

  describe("Data Transformation", () => {
    it("should handle deep clone of complex objects", () => {
      const original = {
        str: "text",
        num: 42,
        bool: true,
        arr: [1, 2, { nested: "value" }],
        obj: { a: 1, b: { c: 2 } },
      };

      // JSON clone preserves primitives and nested objects
      const jsonClone = JSON.parse(JSON.stringify(original));
      expect(jsonClone.str).toBe("text");
      expect(jsonClone.num).toBe(42);
      expect(jsonClone.arr[2].nested).toBe("value");
      expect(jsonClone.obj.b.c).toBe(2);

      // Verify deep clone - different references
      expect(jsonClone).not.toBe(original);
      expect(jsonClone.arr).not.toBe(original.arr);
      expect(jsonClone.obj).not.toBe(original.obj);
      expect(jsonClone.arr[2]).not.toBe(original.arr[2]);

      // Date serialization roundtrip
      const originalDate = new Date("2024-01-01");
      const dateClone = new Date(JSON.parse(JSON.stringify(originalDate)));
      expect(dateClone instanceof Date).toBe(true);
      expect(dateClone.getTime()).toBe(originalDate.getTime());

      // Map can be manually cloned
      const originalMap = new Map([
        ["a", 1],
        ["b", 2],
      ]);
      const mapClone = new Map(originalMap);
      expect(mapClone instanceof Map).toBe(true);
      expect(mapClone.get("a")).toBe(1);

      // Set can be manually cloned
      const originalSet = new Set([1, 2, 3]);
      const setClone = new Set(originalSet);
      expect(setClone instanceof Set).toBe(true);
      expect(setClone.has(2)).toBe(true);
    });

    it("should handle flatten and unflatten objects", () => {
      const flatten = (obj, prefix = "") => {
        const result = {};
        for (const [key, value] of Object.entries(obj)) {
          const newKey = prefix ? `${prefix}.${key}` : key;
          if (value && typeof value === "object" && !Array.isArray(value)) {
            Object.assign(result, flatten(value, newKey));
          } else {
            result[newKey] = value;
          }
        }
        return result;
      };

      const unflatten = (obj) => {
        const result = {};
        for (const [key, value] of Object.entries(obj)) {
          const parts = key.split(".");
          let current = result;
          for (let i = 0; i < parts.length - 1; i++) {
            current[parts[i]] = current[parts[i]] || {};
            current = current[parts[i]];
          }
          current[parts[parts.length - 1]] = value;
        }
        return result;
      };

      const original = {
        a: 1,
        b: {
          c: 2,
          d: {
            e: 3,
          },
        },
      };

      const flat = flatten(original);
      expect(flat).toEqual({ a: 1, "b.c": 2, "b.d.e": 3 });

      const unflat = unflatten(flat);
      expect(unflat).toEqual(original);
    });

    it("should handle object merging with conflicts", () => {
      const target = { a: 1, b: { c: 2, d: 3 } };
      const source = { b: { c: 4, e: 5 }, f: 6 };

      // Shallow merge
      const shallow = { ...target, ...source };
      expect(shallow.b).toEqual({ c: 4, e: 5 }); // Lost 'd'!

      // Deep merge
      const deepMerge = (target, source) => {
        const result = { ...target };
        for (const key of Object.keys(source)) {
          if (
            source[key] &&
            typeof source[key] === "object" &&
            !Array.isArray(source[key]) &&
            result[key] &&
            typeof result[key] === "object" &&
            !Array.isArray(result[key])
          ) {
            result[key] = deepMerge(result[key], source[key]);
          } else {
            result[key] = source[key];
          }
        }
        return result;
      };

      const deep = deepMerge(target, source);
      expect(deep.b).toEqual({ c: 4, d: 3, e: 5 }); // Preserved 'd'
      expect(deep.f).toBe(6);
    });

    it("should handle array manipulation edge cases", () => {
      const arr = [1, 2, 3, 4, 5];

      // Splice returns removed elements
      const removed = arr.splice(1, 2);
      expect(removed).toEqual([2, 3]);
      expect(arr).toEqual([1, 4, 5]);

      // Slice doesn't modify original
      const sliced = arr.slice(1, 3);
      expect(sliced).toEqual([4, 5]);
      expect(arr).toEqual([1, 4, 5]);

      // Flat handles nested arrays
      const nested = [1, [2, 3], [4, [5, 6]]];
      expect(nested.flat()).toEqual([1, 2, 3, 4, [5, 6]]);
      expect(nested.flat(Infinity)).toEqual([1, 2, 3, 4, 5, 6]);
    });

    it("should handle type coercion in comparisons", () => {
      // Loose equality - intentional constant comparisons to test JS behavior
      expect(1 == "1").toBe(true);
      expect(0 == "").toBe(true);
      expect(0 == false).toBe(true); // eslint-disable-line no-constant-binary-expression
      expect("" == false).toBe(true); // eslint-disable-line no-constant-binary-expression
      expect(null == undefined).toBe(true); // eslint-disable-line no-constant-binary-expression
      expect(NaN == NaN).toBe(false); // eslint-disable-line use-isnan -- Special case!

      // Strict equality
      expect(1 === "1").toBe(false);
      expect(0 === "").toBe(false);
      expect(null === undefined).toBe(false); // eslint-disable-line no-constant-binary-expression

      // Object.is - even stricter
      expect(Object.is(0, -0)).toBe(false);
      expect(Object.is(NaN, NaN)).toBe(true);
    });
  });

  describe("Date and Time Serialization", () => {
    it("should handle ISO 8601 format variations", () => {
      const formats = [
        "2024-01-15T10:30:00.000Z",
        "2024-01-15T10:30:00Z",
        "2024-01-15T10:30:00.000+00:00",
        "2024-01-15T10:30:00+0000",
      ];

      formats.forEach((format) => {
        const date = new Date(format);
        expect(date instanceof Date).toBe(true);
        expect(Number.isNaN(date.getTime())).toBe(false);
      });
    });

    it("should handle timestamp conversions", () => {
      const now = Date.now();
      const date = new Date(now);
      const timestamp = date.getTime();

      expect(timestamp).toBe(now);

      // Seconds vs milliseconds
      const seconds = Math.floor(now / 1000);
      const fromSeconds = new Date(seconds * 1000);
      expect(fromSeconds.getTime()).toBe(Math.floor(now / 1000) * 1000);
    });

    it("should handle timezone serialization", () => {
      const date = new Date("2024-01-15T10:30:00-05:00");

      // Always serialize as UTC
      const iso = date.toISOString();
      expect(iso.endsWith("Z")).toBe(true);

      // Local time formatting
      const local = date.toLocaleString("en-US", {
        timeZone: "America/New_York",
      });
      expect(local).toBeDefined();
    });

    it("should handle date arithmetic edge cases", () => {
      const jan1 = new Date(2024, 0, 1);
      const jan32 = new Date(2024, 0, 32);

      // Jan 32 = Feb 1
      expect(jan32.getMonth()).toBe(1); // February (0-indexed)
      expect(jan32.getDate()).toBe(1);

      // Leap year
      const feb29 = new Date(2024, 1, 29);
      expect(feb29.getMonth()).toBe(1);
      expect(feb29.getDate()).toBe(29);

      // Non-leap year
      const feb29_2023 = new Date(2023, 1, 29);
      expect(feb29_2023.getMonth()).toBe(2); // March
      expect(feb29_2023.getDate()).toBe(1);
    });
  });

  describe("Binary Data Formats", () => {
    it("should handle ArrayBuffer operations", () => {
      const buffer = new ArrayBuffer(8);
      const view = new DataView(buffer);

      // Write values
      view.setUint32(0, 0x12345678);
      view.setUint32(4, 0xabcdef00);

      // Read values
      expect(view.getUint32(0)).toBe(0x12345678);
      expect(view.getUint32(4)).toBe(0xabcdef00);
    });

    it("should handle endianness", () => {
      const buffer = new ArrayBuffer(4);
      const view = new DataView(buffer);

      // Big endian
      view.setUint32(0, 0x12345678, false);
      expect(view.getUint32(0, false)).toBe(0x12345678);
      expect(view.getUint32(0, true)).toBe(0x78563412); // Reversed

      // Little endian
      view.setUint32(0, 0x12345678, true);
      expect(view.getUint32(0, true)).toBe(0x12345678);
      expect(view.getUint32(0, false)).toBe(0x78563412);
    });

    it("should handle Blob and File simulation", () => {
      const mockBlob = {
        size: 5,
        type: "text/plain",
        slice: (start, end, type) => ({
          size: end - start,
          type: type || "text/plain",
        }),
      };

      expect(mockBlob.size).toBe(5);
      expect(mockBlob.type).toBe("text/plain");

      const slice = mockBlob.slice(0, 3);
      expect(slice.size).toBe(3);
    });
  });
});
