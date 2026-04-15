/**
 * System metrics collection utilities
 */

import os from "os";
import { createLogger } from "../../lib/logger.js";

const logger = createLogger("server/utils/metrics.js");

/** Bytes per gigabyte constant */
const BYTES_PER_GB = 1024 ** 3;

/** Rounding precision for memory values */
const MEMORY_PRECISION = 100;

/**
 * Platform display names mapping
 */
const PLATFORM_MAP = {
  win32: "Windows",
  darwin: "macOS",
  linux: "Linux",
  freebsd: "FreeBSD",
  openbsd: "OpenBSD",
  sunos: "Solaris",
  aix: "AIX",
  android: "Android",
  haiku: "Haiku",
};

/**
 * Validate CPU stats object
 * @param {Object} stats - CPU stats to validate
 * @returns {boolean} - True if valid
 */
function isValidCpuStats(stats) {
  return (
    stats &&
    typeof stats === "object" &&
    typeof stats.idle === "number" &&
    typeof stats.total === "number" &&
    !isNaN(stats.idle) &&
    !isNaN(stats.total)
  );
}

/**
 * Calculate CPU usage between two measurements.
 * @param {Object} prev - Previous CPU stats { idle, total }
 * @param {Object} current - Current CPU stats { idle, total }
 * @returns {number} - CPU usage percentage (0-100)
 */
export function calculateCpuUsage(prev, current) {
  // Validate inputs
  if (!isValidCpuStats(prev) || !isValidCpuStats(current)) {
    return 0;
  }

  // Calculate differences with bounds checking
  const idleDiff = Math.max(0, current.idle - prev.idle);
  const totalDiff = Math.max(0, current.total - prev.total);

  // Handle edge cases
  if (totalDiff === 0 || idleDiff > totalDiff) {
    return 0;
  }

  // Calculate and clamp usage to valid range
  const usage = ((totalDiff - idleDiff) / totalDiff) * 100;
  return Math.max(0, Math.min(100, Math.round(usage * 100) / 100));
}

/**
 * Get current system metrics.
 * @param {Object} lastCpuInfo - Previous CPU stats for delta calculation
 * @returns {Object} - System metrics { cpu, memory, platform, hostname, uptime }
 */
export function getSystemMetrics(lastCpuInfo = null) {
  try {
    const cpus = os.cpus();
    let totalIdle = 0;
    let totalTick = 0;

    for (const cpu of cpus) {
      for (const type in cpu.times) {
        totalTick += cpu.times[type];
      }
      totalIdle += cpu.times.idle;
    }

    const currentCpuInfo = { idle: totalIdle, total: totalTick };

    // Only calculate CPU usage if we have valid previous stats
    const cpuUsage = isValidCpuStats(lastCpuInfo)
      ? calculateCpuUsage(lastCpuInfo, currentCpuInfo)
      : 0;

    const totalMem = os.totalmem();
    const freeMem = os.freemem();
    const usedMem = totalMem - freeMem;
    const memPercent =
      totalMem > 0 ? Math.round((usedMem / totalMem) * 100) : 0;

    // Platform detection with fallback
    const platformName = PLATFORM_MAP[os.platform()] || os.platform();

    return {
      cpu: {
        usage: cpuUsage,
        cores: cpus.length,
      },
      memory: {
        total:
          Math.round((totalMem / BYTES_PER_GB) * MEMORY_PRECISION) /
          MEMORY_PRECISION,
        used:
          Math.round((usedMem / BYTES_PER_GB) * MEMORY_PRECISION) /
          MEMORY_PRECISION,
        free:
          Math.round((freeMem / BYTES_PER_GB) * MEMORY_PRECISION) /
          MEMORY_PRECISION,
        percent: memPercent,
      },
      platform: platformName,
      hostname: os.hostname(),
      uptime: os.uptime(),
      cpuInfo: currentCpuInfo, // Return for next calculation
    };
  } catch (error) {
    logger.error("Error getting system metrics:", error.message);
    return {
      cpu: { usage: 0, cores: 0 },
      memory: { total: 0, used: 0, free: 0, percent: 0 },
      platform: "Unknown",
      hostname: "Unknown",
      uptime: 0,
      cpuInfo: lastCpuInfo,
    };
  }
}
