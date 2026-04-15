/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview OWB Agent Runner
 * Auto-play strategy games with predefined rules.
 * Usage: node agent-main.js owb
 * @module agent-main
 */

import "dotenv/config";
import { createLogger } from "./api/core/logger.js";
import { showBanner } from "./api/utils/banner.js";
import Orchestrator from "./api/core/orchestrator.js";
import { ensureDockerLLM } from "./api/utils/dockerLLM.js";
import { llmClient } from "./api/agent/index.js";
import owbAgents from "./owb-agents.js";
import { GAME_CONFIG, LLM_CONFIG } from "./owb-config.js";
import { configManager } from "./api/core/config.js";

const logger = createLogger("agent-main.js");

// Global state for signal handlers
let isShuttingDown = false;
let globalOrchestrator = null;

/**
 * Graceful shutdown handler - closes all browsers before exit
 */
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
    if (globalOrchestrator) {
      await globalOrchestrator.shutdown();
      logger.info("[Shutdown] Orchestrator shutdown complete.");
    }
  } catch (error) {
    logger.error("[Shutdown] Error during shutdown:", error.message);
  }

  process.exit(0);
}

/**
 * Set the LLM model to use
 * @param {string} modelName - Model name (e.g., 'qwen3.5:4b')
 */
async function setLLMModel(modelName) {
  const modelToUse = modelName || LLM_CONFIG.defaultModel;

  try {
    await configManager.init();
    configManager.setOverride("agent.llm.model", modelToUse);
    configManager.setOverride("agent.llm.textModel", modelToUse);
    configManager.setOverride("agent.llm.think", false);

    // Reset llmClient config to pick up new overrides
    llmClient.config = null;

    logger.info(
      `LLM model set to: ${modelToUse} (from owb-config: ${LLM_CONFIG.defaultModel})`,
    );
  } catch (e) {
    logger.warn(`Could not set model: ${e.message}`);
  }
}

function showHelp() {
  console.log(`
OWB - Open World Browser Agent Runner
=====================================

Usage:
  node agent-main.js owb [mode] [options]

Modes (auto-play strategies):
  owb rush      - Aggressive early game
  owb turtle    - Defensive: build walls, grow economy, counter
  owb economy   - Focus on resources first
  owb balanced  - Mix of economy and military
  
  owb play      - Auto-play with default strategy (finite loops)
  owb play=rush - Auto-play with rush strategy

  owb build=X   - Build structure X
  owb train=X   - Train X units
  owb attack    - Attack enemy
  owb gather    - Gather resources

Options:
  --loops=N     - Number of loops (default: infinite)
  --model=NAME  - LLM model to use (default: ${LLM_CONFIG.defaultModel})
  --exit        - Auto-exit when done (close browser and process)
  --help        - Show this help

 Default Behavior:
   node agent-main.js owb    - Runs INFINITE auto-play loop (Ctrl+C to stop)

 Examples:
   node agent-main.js owb                    # Infinite loop (default)
   node agent-main.js owb play --loops=10    # Finite loops
   node agent-main.js owb play=rush          # Infinite with rush strategy
   node agent-main.js owb rush --loops=5     # 5 loops with rush
   node agent-main.js owb state-a            # Run state A once
   node agent-main.js owb state-a --exit     # Run state A and exit
   node agent-main.js owb balanced --model=qwen3.5:4b
`);
}

/**
 * Parse CLI arguments
 */
