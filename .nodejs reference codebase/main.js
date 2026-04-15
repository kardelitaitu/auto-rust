/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview The main entry point for the multi-browser automation framework.
 * This file initializes the orchestrator, discovers browsers, and runs automation tasks.
 * @module main
 */

import "dotenv/config";
import { createLogger } from "./api/core/logger.js";
import { showBanner } from "./api/utils/banner.js";
import Orchestrator from "./api/core/orchestrator.js";
import { ensureDockerLLM } from "./api/utils/dockerLLM.js";
import { getSettings } from "./api/utils/configLoader.js";
import { parseTaskArgs } from "./api/utils/task-parser.js";
import { validateTaskPayload } from "./api/utils/task-validator.js";
import {
  TaskStatus,
  summarizeResults,
  formatResult,
} from "./api/core/task-result.js";

const logger = createLogger("main.js");

/**
 * The main entry point of the application.
 * This is an IIFE (Immediately Invoked Function Expression) that runs asynchronously.
 * It sets up the orchestrator, discovers browsers, and runs automation tasks.
 */
(async () => {
  // State for signal handlers - scoped to this IIFE
  let isShuttingDown = false;
  let orchestrator = null;

  // Graceful shutdown handler - closes all browsers before exit
  async function gracefulShutdown(signal) {
    if (isShuttingDown) {
      logger.info(`[Shutdown] Already shutting down, ignoring ${signal}...`);
      return;
    }
    isShuttingDown = true;
    logger.info(
      `[Shutdown] Received ${signal}. Closing browsers and cleaning up...`,
    );

    try {
      if (orchestrator) {
        await orchestrator.shutdown();
        logger.info("[Shutdown] Orchestrator shutdown complete.");
      }
    } catch (error) {
      logger.error("[Shutdown] Error during shutdown:", error.message);
    }

    process.exit(0);
  }

  // Register signal handlers inside the IIFE
  process.on("SIGINT", async () => {
    await gracefulShutdown("SIGINT");
  });

  process.on("SIGTERM", async () => {
    await gracefulShutdown("SIGTERM");
  });

  // Show visual banner first
  showBanner();

  logger.info("MultiBrowseAutomation - Starting up...");

  try {
    // Step 1: Ensure Docker LLM is running (if enabled)
    // logger.info('Checking Docker LLM status...');
    const dockerReady = await ensureDockerLLM();
    if (!dockerReady) {
      logger.warn(
        "Docker LLM is not ready. Local vision processing may not work.",
      );
      logger.warn("Continuing anyway (will fall back to cloud if needed)...");
    }

    orchestrator = new Orchestrator();

    // Parse CLI arguments for options
    const args = process.argv.slice(2);
    const browsersArg = args.find((arg) => arg.startsWith("--browsers="));
    const browserList = browsersArg ? browsersArg.split("=")[1].split(",") : [];

    // Filter out known task names from args so we don't try to run "--browsers=..." as a task
    const tasksToRun = args.filter((arg) => !arg.startsWith("--"));

    logger.info("Attempting to discover and connect to available browsers...");

    // Start dashboard if enabled in config
    const settings = await getSettings();
    if (settings?.ui?.dashboard?.enabled) {
      await orchestrator.startDashboard(settings.ui.dashboard.port || 3001);
    }

    // Retry Loop for Discovery
    const maxRetries = 3;
    let attempt = 1;
    let connectedCount = 0;

    while (attempt <= maxRetries) {
      // if (attempt > 1) logger.info(`[Discovery] Attempt ${attempt}/${maxRetries}...`);

      await orchestrator.startDiscovery({ browsers: browserList });
      connectedCount = orchestrator.sessionManager.activeSessionsCount;

      if (connectedCount > 0) {
        break; // Success!
      }

      if (attempt < maxRetries) {
        logger.warn(
          `[Discovery] No browsers found on attempt ${attempt}. Retrying in 5 seconds...`,
        );
        // logger.info(
        //     `[Tip] Ensure your browser (e.g. ixbrowser, brave) is OPEN and running.`
        // );
        await new Promise((resolve) => setTimeout(resolve, 5000));
        // Clear previous connectors/state if needed, though Orchestrator handles reloading logic
      }
      attempt++;
    }

    const sessionManager = orchestrator.sessionManager;

    if (sessionManager.idleSessionsCount === 0) {
      logger.warn(
        "No browsers discovered or connected after retries. The system will continue but tasks may fail.",
      );
    } else {
      logger.info(
        `Successfully connected to ${sessionManager.idleSessionsCount} browser(s). Ready for automation.`,
      );
    }

    // Parse tasks into sequential groups separated by 'then'
    const { groups, taskCount } = parseTaskArgs(tasksToRun);

    if (taskCount === 0) {
      logger.info("No tasks specified. System initialized in idle mode.");
    } else {
      logger.info(
        `MultiBrowseAutomation - Initialized. Processing ${taskCount} task(s) in ${groups.length} group(s).`,
      );

      // Collect all task results for summary
      const allResults = [];

      for (let i = 0; i < groups.length; i++) {
        const group = groups[i];
        const groupResults = [];

        // Validate tasks before adding
        let tasksAdded = 0;
        let tasksSkipped = 0;

        for (const { name, payload } of group) {
          // Validate task payload
          const validation = validateTaskPayload(name, payload);
          if (!validation.isValid) {
            logger.warn(
              `[Queue] Skipping task '${name}': ${validation.errors.join(", ")}`,
            );
            groupResults.push({
              taskName: name,
              status: TaskStatus.FAILED,
              error: {
                name: "ValidationError",
                message: validation.errors.join(", "),
              },
            });
            tasksSkipped++;
            continue;
          }

          // Add warnings to logger
          if (validation.warnings) {
            validation.warnings.forEach((w) =>
              logger.warn(`[Queue] ${name}: ${w}`),
            );
          }

          try {
            orchestrator.addTask(name, payload);
            tasksAdded++;
          } catch (error) {
            logger.warn(`[Queue] Skipping task '${name}': ${error.message}`);
            groupResults.push({
              taskName: name,
              status: TaskStatus.FAILED,
              error: { name: "AddTaskError", message: error.message },
            });
            tasksSkipped++;
          }
        }

        if (tasksAdded === 0) {
          logger.warn(`[Queue] No valid tasks in group ${i + 1}, skipping...`);
          allResults.push(...groupResults);
          continue;
        }

        logger.info(
          `[Queue] Added ${tasksAdded} task(s)${tasksSkipped > 0 ? `, skipped ${tasksSkipped}` : ""}...`,
        );

        // Wait for tasks and collect results
        await orchestrator.waitForTasksToComplete();

        // Get results from orchestrator (will be populated by task events)
        logger.info(`[Queue] Group ${i + 1} completed.`);
      }

      // Print summary
      logger.info("\n========== TASK RESULTS ==========");
      const summary = summarizeResults(allResults);

      logger.info(`Total: ${summary.total}`);
      logger.info(`Success: ${summary.success}`);
      logger.info(`Failed: ${summary.failed}`);
      logger.info(`Success Rate: ${summary.successRate}`);
      logger.info(`Total Duration: ${summary.totalDuration.toFixed(1)}s`);
      logger.info("");

      // Print individual results
      for (const result of allResults) {
        logger.info(formatResult(result));
      }

      logger.info("=================================\n");

      // Exit with error code if any tasks failed
      if (summary.failed > 0 || summary.timeout > 0) {
        logger.error("Some tasks failed or timed out");
        process.exit(1);
      }
    }
  } catch (error) {
    logger.error("An unexpected error occurred during execution:", error);
    console.error(error.stack);
  } finally {
    // Cleanup
    logger.info("MultiBrowseAutomation - Shutting down...");
    if (orchestrator) {
      await orchestrator.shutdown();
    }
    process.exit(0);
  }
})();

// Handle uncaught errors and interrupts
process.on("uncaughtException", (error) => {
  logger.error("Uncaught Exception:", error.message);
  console.error(error.stack);
  process.exit(1);
});

process.on("unhandledRejection", (reason, promise) => {
  logger.error("Unhandled Promise Rejection at:", promise, "reason:", reason);
  process.exit(1);
});
