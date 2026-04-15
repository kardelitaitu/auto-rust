/**
 * Test file for timestamp formatting logic
 * Tests the relative/absolute time display for dashboard tasks
 */

import { describe, it, expect } from "vitest";

function formatTaskTimestamp(timestamp) {
  if (!timestamp) return "-";

  const now = Date.now();
  const diffMs = now - timestamp;
  const diffSecs = Math.floor(diffMs / 1000);
  const diffMins = Math.floor(diffSecs / 60);
  const diffHrs = Math.floor(diffMins / 60);

  if (diffHrs < 1) {
    if (diffMins > 0) {
      const secs = diffSecs % 60;
      if (secs > 0) {
        return `${diffMins}m ${secs}s ago`;
      }
      return `${diffMins}m ago`;
    }
    return `${diffSecs}s ago`;
  }

  const date = new Date(timestamp);
  const hours = date.getHours();
  const minutes = date.getMinutes();
  const ampm = hours >= 12 ? "PM" : "AM";
  const displayHours = hours % 12 || 12;
  const displayMinutes = minutes.toString().padStart(2, "0");
  const timeStr = `${displayHours}:${displayMinutes} ${ampm}`;

  const tzOffset = -date.getTimezoneOffset() / 60;
  const tzStr = tzOffset >= 0 ? `GMT+${tzOffset}` : `GMT${tzOffset}`;

  return `${timeStr} (${tzStr})`;
}

describe("formatTaskTimestamp", () => {
  it('should return "-" for null timestamp', () => {
    expect(formatTaskTimestamp(null)).toBe("-");
  });

  it('should return "-" for undefined timestamp', () => {
    expect(formatTaskTimestamp(undefined)).toBe("-");
  });

  it("should return seconds ago for < 1 minute", () => {
    const timestamp = Date.now() - 30000;
    expect(formatTaskTimestamp(timestamp)).toBe("30s ago");
  });

  it("should return minutes ago for < 1 hour", () => {
    const timestamp = Date.now() - 60000;
    expect(formatTaskTimestamp(timestamp)).toBe("1m ago");
  });

  it("should return minutes and seconds for partial minute", () => {
    const timestamp = Date.now() - 90000;
    expect(formatTaskTimestamp(timestamp)).toBe("1m 30s ago");
  });

  it("should return minutes only when no seconds remaining", () => {
    const timestamp = Date.now() - 5 * 60000;
    expect(formatTaskTimestamp(timestamp)).toBe("5m ago");
  });

  it("should return 30 minutes ago", () => {
    const timestamp = Date.now() - 30 * 60000;
    expect(formatTaskTimestamp(timestamp)).toBe("30m ago");
  });

  it("should return minutes and seconds at 59:59", () => {
    const timestamp = Date.now() - 59 * 60000 - 59000;
    expect(formatTaskTimestamp(timestamp)).toBe("59m 59s ago");
  });

  it("should return absolute time for 1 hour ago", () => {
    const timestamp = Date.now() - 3600000;
    expect(formatTaskTimestamp(timestamp)).toContain("GMT");
  });

  it("should return absolute time for 2 hours ago", () => {
    const timestamp = Date.now() - 2 * 3600000;
    expect(formatTaskTimestamp(timestamp)).toContain("GMT");
  });

  it("should return absolute time for yesterday", () => {
    const timestamp = Date.now() - 24 * 3600000;
    expect(formatTaskTimestamp(timestamp)).toContain("GMT");
  });
});