function parseArgs(args) {
  const result = {
    mode: "playInfinite", // Default to infinite loop
    strategy: GAME_CONFIG.defaultStrategy,
    target: null,
    model: LLM_CONFIG.defaultModel,
    browsers: [],
    loops: GAME_CONFIG.maxLoops,
    exitOnComplete: false,
    repeat: 1,
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];

    if (arg === "--help" || arg === "-h") {
      showHelp();
      process.exit(0);
    }

    if (arg.startsWith("--")) {
      if (arg.startsWith("--browsers=")) {
        result.browsers = arg.split("=")[1].split(",");
      } else if (arg.startsWith("--loops=")) {
        result.loops = parseInt(arg.split("=")[1], 10);
      } else if (arg.startsWith("--model=")) {
        result.model = arg.split("=")[1].trim();
      } else if (arg === "--exit") {
        result.exitOnComplete = true;
      } else if (arg === "--debug" || arg === "-d") {
        process.env.DEBUG = "true";
        logger.info("[DEBUG] Debug mode enabled");
      }
      continue;
    }

    if (arg === "owb") continue;

    if (arg === "play") {
      result.mode =
        result.loops !== GAME_CONFIG.maxLoops ? "play" : "playInfinite";
      continue;
    }

    if (arg.startsWith("play=")) {
      result.mode = "playInfinite";
      result.strategy = arg.split("=")[1].trim() || result.strategy;
      continue;
    }

    if (["rush", "turtle", "economy", "balanced"].includes(arg)) {
      result.mode = "play";
      result.strategy = arg;
      continue;
    }

    if (arg.startsWith("state-")) {
      result.mode = "state";
      result.target = arg.replace("state-", "").toUpperCase();
      continue;
    }

    if (arg.startsWith("x") && /^\d+$/.test(arg.substring(1))) {
      result.repeat = parseInt(arg.substring(1), 10);
      continue;
    }

    if (arg.startsWith("build=")) {
      result.mode = "build";
      result.target = arg.split("=")[1].trim();
      continue;
    }

    if (arg.startsWith("train=")) {
      result.mode = "train";
      result.target = arg.split("=")[1].trim();
      continue;
    }

    if (arg === "attack") {
      result.mode = "attack";
      continue;
    }

    if (arg === "gather") {
      result.mode = "gather";
      continue;
    }
  }

  return result;
}

/**
 * Execution logic
 */
(async () => {
  showBanner();
  logger.info("OWB Agent Runner - Starting...");

  const args = parseArgs(process.argv.slice(2));
  logger.info(
    `Mode: ${args.mode}, Strategy: ${args.strategy}, Model: ${args.model}`,
  );

  try {
    await setLLMModel(args.model);

    logger.info("Checking Docker LLM status...");
    const dockerReady = await ensureDockerLLM();
    if (!dockerReady) {
      logger.warn("Docker LLM not ready. Will use cloud fallback.");
    }

    globalOrchestrator = new Orchestrator();
    await globalOrchestrator.startDiscovery({ browsers: args.browsers });

    const sessionManager = globalOrchestrator.sessionManager;
    if (sessionManager.idleSessionsCount === 0) {
      logger.error("No browsers found. Please start a browser first.");
      process.exit(1);
    }

    // Execute based on mode
    let result;
    switch (args.mode) {
      case "playInfinite":
        await owbAgents.autoPlayInfinite(args.strategy);
        break;
      case "play":
        await owbAgents.autoPlay(args.strategy, { maxLoops: args.loops });
        break;
      case "state":
        for (let i = 0; i < args.repeat; i++) {
          logger.info(
            `Running state ${args.target} (repeat ${i + 1}/${args.repeat})`,
          );
          result = await owbAgents.executeState({
            key: args.target,
            name: args.target,
          });
        }
        break;
      case "build":
        result = await owbAgents.buildStructure(args.target);
        break;
      case "train":
        result = await owbAgents.trainUnits(args.target, args.repeat);
        break;
      case "attack":
        result = await owbAgents.attack();
        break;
      case "gather":
        result = await owbAgents.gatherResources();
        break;
      default:
        logger.warn(`Unknown mode: ${args.mode}, falling back to playInfinite`);
        await owbAgents.autoPlayInfinite(args.strategy);
    }

    if (result) {
      logger.info("========================================");
      logger.info("RESULT:", result);
      logger.info("========================================");
    }

    if (args.exitOnComplete) {
      logger.info("Stopping orchestrator...");
      await globalOrchestrator.shutdown();
      process.exit(0);
    } else {
      logger.info(
        "Cycle complete. Keeping browser open. Press Ctrl+C to exit.",
      );
      await new Promise(() => {});
    }
  } catch (error) {
    logger.error("Agent execution failed:", error.message);
    console.error(error.stack);
    process.exit(1);
  }
})();

// Process event listeners
process.on("uncaughtException", (error) => {
  logger.error("Uncaught Exception:", error.message);
  console.error(error.stack);
  process.exit(1);
});

process.on("unhandledRejection", (reason) => {
  logger.error("Unhandled Promise Rejection:", reason);
  process.exit(1);
});

process.on("SIGINT", () => gracefulShutdown("SIGINT"));
process.on("SIGTERM", () => gracefulShutdown("SIGTERM"));
