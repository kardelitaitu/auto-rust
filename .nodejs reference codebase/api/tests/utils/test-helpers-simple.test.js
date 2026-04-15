/**
 * Unit tests for api/tests/utils/test-helpers.js
 * Simple tests for mock factories and utilities
 */

import { describe, it, expect } from "vitest";
import {
  createMockLogger,
  createSilentLogger,
  createMockPage,
  createMockLocator,
  createMockSession,
  createMockBrowser,
  createMockTask,
  createMockLLMResponse,
  randomString,
  randomUrl,
  randomEmail,
} from "./test-helpers.js";

describe("test-helpers.js - Mock Factories", () => {
  describe("createMockLogger", () => {
    it("should create logger with all methods", () => {
      const logger = createMockLogger("test");

      expect(logger.info).toBeDefined();
      expect(logger.warn).toBeDefined();
      expect(logger.error).toBeDefined();
      expect(logger.debug).toBeDefined();
      expect(logger.success).toBeDefined();
      expect(typeof logger.info).toBe("function");
    });

    it("should create logger with default module name", () => {
      const logger = createMockLogger();

      expect(logger.info).toBeDefined();
    });
  });

  describe("createSilentLogger", () => {
    it("should create logger with all methods", () => {
      const logger = createSilentLogger();

      expect(logger.info).toBeDefined();
      expect(logger.warn).toBeDefined();
      expect(logger.error).toBeDefined();
      expect(logger.debug).toBeDefined();
      expect(logger.success).toBeDefined();
    });
  });

  describe("createMockPage", () => {
    it("should create page with navigation methods", () => {
      const page = createMockPage();

      expect(page.goto).toBeDefined();
      expect(page.goBack).toBeDefined();
      expect(page.goForward).toBeDefined();
      expect(page.reload).toBeDefined();
    });

    it("should create page with content methods", () => {
      const page = createMockPage();

      expect(page.content).toBeDefined();
      expect(page.title).toBeDefined();
      expect(page.url).toBeDefined();
    });

    it("should create page with click and screenshot", () => {
      const page = createMockPage();

      expect(page.click).toBeDefined();
      expect(page.screenshot).toBeDefined();
    });

    it("should create page with evaluate methods", () => {
      const page = createMockPage();

      expect(page.evaluate).toBeDefined();
      expect(page.evaluateHandle).toBeDefined();
    });

    it("should create page with wait methods", () => {
      const page = createMockPage();

      expect(page.waitForTimeout).toBeDefined();
      expect(page.waitForLoadState).toBeDefined();
      expect(page.waitForURL).toBeDefined();
    });

    it("should create page with viewport", () => {
      const page = createMockPage();

      expect(page.viewportSize).toBeDefined();
      expect(page.viewportSize()).toEqual({ width: 1280, height: 720 });
    });

    it("should create page with keyboard", () => {
      const page = createMockPage();

      expect(page.keyboard).toBeDefined();
      expect(page.keyboard.press).toBeDefined();
      expect(page.keyboard.type).toBeDefined();
    });

    it("should create page with mouse", () => {
      const page = createMockPage();

      expect(page.mouse).toBeDefined();
      expect(page.mouse.click).toBeDefined();
      expect(page.mouse.move).toBeDefined();
    });

    it("should create page with locator methods", () => {
      const page = createMockPage();

      expect(page.locator).toBeDefined();
      expect(page.getByRole).toBeDefined();
      expect(page.getByText).toBeDefined();
      expect(page.getByTestId).toBeDefined();
    });

    it("should allow overriding defaults", () => {
      const customUrl = "https://custom.com";
      const page = createMockPage({ url: () => customUrl });

      expect(page.url()).toBe(customUrl);
    });
  });

  describe("createMockLocator", () => {
    it("should create locator with action methods", () => {
      const locator = createMockLocator();

      expect(locator.click).toBeDefined();
      expect(locator.fill).toBeDefined();
      expect(locator.type).toBeDefined();
      expect(locator.press).toBeDefined();
    });

    it("should create locator with query methods", () => {
      const locator = createMockLocator();

      expect(locator.count).toBeDefined();
      expect(locator.first).toBeDefined();
      expect(locator.last).toBeDefined();
      expect(locator.nth).toBeDefined();
    });

    it("should create locator with state methods", () => {
      const locator = createMockLocator();

      expect(locator.isVisible).toBeDefined();
      expect(locator.isHidden).toBeDefined();
      expect(locator.isEnabled).toBeDefined();
      expect(locator.isDisabled).toBeDefined();
    });

    it("should create locator with content methods", () => {
      const locator = createMockLocator();

      expect(locator.innerText).toBeDefined();
      expect(locator.innerHTML).toBeDefined();
      expect(locator.textContent).toBeDefined();
      expect(locator.getAttribute).toBeDefined();
    });

    it("should create locator with geometry methods", () => {
      const locator = createMockLocator();

      expect(locator.boundingBox).toBeDefined();
    });

    it("should allow chaining first/last/nth", () => {
      const locator = createMockLocator();

      expect(locator.first()).toBe(locator);
      expect(locator.last()).toBe(locator);
      expect(locator.nth(0)).toBe(locator);
    });
  });

  describe("createMockSession", () => {
    it("should create session with required properties", () => {
      const session = createMockSession();

      expect(session.id).toBeDefined();
      expect(session.browser).toBeDefined();
      expect(session.windowName).toBeDefined();
      expect(session.ws).toBeDefined();
      expect(session.http).toBeDefined();
      expect(session.port).toBeDefined();
      expect(session.status).toBeDefined();
    });

    it("should create session with default port", () => {
      const session = createMockSession();

      expect(session.port).toBe(9222);
    });

    it("should create session with default status", () => {
      const session = createMockSession();

      expect(session.status).toBe("active");
    });

    it("should allow overriding values", () => {
      const session = createMockSession({ port: 9333 });

      expect(session.port).toBe(9333);
    });
  });

  describe("createMockBrowser", () => {
    it("should create browser with required methods", () => {
      const browser = createMockBrowser();

      expect(browser.contexts).toBeDefined();
      expect(browser.newContext).toBeDefined();
      expect(browser.close).toBeDefined();
      expect(browser.isConnected).toBeDefined();
      expect(browser.version).toBeDefined();
    });

    it("should return true for isConnected", () => {
      const browser = createMockBrowser();

      expect(browser.isConnected()).toBe(true);
    });

    it("should return version string", () => {
      const browser = createMockBrowser();

      expect(typeof browser.version()).toBe("string");
    });
  });

  describe("createMockTask", () => {
    it("should create task with required properties", () => {
      const task = createMockTask();

      expect(task.id).toBeDefined();
      expect(task.taskName).toBeDefined();
      expect(task.payload).toBeDefined();
      expect(task.status).toBeDefined();
      expect(task.retries).toBeDefined();
      expect(task.maxRetries).toBeDefined();
    });

    it("should create task with default status", () => {
      const task = createMockTask();

      expect(task.status).toBe("pending");
    });

    it("should create task with default retries", () => {
      const task = createMockTask();

      expect(task.retries).toBe(0);
      expect(task.maxRetries).toBe(3);
    });

    it("should allow overriding values", () => {
      const task = createMockTask({ taskName: "custom" });

      expect(task.taskName).toBe("custom");
    });
  });

  describe("createMockLLMResponse", () => {
    it("should create response with required properties", () => {
      const response = createMockLLMResponse();

      expect(response.success).toBeDefined();
      expect(response.content).toBeDefined();
      expect(response.metadata).toBeDefined();
    });

    it("should create response with default success", () => {
      const response = createMockLLMResponse();

      expect(response.success).toBe(true);
    });

    it("should create response with metadata", () => {
      const response = createMockLLMResponse();

      expect(response.metadata.model).toBeDefined();
      expect(response.metadata.tokens).toBeDefined();
    });

    it("should allow overriding values", () => {
      const response = createMockLLMResponse({ success: false });

      expect(response.success).toBe(false);
    });
  });
});

