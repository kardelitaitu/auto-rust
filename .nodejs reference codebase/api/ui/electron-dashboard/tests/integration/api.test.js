/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * @fileoverview Integration tests for dashboard REST API endpoints
 */
import { describe, it, expect, beforeAll, afterAll, vi } from "vitest";
import { createServer } from "net";

/**
 * Find an available port by trying to bind to port 0
 */
function getAvailablePort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.listen(0, () => {
      const port = server.address().port;
      server.close(() => resolve(port));
    });
    server.on("error", reject);
  });
}

describe("Dashboard Export Endpoints", () => {
  let server;
  let TEST_PORT;

  beforeAll(async () => {
    TEST_PORT = await getAvailablePort();
    const { DashboardServer } = await import("../../dashboard.js");
    server = new DashboardServer(TEST_PORT);
    await server.start();
    // Give server time to fully initialize
    await new Promise((resolve) => setTimeout(resolve, 100));
  }, 30000);

  afterAll(async () => {
    if (server) {
      await server.stop();
      // Give time for port to be released
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  });

  it("should respond to /health endpoint", async () => {
    const response = await fetch(`http://localhost:${TEST_PORT}/health`);
    const data = await response.json();

    expect(data.status).toBe("ok");
    expect(data.timestamp).toBeDefined();
  });

  it("should respond to /api/status endpoint", async () => {
    const response = await fetch(`http://localhost:${TEST_PORT}/api/status`);
    const data = await response.json();

    expect(data.ready).toBe(true);
    expect(data.sessions).toBeDefined();
  });

  it("should export data as JSON", async () => {
    const response = await fetch(
      `http://localhost:${TEST_PORT}/api/export/json`,
    );

    expect(response.headers.get("content-disposition")).toContain(
      "dashboard-export.json",
    );
    const data = await response.json();
    expect(data.exportedAt).toBeDefined();
  });

  it("should export tasks as CSV", async () => {
    const response = await fetch(
      `http://localhost:${TEST_PORT}/api/export/csv`,
    );

    expect(response.headers.get("content-disposition")).toContain(
      "tasks-export.csv",
    );
    const text = await response.text();
    expect(text).toContain(
      "id,taskName,sessionId,timestamp,status,success,duration",
    );
  });

  it("should respond to /api/sessions endpoint", async () => {
    const response = await fetch(`http://localhost:${TEST_PORT}/api/sessions`);
    const data = await response.json();

    expect(Array.isArray(data)).toBe(true);
  });

  it("should respond to /api/metrics endpoint", async () => {
    const response = await fetch(`http://localhost:${TEST_PORT}/api/metrics`);
    const data = await response.json();

    expect(data).toBeDefined();
  });
});
