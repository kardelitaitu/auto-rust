/**
 * OWB Prompt System - State E: Build Options
 * Detection and action for three building choices menu
 */

import { getBaseSystemPrompt, getActionSystemPrompt } from "../base/system.js";
import {
  getMenuVisualGuide,
  BUILDING_ICONS as _BUILDING_ICONS,
} from "../base/visual-guide.js";
import { formatCoordinateConstraints } from "../shared/coordinates.js";
import {
  validateCoordinateResponse,
  parseLLMJson,
} from "../shared/validation.js";

/**
 * State E identifier and description
 */
export const STATE_E = {
  key: "E",
  name: "BUILD_OPTIONS",
  description: "Three building options menu",
  action: "Select building to construct",
};

/**
 * Detection prompt - identifies if current state is State E
 * @param {number} imageWidth - Image width
 * @param {number} imageHeight - Image height
 * @returns {object} Prompt with system and user messages
 */
export function getDetectionPrompt(imageWidth, imageHeight) {
  const systemPrompt = getBaseSystemPrompt({
    imageWidth,
    imageHeight,
    task: "Identify if this is State E (Build Options with three hexagonal choices)",
  });

  const userPrompt = `
<YOUR JOB>
Check if this image shows State E: Build Options Menu with three hexagonal choices.

<DETECTION CRITERIA>
State E is present when you see:
1. THREE hexagonal options arranged in a triangle/row
2. Each has a unique icon and cost number
3. Costs are typically: 300, 200, 500 gold
4. Menu overlay on the game map

<OUTPUT FORMAT>
Return ONLY this JSON:
{
  "isStateE": true/false,
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
 * Action prompt - selects a building from the three options
 * @param {number} vprepWidth - V-PREP image width
 * @param {number} vprepHeight - V-PREP image height
 * @param {object} viewport - Viewport dimensions {width, height}
 * @param {number} gold - Current gold amount
 * @param {string} strategy - Building strategy ('balanced', 'defensive', 'offensive')
 * @returns {object} Prompt with system and user messages
 */
export function getActionPrompt(
  vprepWidth,
  vprepHeight,
  viewport,
  gold = 0,
  strategy = "balanced",
) {
  const systemPrompt = getActionSystemPrompt({
    imageWidth: vprepWidth,
    imageHeight: vprepHeight,
    actionType: "Select the best affordable building from three options",
  });

  const userPrompt = `
<YOUR JOB>
Analyze the three building options and select the best one you can afford.

${getMenuVisualGuide()}

${formatCoordinateConstraints(vprepWidth, vprepHeight, 50)}

<CURRENT GOLD>
You have: ${gold} gold

<STRATEGY>
Preferred strategy: ${strategy}

<BUILDING OPTIONS>
1. DEFENSIVE (Left hex, ~300 gold)
   - Shield/fortress icon
   - Good for protecting territory
   
2. MELEE (Center hex, ~200 gold)
   - Sword/arrow icon
   - Offensive building, cheaper
   
3. HEALER (Right hex, ~500 gold)
   - Plus/cross icon
   - Support building, most expensive

<SELECTION LOGIC>
1. First, identify all three options and their costs
2. Filter to only affordable options (cost <= ${gold})
3. Select based on strategy:
   - "balanced": Prefer MELEE (200) if affordable, else DEFENSIVE (300)
   - "defensive": Prefer DEFENSIVE (300) if affordable, else cheapest
   - "offensive": Prefer MELEE (200) if affordable, else cheapest
4. If none affordable, return not found

<OUTPUT FORMAT>
If affordable building found:
{"x": <hex_center_x>, "y": <hex_center_y>, "found": true, "building": "defensive|melee|healer", "cost": <cost>}

If none affordable:
{"x": 0, "y": 0, "found": false, "building": null, "cost": null}

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
 * Validate State E action response
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

  // Check if no affordable building
  if (parsed.found === false) {
    return {
      valid: true,
      errors: [],
      parsed: { found: false, x: 0, y: 0, building: null, cost: null },
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

  // Validate building type
  const validBuildings = ["defensive", "melee", "healer"];
  if (parsed.building && !validBuildings.includes(parsed.building)) {
    validation.errors.push(`Invalid building type: ${parsed.building}`);
    validation.valid = false;
  }

  validation.parsed.building = parsed.building || null;
  validation.parsed.cost = parsed.cost || null;

  return validation;
}

/**
 * Get complete State E prompt package
 * @param {object} options - Prompt options
 * @returns {object} Complete prompt package
 */
export function getStateEPrompt(options = {}) {
  const {
    vprepWidth = 640,
    vprepHeight = 360,
    viewportWidth = 1280,
    viewportHeight = 720,
    gold = 0,
    strategy = "balanced",
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
    strategy,
  );
}

export default {
  STATE_E,
  getDetectionPrompt,
  getActionPrompt,
  validateResponse,
  getStateEPrompt,
};
