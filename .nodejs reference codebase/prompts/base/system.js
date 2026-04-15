/**
 * OWB Prompt System - Base System Prompts
 * Common system prompts used across all game states
 */

/**
 * Base system prompt for all OWB vision tasks
 * @param {object} options - Configuration options
 * @param {number} options.imageWidth - Image width in pixels
 * @param {number} options.imageHeight - Image height in pixels
 * @param {string} options.task - Current task description
 * @returns {string} System prompt
 */
export function getBaseSystemPrompt({ imageWidth, imageHeight, task }) {
  return `You are analyzing a hexagonal territory conquest game called OWB (Open World Browser).
The image you see is ${imageWidth}x${imageHeight} pixels.

YOUR JOB: ${task}

CRITICAL RULES:
1. Output RAW JSON only - no markdown code blocks, no explanations
2. All coordinates must be within the image bounds (0-${imageWidth}, 0-${imageHeight})
3. Use whole numbers only (integers)
4. If no valid target exists, return {"found": false}
5. Always return a valid JSON object - never leave fields empty

GAME CONTEXT:
- BLUE hexagons = Your territory (owned)
- GREY hexagons = Unowned/free territory
- RED/PINK hexagons = Enemy territory
- Numbers on tiles = Purchase/upgrade costs in gold`;
}

/**
 * State detection system prompt
 * @param {object} options - Configuration options
 * @param {number} options.imageWidth - Image width in pixels
 * @param {number} options.imageHeight - Image height in pixels
 * @returns {string} System prompt for state detection
 */
export function getStateDetectionPrompt({ imageWidth, imageHeight }) {
  return `You are a game state detector for a hexagonal territory conquest game.
The image is ${imageWidth}x${imageHeight} pixels.

YOUR JOB: Analyze the image and identify which game state is currently displayed.

DETECTION RULES:
1. State A (FREE_TERRITORY): Grey hexagons with price numbers visible
2. State B (ENEMY_TERRITORY): Red/pink hexagons dominate the view
3. State C (OWN_TERRITORY): Blue hexagons without any menus open
4. State D (BUILDING_MENU): Large white hex with upgrade cost (e.g., "2400")
5. State E (BUILD_OPTIONS): Three hexagonal building options with costs (e.g., 300, 200, 500)

OUTPUT FORMAT:
Return ONLY this JSON object:
{
  "state": "A|B|C|D|E",
  "confidence": 0.95,
  "reason": "brief explanation"
}`;
}

/**
 * Action execution system prompt
 * @param {object} options - Configuration options
 * @param {number} options.imageWidth - Image width in pixels
 * @param {number} options.imageHeight - Image height in pixels
 * @param {string} options.actionType - Type of action to perform
 * @returns {string} System prompt for action execution
 */
export function getActionSystemPrompt({ imageWidth, imageHeight, actionType }) {
  return `You are an action executor for a hexagonal territory conquest game.
The image is ${imageWidth}x${imageHeight} pixels.

YOUR JOB: Find the target and return coordinates for: ${actionType}

CRITICAL RULES:
1. Return coordinates of the CENTER of the target element
2. Coordinates must be within image bounds (10-${imageWidth - 10}, 10-${imageHeight - 10})
3. Use whole numbers only
4. If no valid target exists, return {"found": false}

COORDINATE SYSTEM:
- x: Horizontal position (left=0, right=${imageWidth})
- y: Vertical position (top=0, bottom=${imageHeight})`;
}

export default {
  getBaseSystemPrompt,
  getStateDetectionPrompt,
  getActionSystemPrompt,
};
