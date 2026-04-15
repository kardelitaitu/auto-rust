/**
 * OWB Prompt System - Index
 * Central export for all prompt modules
 */

// Base prompts
import {
  getBaseSystemPrompt,
  getStateDetectionPrompt,
  getActionSystemPrompt,
} from "./base/system.js";
import {
  TILE_COLORS,
  PRICE_NUMBERS,
  BUILDING_ICONS,
  getTileVisualGuide,
  getMenuVisualGuide,
  getCompleteVisualReference,
} from "./base/visual-guide.js";

// Shared utilities
import {
  scaleToViewport,
  scaleToVPrep,
  validateCoordinates,
  formatCoordinateConstraints,
  parseCoordinates,
  calculateDistance,
  getImageCenter,
} from "./shared/coordinates.js";
import {
  validateStateDetection,
  validateCoordinateResponse,
  validateActionResponse,
  parseLLMJson,
  createValidationResult,
} from "./shared/validation.js";

// State prompts
import stateA, {
  STATE_A,
  getStateAPrompt,
  validateResponse as validateStateA,
} from "./states/state-a.js";
import stateB, {
  STATE_B,
  getStateBPrompt,
  validateResponse as validateStateB,
} from "./states/state-b.js";
import stateC, {
  STATE_C,
  getStateCPrompt,
  validateResponse as validateStateC,
} from "./states/state-c.js";
import stateD, {
  STATE_D,
  getStateDPrompt,
  validateResponse as validateStateD,
} from "./states/state-d.js";
import stateE, {
  STATE_E,
  getStateEPrompt,
  validateResponse as validateStateE,
} from "./states/state-e.js";

/**
 * All game states
 */
export const STATES = {
  A: STATE_A,
  B: STATE_B,
  C: STATE_C,
  D: STATE_D,
  E: STATE_E,
};

/**
 * State prompt getters by key
 */
const statePromptGetters = {
  A: getStateAPrompt,
  B: getStateBPrompt,
  C: getStateCPrompt,
  D: getStateDPrompt,
  E: getStateEPrompt,
};

/**
 * State validators by key
 */
const stateValidators = {
  A: validateStateA,
  B: validateStateB,
  C: validateStateC,
  D: validateStateD,
  E: validateStateE,
};

/**
 * Get prompt for a specific state
 * @param {string} stateKey - State key (A, B, C, D, E)
 * @param {object} options - Prompt options
 * @returns {object} Prompt with messages array
 */
export function getStatePrompt(stateKey, options = {}) {
  const getter = statePromptGetters[stateKey];
  if (!getter) {
    throw new Error(
      `Unknown state: ${stateKey}. Valid states: ${Object.keys(STATES).join(", ")}`,
    );
  }
  return getter(options);
}

/**
 * Validate response for a specific state
 * @param {string} stateKey - State key (A, B, C, D, E)
 * @param {string} rawResponse - Raw LLM response
 * @param {object} options - Validation options
 * @returns {object} Validation result
 */
export function validateStateResponse(stateKey, rawResponse, options = {}) {
  const validator = stateValidators[stateKey];
  if (!validator) {
    throw new Error(
      `Unknown state: ${stateKey}. Valid states: ${Object.keys(STATES).join(", ")}`,
    );
  }
  return validator(rawResponse, options);
}

/**
 * Get state detection prompt (wrapper for base system prompt)
 * @param {number} imageWidth - Image width
 * @param {number} imageHeight - Image height
 * @returns {object} Detection prompt
 */
export function getStateDetectionPromptWrapper(imageWidth, imageHeight) {
  return getStateDetectionPrompt({ imageWidth, imageHeight });
}

/**
 * Detect game state from LLM response
 * @param {string} rawResponse - Raw LLM response
 * @returns {object} Detected state info
 */
export function detectStateFromResponse(rawResponse) {
  const parsed = parseLLMJson(rawResponse);
  if (!parsed) {
    return { state: "A", confidence: 0, reason: "Failed to parse response" };
  }

  // Normalize state
  const stateMap = {
    FREE_TERRITORY: "A",
    ENEMY_TERRITORY: "B",
    OWN_TERRITORY: "C",
    BUILDING_MENU: "D",
    BUILD_OPTIONS: "E",
  };

  const state = stateMap[parsed.state] || parsed.state || "A";

  return {
    state,
    confidence: parsed.confidence || 0.5,
    reason: parsed.reason || "No reason provided",
  };
}

/**
 * Get all state names
 * @returns {string[]} Array of state keys
 */
export function getAllStateKeys() {
  return Object.keys(STATES);
}

/**
 * Get state info by key
 * @param {string} stateKey - State key
 * @returns {object} State info
 */
export function getStateInfo(stateKey) {
  return STATES[stateKey] || null;
}

// Re-export everything for convenience
export {
  // Base
  getBaseSystemPrompt,
  getStateDetectionPrompt as getBaseStateDetectionPrompt,
  getActionSystemPrompt,
  TILE_COLORS,
  PRICE_NUMBERS,
  BUILDING_ICONS,
  getTileVisualGuide,
  getMenuVisualGuide,
  getCompleteVisualReference,

  // Shared
  scaleToViewport,
  scaleToVPrep,
  validateCoordinates,
  formatCoordinateConstraints,
  parseCoordinates,
  calculateDistance,
  getImageCenter,
  validateStateDetection,
  validateCoordinateResponse,
  validateActionResponse,
  parseLLMJson,
  createValidationResult,

  // States
  stateA,
  stateB,
  stateC,
  stateD,
  stateE,
  getStateAPrompt,
  getStateBPrompt,
  getStateCPrompt,
  getStateDPrompt,
  getStateEPrompt,
};

export default {
  STATES,
  getStatePrompt,
  validateStateResponse,
  getStateDetectionPrompt,
  detectStateFromResponse,
  getAllStateKeys,
  getStateInfo,
};
