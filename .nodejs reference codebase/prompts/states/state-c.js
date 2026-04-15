/**
 * OWB Prompt System - State C: Own Territory
 * Detection and action for building on empty blue hexes
 */

import { getBaseSystemPrompt, getActionSystemPrompt } from "../base/system.js";
import { getTileVisualGuide } from "../base/visual-guide.js";
import { formatCoordinateConstraints } from "../shared/coordinates.js";
import {
  validateCoordinateResponse,
  parseLLMJson,
} from "../shared/validation.js";

/**
 * State C identifier and description
 */
export const STATE_C = {
  key: "C",
  name: "OWN_TERRITORY",
  description: "Blue owned territory without buildings",
  action: "Build on empty blue hex",
};

/**
 * Detection prompt - identifies if current state is State C
 * @param {number} imageWidth - Image width
 * @param {number} imageHeight - Image height
 * @returns {object} Prompt with system and user messages
 */
export function getDetectionPrompt(imageWidth, imageHeight) {
  const systemPrompt = getBaseSystemPrompt({
    imageWidth,
    imageHeight,
    task: "Identify if this is State C (Own Territory - blue hexes without menus)",
  });

  const userPrompt = `
<YOUR JOB>
Check if this image shows State C: Own Territory with blue hexagons and no menus.

<DETECTION CRITERIA>
State C is present when you see:
1. BLUE hexagons (your territory) visible
2. NO building menus or upgrade menus open
3. NO grey purchasable tiles with prices visible
4. Empty blue hexes available for building

<OUTPUT FORMAT>
Return ONLY this JSON:
{
  "isStateC": true/false,
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
 * Action prompt - finds empty blue hex to build on
 * @param {number} vprepWidth - V-PREP image width
 * @param {number} vprepHeight - V-PREP image height
 * @param {object} viewport - Viewport dimensions {width, height}
 * @returns {object} Prompt with system and user messages
 */
export function getActionPrompt(vprepWidth, vprepHeight, _viewport) {
  const systemPrompt = getActionSystemPrompt({
    imageWidth: vprepWidth,
    imageHeight: vprepHeight,
    actionType: "Find and click an empty blue hex to open build menu",
  });

  const userPrompt = `
<YOUR JOB>
Find ONE empty blue hex tile (your territory without a building).
Click it to open the building menu.

${getTileVisualGuide()}

${formatCoordinateConstraints(vprepWidth, vprepHeight, 50)}

<TARGET (click these)>
- BLUE hexagons (your territory)
- Must be EMPTY (no building icon on it)
- Prefer hexes near the edge of your territory

<IGNORE>
- Blue hexes with buildings already on them
- Grey hexes (unowned)
- Red hexes (enemy)
- Any menu overlays

<HOW TO FIND - 3 CHECKS>
CHECK 1: Find blue tiles
Look for hexes with blue coloring (your territory).

CHECK 2: Check if empty
Does the blue hex have any building icon on it? If no, it's empty.

CHECK 3: Return the hex center
Give the CENTER of the EMPTY BLUE HEX.

<OUTPUT FORMAT>
If found - return coordinates of the empty blue hex center:
{"x": <hex_center_x>, "y": <hex_center_y>, "found": true, "target": "empty_blue_hex"}

If no empty blue hex found:
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
 * Validate State C action response
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

  validation.parsed.target = "empty_blue_hex";

  return validation;
}

/**
 * Get complete State C prompt package
 * @param {object} options - Prompt options
 * @returns {object} Complete prompt package
 */
export function getStateCPrompt(options = {}) {
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
  STATE_C,
  getDetectionPrompt,
  getActionPrompt,
  validateResponse,
  getStateCPrompt,
};
