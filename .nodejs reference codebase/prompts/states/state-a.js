/**
 * OWB Prompt System - State A: Free Territory
 * Detection and action for purchasing grey unowned land
 */

import { getBaseSystemPrompt, getActionSystemPrompt } from "../base/system.js";
import { getTileVisualGuide } from "../base/visual-guide.js";
import {
  formatCoordinateConstraints,
  validateCoordinates as _validateCoordinates,
} from "../shared/coordinates.js";
import {
  validateCoordinateResponse,
  parseLLMJson,
} from "../shared/validation.js";

/**
 * State A identifier and description
 */
export const STATE_A = {
  key: "A",
  name: "FREE_TERRITORY",
  description: "Grey free land with price numbers",
  action: "Buy land adjacent to blue territory",
};

/**
 * Detection prompt - identifies if current state is State A
 * @param {number} imageWidth - Image width
 * @param {number} imageHeight - Image height
 * @returns {object} Prompt with system and user messages
 */
export function getDetectionPrompt(imageWidth, imageHeight) {
  const systemPrompt = getBaseSystemPrompt({
    imageWidth,
    imageHeight,
    task: "Identify if this is State A (Free Territory with purchasable grey tiles)",
  });

  const userPrompt = `
<YOUR JOB>
Check if this image shows State A: Free Territory with purchasable grey tiles.

<DETECTION CRITERIA>
State A is present when you see:
1. GREY hexagons with WHITE price numbers (like "Free","40", "160", "1200", "2400")
2. BLUE hexagons (your territory) nearby
3. NO building menus or upgrade menus open

<OUTPUT FORMAT>
Return ONLY this JSON:
{
  "isStateA": true/false,
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
 * Action prompt - finds purchasable grey tile with price
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
      "Find and click the NUMBER on a grey hex tile that touches blue territory",
  });

  const userPrompt = `
<YOUR JOB>
Find ONE grey hex tile with a price number that TOUCHES a blue hex tile.
Return the coordinates of the NUMBER TEXT itself (not the tile center).

${getTileVisualGuide()}

${formatCoordinateConstraints(vprepWidth, vprepHeight, 50)}

<TARGET (click these)>
- Grey hexagon with WHITE number: "00", "50", "100", "200", "1200"
- Number is CENTERED in the hex
- Must TOUCH a blue hex (share an edge)

<IGNORE>
- Grey hexes WITHOUT numbers (not purchasable)
- Numbers that don't touch blue territory
- Red/pink hexes (enemy territory)

<HOW TO FIND - 3 CHECKS>
CHECK 1: Find grey tiles WITH numbers
Look for grey hexes with "50", "100", "200" etc written on them.

CHECK 2: Check if it touches blue
Does any edge of that grey tile touch a blue tile?

CHECK 3: Return the NUMBER's position
Give the CENTER of the NUMBER TEXT, not the tile.

<OUTPUT FORMAT>
If found - return coordinates of the NUMBER:
{"x": <number_center_x>, "y": <number_center_y>, "found": true, "price": "<number>"}

If not found:
{"x": 0, "y": 0, "found": false, "price": null}

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
 * Validate State A action response
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
      parsed: { found: false, x: 0, y: 0, price: null },
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

  // Add price to parsed result
  validation.parsed.price = parsed.price || null;
  validation.parsed.target = "grey_tile_number";

  return validation;
}

/**
 * Get complete State A prompt package
 * @param {object} options - Prompt options
 * @returns {object} Complete prompt package
 */
export function getStateAPrompt(options = {}) {
  const {
    vprepWidth = 640,
    vprepHeight = 360,
    viewportWidth = 1280,
    viewportHeight = 720,
    mode = "action", // 'detection' or 'action'
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
  STATE_A,
  getDetectionPrompt,
  getActionPrompt,
  validateResponse,
  getStateAPrompt,
};
