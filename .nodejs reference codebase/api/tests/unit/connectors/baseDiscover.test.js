/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect } from "vitest";
import BaseDiscover from "@api/connectors/baseDiscover.js";

describe("BaseDiscover", () => {
  describe("Constructor", () => {
    it("should throw error when trying to instantiate BaseDiscover directly", () => {
      expect(() => new BaseDiscover()).toThrow(
        "Cannot construct BaseDiscover instances directly",
      );
    });

    it("should not throw when instantiated as part of a subclass", () => {
      class TestDiscover extends BaseDiscover {
        constructor() {
          super();
        }
        async discover() {
          return [];
        }
      }
      expect(() => new TestDiscover()).not.toThrow();
    });
  });

  describe("discover() method", () => {
    it("should throw error when called on BaseDiscover directly", async () => {
      class TestDiscover extends BaseDiscover {
        constructor() {
          super();
        }
      }
      const instance = new TestDiscover();
      await expect(instance.discover()).rejects.toThrow(
        'Method "discover()" must be implemented by subclass',
      );
    });
  });
});

describe("BaseDiscover subclass behavior", () => {
  class ConcreteDiscover extends BaseDiscover {
    constructor(mockReturnValue = []) {
      super();
      this._mockReturnValue = mockReturnValue;
    }

    async discover() {
      return this._mockReturnValue;
    }
  }

  it("should allow subclass to override discover method", async () => {
    const mockProfiles = [
      { id: "test-1", name: "Test Browser 1", type: "test" },
      { id: "test-2", name: "Test Browser 2", type: "test" },
    ];

    const discover = new ConcreteDiscover(mockProfiles);
    const result = await discover.discover();

    expect(result).toEqual(mockProfiles);
    expect(result).toHaveLength(2);
  });

  it("should allow subclass to return empty array", async () => {
    const discover = new ConcreteDiscover([]);
    const result = await discover.discover();

    expect(result).toEqual([]);
  });

  it("should allow subclass to return null or undefined", async () => {
    class NullDiscover extends BaseDiscover {
      constructor() {
        super();
      }
      async discover() {
        return null;
      }
    }

    const discover = new NullDiscover();
    const result = await discover.discover();

    expect(result).toBeNull();
  });
});
