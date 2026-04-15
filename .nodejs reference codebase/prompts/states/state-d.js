/**
 * OWB Prompt System - State D: Building Menu
 * Detection and action for upgrade menu with large white hex and cost
 */

import { getBaseSystemPrompt, getActionSystemPrompt } from "../base/system.js";
import { getMenuVisualGuide } from "../base/visual-guide.js";
import { formatCoordinateConstraints } from "../shared/coordinates.js";
import {
  validateCoordinateResponse,
  parseLLMJson,
} from "../shared/validation.js";

/**
 * State D identifier and description
 */
export const STATE_D = {
  key: "D",
  name: "BUILDING_MENU",
  description: "Building upgrade menu open",
  action: "Click upgrade button or close menu",
};

/**
 * Detection prompt - identifies if current state is State D
 * @param {number} imageWidth - Image width
 * @param {number} imageHeight - Image height
 * @returns {object} Prompt with system and user messages
 */
export function getDetectionPrompt(imageWidth, imageHeight) {
  const systemPrompt = getBaseSystemPrompt({
    imageWidth,
    imageHeight,
    task: "Identify if this is State D (Building Upgrade Menu with large white hex)",
  });

  const userPrompt = `
<YOUR JOB>
Check if this image shows State D: Building Upgrade Menu.

<DETECTION CRITERIA>
State D is present when you see:
1. Large WHITE hexagon in center with building icon
2. Cost number displayed (e.g., "2400")
3. May show level indicator (small number like "1")
4. Upgrade button visible below

<OUTPUT FORMAT>
Return ONLY this JSON:
{
  "isStateD": true/false,
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
 * Action prompt - finds upgrade button or close button
 * @param {number} vprepWidth - V-PREP image width
 * @param {number} vprepHeight - V-PREP image height
 * @param {object} viewport - Viewport dimensions {width, height}
 * @param {number} gold - Current gold amount
 * @returns {object} Prompt with system and user messages
 */
export function getActionPrompt(vprepWidth, vprepHeight, viewport, gold = 0) {
  const systemPrompt = getActionSystemPrompt({
    imageWidth: vprepWidth,
    imageHeight: vprepHeight,
    actionType: "Click upgrade button if affordable, or close menu",
  });

  const userPrompt = `
<YOUR JOB>
Analyze the building upgrade menu and decide:
1. If you have enough gold (${gold}), click the UPGRADE button
2. If not enough gold, click outside the menu to close it

${getMenuVisualGuide()}

${formatCoordinateConstraints(vprepWidth, vprepHeight, 50)}

<CURRENT GOLD>
You have: ${gold} gold

<UPGRADE COST>
Look for the cost number on the large white hex (e.g., "2400")

<DECISION LOGIC>
- If gold >= cost: Click the UPGRADE button
- If gold < cost: Click outside the menu to close it

<TARGET - UPGRADE BUTTON>
- Located below the large white hex
- Usually a button with "UPGRADE" text or arrow icon
- Click its center

<TARGET - CLOSE MENU>
- Click anywhere outside the white hex menu
- Prefer clicking on blue territory
- Avoid clicking on the menu itself

<OUTPUT FORMAT>
If upgrading:
{"x": <button_x>, "y": <button_y>, "found": true, "action": "upgrade", "cost": <cost_number>}

If closing menu:
{"x": <outside_x>, "y": <outside_y>, "found": true, "action": "close", "cost": null}

If menu not found:
{"x": 0, "y": 0, "found": false, "action": null, "cost": null}

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
 * Validate State D action response
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

  // Check if menu not found
  if (parsed.found === false) {
    return {
      valid: true,
      errors: [],
      parsed: { found: false, x: 0, y: 0, action: null, cost: null },
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

  validation.parsed.action = parsed.action || "upgrade";
  validation.parsed.cost = parsed.cost || null;

  return validation;
}

/**
 * Get complete State D prompt package
 * @param {object} options - Prompt options
 * @returns {object} Complete prompt package
 */
export function getStateDPrompt(options = {}) {
  const {
    vprepWidth = 640,
    vprepHeight = 360,
    viewportWidth = 1280,
    viewportHeight = 720,
    gold = 0,
    mode = "action",
  } = options;

  if (mode === "detection") {
    return getDetectionPrompt(viewportWidth, viewportHeight);
  }

  return getActionPrompt(
    vprepWidth,
    vprepHeight,
    {
      width: viewportWidth,
      height: viewportHeight,
    },
    gold,
  );
}

export default {
  STATE_D,
  getDetectionPrompt,
  getActionPrompt,
  validateResponse,
  getStateDPrompt,
};