describe("test-helpers.js - Data Generators", () => {
  describe("randomString", () => {
    it("should generate string of default length", () => {
      const str = randomString();

      expect(str).toHaveLength(10);
    });

    it("should generate string of custom length", () => {
      const str = randomString(20);

      expect(str).toHaveLength(20);
    });

    it("should generate alphanumeric strings", () => {
      const str = randomString(100);

      expect(str).toMatch(/^[a-z0-9]+$/);
    });

    it("should generate different strings each time", () => {
      const str1 = randomString();
      const str2 = randomString();

      expect(str1).not.toBe(str2);
    });
  });

  describe("randomUrl", () => {
    it("should generate valid URL format", () => {
      const url = randomUrl();

      expect(url).toMatch(/^https:\/\/[a-z0-9]+\.example\.com\/[a-z0-9]+$/);
    });

    it("should generate different URLs each time", () => {
      const url1 = randomUrl();
      const url2 = randomUrl();

      expect(url1).not.toBe(url2);
    });
  });

  describe("randomEmail", () => {
    it("should generate valid email format", () => {
      const email = randomEmail();

      expect(email).toMatch(/^[a-z0-9]+@example\.com$/);
    });

    it("should generate different emails each time", () => {
      const email1 = randomEmail();
      const email2 = randomEmail();

      expect(email1).not.toBe(email2);
    });
  });
});
