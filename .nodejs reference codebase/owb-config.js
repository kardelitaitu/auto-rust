/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview OWB Auto-Play Configuration
 * Territory game strategy configuration
 * @module owb-config
 */

export const LLM_CONFIG = {
  defaultModel: "gemma3:4b",
  fallbackModel: "qwen2.5vl:3b",
  visionEnabled: true,
  // Higher token limits for vision tasks with detailed prompts
  maxTokens: 4096,
  contextLength: 8192,
};

export const GAME_MECHANICS = {
  resources: {
    gold: {
      displayLocation: "top-right",
      selector: '[class*="gold"], #gold, [id*="gold"], :has-text("Gold")',
    },
  },
  land: {
    unowned: { color: "gray", description: "gray/unowned" },
    owned: { color: "blue", description: "blue/owned" },
    purchasable: { description: "adjacent to owned land" },
  },
  buildings: {
    defensive: {
      name: "Defensive",
      alias: ["defensive", "defense", "tower", "shield"],
      cost: { gold: 200 },
      description: "Defensive position building",
    },
  },
};

export const STRATEGIES = {
  defend: {
    name: "Defend",
    description: "Focus exclusively on defensive buildings",
    phases: [
      { priority: 1, action: "buyLand", count: 5, maxAttempts: 5 },
      {
        priority: 2,
        action: "build",
        building: "defensive",
        count: 20,
        maxAttempts: 10,
      },
      {
        priority: 3,
        action: "upgrade",
        building: "defensive",
        count: 20,
        maxAttempts: 15,
      },
    ],
  },
  balanced: {
    name: "Balanced",
    description: "Mix of expansion and all building types",
    phases: [
      { priority: 1, action: "buyLand", count: 2 },
      { priority: 2, action: "build", building: "defensive", count: 10 },
      {
        priority: 3,
        action: "upgrade",
        building: "defensive",
        count: 10,
        maxAttempts: 10,
      },
    ],
  },
};

export const GAME_CONFIG = {
  defaultStrategy: "defend",
  loopDelay: 500,
  phaseDelay: 1500,
  maxLoops: 10,
  stuckRecovery: true,
  defaultGold: 180,
  resources: {
    watchInterval: 2000,
    waitTimeout: 60000,
    minGoldForAction: 50,
    minGoldForUpgrade: 5000000,
  },
};

export const BUILDING_COSTS = {
  defensive: 200,
};

export const LAND_COSTS = {
  base: 50,
  increasePerLand: 10,
};

export const VPREP_CONFIG = {
  targetWidth: 640,
  contrast: 1.25,
  quality: 78,
};

export default {
  GAME_MECHANICS,
  STRATEGIES,
  GAME_CONFIG,
  BUILDING_COSTS,
  LAND_COSTS,
  VPREP_CONFIG,
};
