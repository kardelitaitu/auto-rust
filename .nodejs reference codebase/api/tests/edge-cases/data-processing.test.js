import { describe, it, expect } from "vitest";

/**
 * Edge Case Tests: Data Processing and Transformation
 *
 * Tests for data transformation, parsing, serialization edge cases
 * and complex data manipulation patterns.
 */
describe("Edge Cases: Data Processing and Transformation", () => {
  describe("Data Validation", () => {
    it("should validate deeply nested objects", () => {
      const validateNested = (obj, schema) => {
        const errors = [];

        const validate = (value, path, schemaNode) => {
          if (schemaNode.required && (value === undefined || value === null)) {
            errors.push(`${path} is required`);
            return;
          }

          if (value === undefined || value === null) return;

          if (schemaNode.type) {
            const actualType = Array.isArray(value) ? "array" : typeof value;
            if (actualType !== schemaNode.type) {
              errors.push(
                `${path} expected ${schemaNode.type}, got ${actualType}`,
              );
              return;
            }
          }

          if (schemaNode.children && typeof value === "object") {
            for (const [key, childSchema] of Object.entries(
              schemaNode.children,
            )) {
              validate(value[key], `${path}.${key}`, childSchema);
            }
          }
        };

        validate(obj, "root", schema);
        return errors;
      };

      const schema = {
        type: "object",
        required: true,
        children: {
          name: { type: "string", required: true },
          age: { type: "number", required: true },
          address: {
            type: "object",
            children: {
              street: { type: "string", required: true },
              city: { type: "string", required: true },
            },
          },
        },
      };

      const validData = {
        name: "John",
        age: 30,
        address: { street: "Main St", city: "NYC" },
      };
      expect(validateNested(validData, schema)).toEqual([]);

      const invalidData = {
        age: "thirty",
        address: { city: 123 },
      };
      const errors = validateNested(invalidData, schema);
      expect(errors).toContain("root.name is required");
      expect(errors).toContain("root.age expected number, got string");
      expect(errors).toContain("root.address.street is required");
    });

    it("should implement schema validation with coercion", () => {
      const validateAndCoerce = (data, schema) => {
        const result = {};
        const errors = [];

        for (const [key, rule] of Object.entries(schema)) {
          let value = data[key];

          if (value === undefined || value === null) {
            if (rule.default !== undefined) {
              value = rule.default;
            } else if (rule.required) {
              errors.push(`${key} is required`);
              continue;
            }
          }

          if (value !== undefined && rule.coerce) {
            try {
              value = rule.coerce(value);
            } catch (e) {
              errors.push(`${key} coercion failed: ${e.message}`);
              continue;
            }
          }

          if (value !== undefined && rule.validate && !rule.validate(value)) {
            errors.push(`${key} validation failed`);
            continue;
          }

          result[key] = value;
        }

        return { result, errors };
      };

      const schema = {
        count: {
          required: true,
          coerce: (v) => parseInt(v, 10),
          validate: (v) => !isNaN(v) && v > 0,
        },
        enabled: {
          default: false,
          coerce: (v) => Boolean(v),
        },
        tags: {
          coerce: (v) => (Array.isArray(v) ? v : [v]),
        },
      };

      const { result, errors } = validateAndCoerce(
        { count: "42", tags: "single" },
        schema,
      );

      expect(errors).toEqual([]);
      expect(result.count).toBe(42);
      expect(result.enabled).toBe(false);
      expect(result.tags).toEqual(["single"]);
    });

    it("should validate data against JSON schema-like structure", () => {
      const createValidator = (schema) => {
        const validate = (data, schemaNode, path = "root") => {
          const errors = [];

          if (schemaNode.type) {
            const type = schemaNode.type;
            if (type === "string" && typeof data !== "string") {
              errors.push(`${path}: expected string`);
            } else if (type === "number" && typeof data !== "number") {
              errors.push(`${path}: expected number`);
            } else if (type === "array" && !Array.isArray(data)) {
              errors.push(`${path}: expected array`);
            } else if (
              type === "object" &&
              (typeof data !== "object" || Array.isArray(data))
            ) {
              errors.push(`${path}: expected object`);
            }
          }

          if (
            schemaNode.properties &&
            typeof data === "object" &&
            !Array.isArray(data)
          ) {
            for (const [key, propSchema] of Object.entries(
              schemaNode.properties,
            )) {
              errors.push(...validate(data[key], propSchema, `${path}.${key}`));
            }
          }

          if (schemaNode.items && Array.isArray(data)) {
            data.forEach((item, i) => {
              errors.push(...validate(item, schemaNode.items, `${path}[${i}]`));
            });
          }

          return errors;
        };

        return { validate: (data) => validate(data, schema) };
      };

      const schema = {
        type: "object",
        properties: {
          users: {
            type: "array",
            items: {
              type: "object",
              properties: {
                name: { type: "string" },
                age: { type: "number" },
              },
            },
          },
        },
      };

      const validator = createValidator(schema);

      const valid = { users: [{ name: "John", age: 30 }] };
      expect(validator.validate(valid)).toEqual([]);

      const invalid = { users: ["not an object"] };
      const errors = validator.validate(invalid);
      expect(errors.length).toBeGreaterThan(0);
    });
  });

  describe("Data Transformation", () => {
    it("should implement deep object mapping", () => {
      const deepMap = (obj, fn, path = []) => {
        if (Array.isArray(obj)) {
          return obj.map((item, i) => deepMap(item, fn, [...path, i]));
        }

        if (obj && typeof obj === "object") {
          const result = {};
          for (const [key, value] of Object.entries(obj)) {
            result[key] = deepMap(value, fn, [...path, key]);
          }
          return result;
        }

        return fn(obj, path);
      };

      const obj = {
        a: 1,
        b: {
          c: 2,
          d: {
            e: 3,
          },
        },
        f: [1, 2, { g: 4 }],
      };

      const result = deepMap(obj, (value) =>
        typeof value === "number" ? value * 2 : value,
      );

      expect(result.a).toBe(2);
      expect(result.b.c).toBe(4);
      expect(result.b.d.e).toBe(6);
      expect(result.f[0]).toBe(2);
      expect(result.f[2].g).toBe(8);
    });

    it("should flatten nested structures", () => {
      const flatten = (obj, prefix = "", separator = ".") => {
        const result = {};

        const process = (current, currentPath) => {
          if (
            current &&
            typeof current === "object" &&
            !Array.isArray(current)
          ) {
            for (const [key, value] of Object.entries(current)) {
              const newPath = currentPath
                ? `${currentPath}${separator}${key}`
                : key;
              process(value, newPath);
            }
          } else {
            result[currentPath] = current;
          }
        };

        process(obj, prefix);
        return result;
      };

      const nested = {
        user: {
          profile: {
            name: "John",
            settings: {
              theme: "dark",
            },
          },
          posts: ["post1", "post2"],
        },
      };

      const flat = flatten(nested);

      expect(flat["user.profile.name"]).toBe("John");
      expect(flat["user.profile.settings.theme"]).toBe("dark");
      expect(flat["user.posts"]).toEqual(["post1", "post2"]);
    });

    it("should unflatten structures", () => {
      const unflatten = (obj, separator = ".") => {
        const result = {};

        for (const [key, value] of Object.entries(obj)) {
          const parts = key.split(separator);
          let current = result;

          for (let i = 0; i < parts.length - 1; i++) {
            if (!current[parts[i]] || typeof current[parts[i]] !== "object") {
              current[parts[i]] = {};
            }
            current = current[parts[i]];
          }

          current[parts[parts.length - 1]] = value;
        }

        return result;
      };

      const flat = {
        "user.name": "John",
        "user.age": 30,
        "user.address.street": "Main St",
      };

      const nested = unflatten(flat);

      expect(nested.user.name).toBe("John");
      expect(nested.user.age).toBe(30);
      expect(nested.user.address.street).toBe("Main St");
    });

    it("should implement object diff", () => {
      const diff = (obj1, obj2, path = []) => {
        const changes = [];

        const allKeys = new Set([
          ...Object.keys(obj1 || {}),
          ...Object.keys(obj2 || {}),
        ]);

        for (const key of allKeys) {
          const currentPath = [...path, key];
          const val1 = obj1?.[key];
          const val2 = obj2?.[key];

          if (val1 === val2) continue;

          if (
            val1 &&
            val2 &&
            typeof val1 === "object" &&
            typeof val2 === "object" &&
            !Array.isArray(val1) &&
            !Array.isArray(val2)
          ) {
            changes.push(...diff(val1, val2, currentPath));
          } else {
            changes.push({
              path: currentPath.join("."),
              type:
                val1 === undefined
                  ? "added"
                  : val2 === undefined
                    ? "removed"
                    : "changed",
              oldValue: val1,
              newValue: val2,
            });
          }
        }

        return changes;
      };

      const obj1 = { a: 1, b: { c: 2, d: 3 }, e: [1, 2] };
      const obj2 = { a: 1, b: { c: 2, d: 4 }, e: [1, 2, 3], f: 5 };

      const changes = diff(obj1, obj2);

      expect(changes).toContainEqual({
        path: "b.d",
        type: "changed",
        oldValue: 3,
        newValue: 4,
      });
      expect(changes).toContainEqual({
        path: "e",
        type: "changed",
        oldValue: [1, 2],
        newValue: [1, 2, 3],
      });
      expect(changes).toContainEqual({
        path: "f",
        type: "added",
        oldValue: undefined,
        newValue: 5,
      });
    });

    it("should implement pick and omit operations", () => {
      const pick = (obj, keys) => {
        const result = {};
        for (const key of keys) {
          if (key in obj) {
            result[key] = obj[key];
          }
        }
        return result;
      };

      const omit = (obj, keys) => {
        const result = { ...obj };
        for (const key of keys) {
          delete result[key];
        }
        return result;
      };

      const obj = { a: 1, b: 2, c: 3, d: 4 };

      expect(pick(obj, ["a", "c"])).toEqual({ a: 1, c: 3 });
      expect(omit(obj, ["b", "d"])).toEqual({ a: 1, c: 3 });
    });
  });

  describe("String Processing", () => {
    it("should handle template literal parsing", () => {
      const parseTemplate = (template, data) => {
        return template.replace(/\{\{(\w+)\}\}/g, (match, key) => {
          return data[key] !== undefined ? String(data[key]) : match;
        });
      };

      const template = "Hello {{name}}, you have {{count}} messages.";
      const result = parseTemplate(template, { name: "John", count: 5 });

      expect(result).toBe("Hello John, you have 5 messages.");

      // Missing key keeps placeholder
      expect(parseTemplate(template, { name: "John" })).toBe(
        "Hello John, you have {{count}} messages.",
      );
    });

    it("should implement fuzzy string matching", () => {
      const fuzzyMatch = (pattern, text, options = {}) => {
        const { caseSensitive = false, threshold = 0.5 } = options;

        const p = caseSensitive ? pattern : pattern.toLowerCase();
        const t = caseSensitive ? text : text.toLowerCase();

        if (t.includes(p)) return { match: true, score: 1 };

        // Levenshtein distance
        const matrix = [];
        for (let i = 0; i <= t.length; i++) {
          matrix[i] = [i];
        }
        for (let j = 0; j <= p.length; j++) {
          matrix[0][j] = j;
        }

        for (let i = 1; i <= t.length; i++) {
          for (let j = 1; j <= p.length; j++) {
            if (t[i - 1] === p[j - 1]) {
              matrix[i][j] = matrix[i - 1][j - 1];
            } else {
              matrix[i][j] = Math.min(
                matrix[i - 1][j - 1] + 1,
                matrix[i][j - 1] + 1,
                matrix[i - 1][j] + 1,
              );
            }
          }
        }

        const distance = matrix[t.length][p.length];
        const maxLen = Math.max(t.length, p.length);
        const score = 1 - distance / maxLen;

        return { match: score >= threshold, score };
      };

      expect(fuzzyMatch("hello", "hello world").match).toBe(true);
      expect(
        fuzzyMatch("hello", "Hello World", { caseSensitive: true }).match,
      ).toBe(false);
      expect(fuzzyMatch("hello", "Hello World").match).toBe(true);
      expect(fuzzyMatch("xyz", "hello").score).toBeLessThan(0.5);
    });

    it("should extract structured data from text", () => {
      const extractPatterns = (text, patterns) => {
        const results = {};

        for (const [name, pattern] of Object.entries(patterns)) {
          const matches = text.match(pattern);
          if (matches) {
            results[name] = matches[1] || matches[0];
          }
        }

        return results;
      };

      const text =
        "Contact: john@example.com, Phone: +1-555-123-4567, Price: $99.99";

      const extracted = extractPatterns(text, {
        email: /([a-zA-Z0-9._-]+@[a-zA-Z0-9._-]+\.[a-zA-Z0-9_-]+)/,
        phone: /(\+\d{1,3}-\d{3}-\d{3}-\d{4})/,
        price: /\$(\d+\.\d{2})/,
      });

      expect(extracted.email).toBe("john@example.com");
      expect(extracted.phone).toBe("+1-555-123-4567");
      expect(extracted.price).toBe("99.99");
    });

    it("should implement string sanitization", () => {
      const sanitize = (input, options = {}) => {
        const {
          trim = true,
          lowerCase = false,
          removeExtraSpaces = true,
          removeHtml = true,
          maxLength = Infinity,
        } = options;

        let result = input;

        if (trim) result = result.trim();
        if (removeHtml) result = result.replace(/<[^>]*>/g, "");
        if (removeExtraSpaces) result = result.replace(/\s+/g, " ");
        if (lowerCase) result = result.toLowerCase();
        if (result.length > maxLength) result = result.slice(0, maxLength);

        return result;
      };

      expect(sanitize("  Hello   World  ")).toBe("Hello World");
      expect(sanitize('<script>alert("xss")</script>Hello')).toBe(
        'alert("xss")Hello',
      );
      expect(sanitize("HELLO", { lowerCase: true })).toBe("hello");
      expect(sanitize("Hello World", { maxLength: 5 })).toBe("Hello");
    });
  });

  describe("Array Processing", () => {
    it("should implement array grouping", () => {
      const groupBy = (arr, key) => {
        return arr.reduce((groups, item) => {
          const groupKey = typeof key === "function" ? key(item) : item[key];
          if (!groups[groupKey]) {
            groups[groupKey] = [];
          }
          groups[groupKey].push(item);
          return groups;
        }, {});
      };

      const data = [
        { type: "fruit", name: "apple" },
        { type: "vegetable", name: "carrot" },
        { type: "fruit", name: "banana" },
        { type: "fruit", name: "orange" },
        { type: "vegetable", name: "broccoli" },
      ];

      const grouped = groupBy(data, "type");

      expect(grouped.fruit).toHaveLength(3);
      expect(grouped.vegetable).toHaveLength(2);
    });

    it("should implement array chunking", () => {
      const chunk = (arr, size) => {
        const chunks = [];
        for (let i = 0; i < arr.length; i += size) {
          chunks.push(arr.slice(i, i + size));
        }
        return chunks;
      };

      expect(chunk([1, 2, 3, 4, 5, 6, 7], 3)).toEqual([
        [1, 2, 3],
        [4, 5, 6],
        [7],
      ]);

      expect(chunk([], 5)).toEqual([]);
    });

    it("should implement array deduplication", () => {
      const unique = (arr, key) => {
        if (!key) {
          return [...new Set(arr)];
        }

        const seen = new Set();
        return arr.filter((item) => {
          const k = typeof key === "function" ? key(item) : item[key];
          if (seen.has(k)) return false;
          seen.add(k);
          return true;
        });
      };

      expect(unique([1, 2, 2, 3, 3, 3])).toEqual([1, 2, 3]);

      const data = [
        { id: 1, name: "a" },
        { id: 2, name: "b" },
        { id: 1, name: "c" },
      ];
      expect(unique(data, "id")).toHaveLength(2);
    });

    it("should implement array shuffling", () => {
      const shuffle = (arr, seed = Date.now()) => {
        const result = [...arr];
        let random = seed;

        const seededRandom = () => {
          random = (random * 1103515245 + 12345) & 0x7fffffff;
          return random / 0x7fffffff;
        };

        for (let i = result.length - 1; i > 0; i--) {
          const j = Math.floor(seededRandom() * (i + 1));
          [result[i], result[j]] = [result[j], result[i]];
        }

        return result;
      };

      const original = [1, 2, 3, 4, 5];

      // Same seed produces same shuffle
      const shuffled1 = shuffle(original, 12345);
      const shuffled2 = shuffle(original, 12345);
      expect(shuffled1).toEqual(shuffled2);

      // Different seed produces different shuffle
      const shuffled3 = shuffle(original, 54321);
      expect(shuffled3).not.toEqual(shuffled1);

      // Original unchanged
      expect(original).toEqual([1, 2, 3, 4, 5]);
    });

    it("should implement zip operation", () => {
      const zip = (...arrays) => {
        const length = Math.min(...arrays.map((a) => a.length));
        return Array.from({ length }, (_, i) => arrays.map((a) => a[i]));
      };

      expect(zip([1, 2, 3], ["a", "b", "c"], [true, false, true])).toEqual([
        [1, "a", true],
        [2, "b", false],
        [3, "c", true],
      ]);

      // Different lengths
      expect(zip([1, 2], ["a", "b", "c"])).toEqual([
        [1, "a"],
        [2, "b"],
      ]);
    });

    it("should implement array partitioning", () => {
      const partition = (arr, predicate) => {
        const pass = [];
        const fail = [];

        for (const item of arr) {
          if (predicate(item)) {
            pass.push(item);
          } else {
            fail.push(item);
          }
        }

        return [pass, fail];
      };

      const [evens, odds] = partition([1, 2, 3, 4, 5, 6], (n) => n % 2 === 0);

      expect(evens).toEqual([2, 4, 6]);
      expect(odds).toEqual([1, 3, 5]);
    });
  });

  describe("JSON Processing", () => {
    it("should handle circular references in JSON", () => {
      const stringifySafe = (obj, space) => {
        const seen = new WeakSet();

        return JSON.stringify(
          obj,
          (key, value) => {
            if (typeof value === "object" && value !== null) {
              if (seen.has(value)) {
                return "[Circular]";
              }
              seen.add(value);
            }
            return value;
          },
          space,
        );
      };

      const obj = { a: 1 };
      obj.self = obj;

      const json = stringifySafe(obj);
      expect(json).toContain('"a":1');
      expect(json).toContain('"self":"[Circular]"');
    });

    it("should implement safe JSON parsing with defaults", () => {
      const parseSafe = (json, defaultValue = null) => {
        try {
          return JSON.parse(json);
        } catch {
          return defaultValue;
        }
      };

      expect(parseSafe('{"a":1}')).toEqual({ a: 1 });
      expect(parseSafe("invalid json")).toBe(null);
      expect(parseSafe("invalid json", {})).toEqual({});
    });

    it("should handle JSON path queries", () => {
      const jsonPath = (obj, path) => {
        const parts = path.split(".");
        let current = obj;

        for (const part of parts) {
          if (current === null || current === undefined) return undefined;
          current = current[part];
        }

        return current;
      };

      const data = {
        users: [
          { name: "John", posts: [{ title: "Hello" }] },
          { name: "Jane", posts: [{ title: "World" }] },
        ],
      };

      expect(jsonPath(data, "users.0.name")).toBe("John");
      expect(jsonPath(data, "users.1.posts.0.title")).toBe("World");
      expect(jsonPath(data, "users.2.name")).toBe(undefined);
    });

    it("should merge deep objects", () => {
      const deepMerge = (target, ...sources) => {
        for (const source of sources) {
          for (const [key, value] of Object.entries(source)) {
            if (value && typeof value === "object" && !Array.isArray(value)) {
              if (!target[key] || typeof target[key] !== "object") {
                target[key] = {};
              }
              deepMerge(target[key], value);
            } else {
              target[key] = value;
            }
          }
        }
        return target;
      };

      const target = { a: { b: 1, c: { d: 2 } } };
      const source = { a: { c: { e: 3 }, f: 4 } };

      const result = deepMerge(target, source);

      expect(result.a.b).toBe(1);
      expect(result.a.c.d).toBe(2);
      expect(result.a.c.e).toBe(3);
      expect(result.a.f).toBe(4);
    });
  });

  describe("Data Aggregation", () => {
    it("should implement reduce with index and array", () => {
      const reduceRight = (arr, fn, initial) => {
        let acc = initial;
        for (let i = arr.length - 1; i >= 0; i--) {
          acc = fn(acc, arr[i], i, arr);
        }
        return acc;
      };

      const result = reduceRight([1, 2, 3, 4], (acc, val) => acc + val, 0);
      expect(result).toBe(10);

      // Build string from right
      const str = reduceRight(["a", "b", "c"], (acc, val) => acc + val, "");
      expect(str).toBe("cba");
    });

    it("should implement aggregate functions", () => {
      const aggregate = (arr, field, operation) => {
        const values = arr.map((item) => item[field]);

        switch (operation) {
          case "sum":
            return values.reduce((a, b) => a + b, 0);
          case "avg":
            return values.reduce((a, b) => a + b, 0) / values.length;
          case "min":
            return Math.min(...values);
          case "max":
            return Math.max(...values);
          case "count":
            return values.length;
          default:
            throw new Error(`Unknown operation: ${operation}`);
        }
      };

      const data = [
        { product: "A", sales: 100 },
        { product: "B", sales: 200 },
        { product: "C", sales: 150 },
      ];

      expect(aggregate(data, "sales", "sum")).toBe(450);
      expect(aggregate(data, "sales", "avg")).toBe(150);
      expect(aggregate(data, "sales", "min")).toBe(100);
      expect(aggregate(data, "sales", "max")).toBe(200);
    });

    it("should implement running totals", () => {
      const runningTotal = (arr, field) => {
        let total = 0;
        return arr.map((item) => {
          total += item[field];
          return { ...item, [`running_${field}`]: total };
        });
      };

      const data = [
        { day: 1, value: 10 },
        { day: 2, value: 20 },
        { day: 3, value: 15 },
      ];

      const result = runningTotal(data, "value");

      expect(result[0].running_value).toBe(10);
      expect(result[1].running_value).toBe(30);
      expect(result[2].running_value).toBe(45);
    });

    it("should implement frequency counting", () => {
      const frequency = (arr, key) => {
        const freq = new Map();
        for (const item of arr) {
          const k =
            typeof key === "function" ? key(item) : key ? item[key] : item;
          freq.set(k, (freq.get(k) || 0) + 1);
        }
        return freq;
      };

      const words = ["apple", "banana", "apple", "orange", "banana", "apple"];
      const freq = frequency(words);

      expect(freq.get("apple")).toBe(3);
      expect(freq.get("banana")).toBe(2);
      expect(freq.get("orange")).toBe(1);
    });
  });

  describe("Data Encoding", () => {
    it("should implement base64 encoding/decoding", () => {
      const encode = (str) => Buffer.from(str).toString("base64");
      const decode = (base64) => Buffer.from(base64, "base64").toString("utf8");

      const original = "Hello, World! 你好";
      const encoded = encode(original);
      const decoded = decode(encoded);

      expect(encoded).not.toBe(original);
      expect(decoded).toBe(original);
    });

    it("should implement URL encoding/decoding", () => {
      const encode = (str) => encodeURIComponent(str);
      const decode = (encoded) => decodeURIComponent(encoded);

      const original = "key=value&special=!@#$%^&*()";
      const encoded = encode(original);
      const decoded = decode(encoded);

      expect(encoded).not.toBe(original);
      expect(decoded).toBe(original);
    });

    it("should implement hex encoding/decoding", () => {
      const encode = (str) => {
        return Array.from(str)
          .map((c) => c.charCodeAt(0).toString(16).padStart(2, "0"))
          .join("");
      };

      const decode = (hex) => {
        const bytes = hex.match(/.{2}/g) || [];
        return bytes.map((b) => String.fromCharCode(parseInt(b, 16))).join("");
      };

      const original = "Hello";
      const encoded = encode(original);
      const decoded = decode(encoded);

      expect(encoded).toBe("48656c6c6f");
      expect(decoded).toBe(original);
    });
  });
});
