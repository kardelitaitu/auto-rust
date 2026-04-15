/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

/**
 * @fileoverview Benchmark Harness
 * Provides utilities for measuring performance of critical paths
 * @module tests/benchmarks/harness
 */

/**
 * Simple benchmark runner
 * @param {string} name - Benchmark name
 * @param {Function} fn - Function to benchmark
 * @param {number} iterations - Number of iterations
 * @returns {Promise<{name: string, avg: number, min: number, max: number, total: number}>}
 */
export async function bench(name, fn, iterations = 100) {
  const times = [];

  for (let i = 0; i < iterations; i++) {
    const start = performance.now();
    await fn();
    const end = performance.now();
    times.push(end - start);
  }

  const total = times.reduce((a, b) => a + b, 0);
  const avg = total / times.length;
  const min = Math.min(...times);
  const max = Math.max(...times);

  return {
    name,
    avg: Math.round(avg * 100) / 100,
    min: Math.round(min * 100) / 100,
    max: Math.round(max * 100) / 100,
    total: Math.round(total * 100) / 100,
  };
}

/**
 * Format benchmark results for display
 * @param {object} result - Benchmark result
 * @returns {string}
 */
export function formatBench(result) {
  return `${result.name}: avg=${result.avg}ms, min=${result.min}ms, max=${result.max}ms (${result.total}ms total)`;
}

/**
 * Create a mock page for benchmarking
 * @returns {object}
 */
export function createMockPage() {
  return {
    url: () => "https://example.com",
    evaluate: () => Promise.resolve({}),
    waitForSelector: () => Promise.resolve({}),
    locator: () => ({
      first: () => ({
        boundingBox: () =>
          Promise.resolve({ x: 100, y: 100, width: 50, height: 20 }),
        isVisible: () => Promise.resolve(true),
        textContent: () => Promise.resolve("Test"),
        getAttribute: () => Promise.resolve("value"),
        evaluate: () => Promise.resolve(false), // For isObscured check
      }),
      count: () => Promise.resolve(5),
      evaluate: () => Promise.resolve(null), // For exists check
    }),
  };
}

/**
 * Create a mock cursor for benchmarking
 * @returns {object}
 */
export function createMockCursor() {
  return {
    move: () => Promise.resolve(),
    click: () => Promise.resolve(),
  };
}

export default { bench, formatBench, createMockPage, createMockCursor };
