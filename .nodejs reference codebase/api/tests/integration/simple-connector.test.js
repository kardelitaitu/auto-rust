/**
 * Simple Connector Discovery Integration Test
 * Basic connector discovery functionality
 */

import { describe, it, expect, vi } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  })),
}));

describe("Simple Connector Discovery Integration", () => {
  it("should import base discover module", async () => {
    const base = await import("../../connectors/baseDiscover.js");
    expect(base.BaseDiscover || base.default).toBeDefined();
  });

  it("should import local chrome discover", async () => {
    const chrome = await import("../../connectors/discovery/localChrome.js");
    expect(chrome.LocalChromeDiscover || chrome.default).toBeDefined();
  });

  it("should import ixbrowser discover", async () => {
    const ixbrowser = await import("../../connectors/discovery/ixbrowser.js");
    expect(ixbrowser.IxBrowserDiscover || ixbrowser.default).toBeDefined();
  });

  it("should import morelogin discover", async () => {
    const morelogin = await import("../../connectors/discovery/morelogin.js");
    expect(morelogin.MoreLoginDiscover || morelogin.default).toBeDefined();
  });
});
