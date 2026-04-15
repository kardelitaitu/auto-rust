/**
 * OWB Prompt System - State B: Enemy Territory
 * Detection and action for attacking red enemy hexes
 */

import { getBaseSystemPrompt, getActionSystemPrompt } from "../base/system.js";
import { getTileVisualGuide } from "../base/visual-guide.js";
import { formatCoordinateConstraints } from "../shared/coordinates.js";
import {
  validateCoordinateResponse,
  parseLLMJson,
} from "../shared/validation.js";

/**
 * State B identifier and description
 */
export const STATE_B = {
  key: "B",
  name: "ENEMY_TERRITORY",
  description: "Red enemy territory visible",
  action: "Attack enemy hex adjacent to blue territory",
};

/**
 * Detection prompt - identifies if current state is State B
 * @param {number} imageWidth - Image width
 * @param {number} imageHeight - Image height
 * @returns {object} Prompt with system and user messages
 */
export function getDetectionPrompt(imageWidth, imageHeight) {
  const systemPrompt = getBaseSystemPrompt({
    imageWidth,
    imageHeight,
    task: "Identify if this is State B (Enemy Territory with red/pink hexes)",
  });

  const userPrompt = `
<YOUR JOB>
Check if this image shows State B: Enemy Territory with red/pink hexagons.

<DETECTION CRITERIA>
State B is present when you see:
1. RED or PINK hexagons (enemy territory)
2. These red hexes are ADJACENT to your BLUE territory
3. NO building menus or upgrade menus open

<OUTPUT FORMAT>
Return ONLY this JSON:
{
  "isStateB": true/false,
  "confidence": 0.0-1.0,
  "reason": "brief explanation"
}
`.trim();

  return {
    messages: [
      { role: "system", content: systemPrompt },
      { role: "user", content: userPrompt },
    ],
  };
}

/**
 * Action prompt - finds enemy hex to attack
 * @param {number} vprepWidth - V-PREP image width
 * @param {number} vprepHeight - V-PREP image height
 * @param {object} viewport - Viewport dimensions {width, height}
 * @returns {object} Prompt with system and user messages
 */
export function getActionPrompt(vprepWidth, vprepHeight, _viewport) {
  const systemPrompt = getActionSystemPrompt({
    imageWidth: vprepWidth,
    imageHeight: vprepHeight,
    actionType:
      "Find and click a red enemy hex that touches your blue territory",
  });

  const userPrompt = `
<YOUR JOB>
Find ONE red/pink enemy hex tile that TOUCHES your blue territory.
Return the coordinates of the CENTER of that red hex.

${getTileVisualGuide()}

${formatCoordinateConstraints(vprepWidth, vprepHeight, 50)}

<TARGET (click these)>
- RED or PINK hexagons (enemy territory)
- Must TOUCH a blue hex (share an edge)
- Prefer hexes that are strategically valuable

<IGNORE>
- Blue hexes (your territory)
- Grey hexes (unowned)
- Red hexes that don't touch your territory

<HOW TO FIND - 3 CHECKS>
CHECK 1: Find red/pink tiles
Look for hexes with red/pink coloring.

CHECK 2: Check if it touches blue
Does any edge of that red tile touch your blue territory?

CHECK 3: Return the hex center
Give the CENTER of the RED HEX.

<OUTPUT FORMAT>
If found - return coordinates of the red hex center:
{"x": <hex_center_x>, "y": <hex_center_y>, "found": true, "target": "enemy_hex"}

If not found:
{"x": 0, "y": 0, "found": false, "target": null}

IMPORTANT: Only return the JSON object, nothing else.
`.trim();

  return {
    messages: [
      { role: "system", content: systemPrompt },
      { role: "user", content: userPrompt },
    ],
  };
}

/**
 * Validate State B action response
 * @param {string} rawResponse - Raw LLM response
 * @param {object} options - Validation options
 * @returns {object} Validation result
 */
export function validateResponse(
  rawResponse,
  {
    vprepWidth,
    vprepHeight,
    viewportWidth: _viewportWidth,
    viewportHeight: _viewportHeight,
  },
) {
  const parsed = parseLLMJson(rawResponse);
  if (!parsed) {
    return { valid: false, errors: ["Failed to parse JSON"], parsed: null };
  }

  // Check if no target found
  if (parsed.found === false) {
    return {
      valid: true,
      errors: [],
      parsed: { found: false, x: 0, y: 0, target: null },
    };
  }

  // Validate coordinates in V-PREP space
  const validation = validateCoordinateResponse(parsed, {
    width: vprepWidth,
    height: vprepHeight,
    margin: 50,
  });

  if (!validation.valid) {
    return validation;
  }

  validation.parsed.target = "enemy_hex";

  return validation;
}

/**
 * Get complete State B prompt package
 * @param {object} options - Prompt options
 * @returns {object} Complete prompt package
 */
export function getStateBPrompt(options = {}) {
  const {
    vprepWidth = 640,
    vprepHeight = 360,
    viewportWidth = 1280,
    viewportHeight = 720,
    mode = "action",
  } = options;

  if (mode === "detection") {
    return getDetectionPrompt(viewportWidth, viewportHeight);
  }

  return getActionPrompt(vprepWidth, vprepHeight, {
    width: viewportWidth,
    height: viewportHeight,
  });
}

export default {
  STATE_B,
  getDetectionPrompt,
  getActionPrompt,
  validateResponse,
  getStateBPrompt,
};
