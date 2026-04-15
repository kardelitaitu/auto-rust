/**
 * OWB Prompt System - Validation Rules
 * JSON output validation and parsing utilities
 */

/**
 * Validate state detection response
 * @param {object} response - LLM response to validate
 * @returns {object} Validation result {valid, errors, parsed}
 */
export function validateStateDetection(response) {
  const errors = [];
  const validStates = [
    "A",
    "B",
    "C",
    "D",
    "E",
    "FREE_TERRITORY",
    "ENEMY_TERRITORY",
    "OWN_TERRITORY",
    "BUILDING_MENU",
    "BUILD_OPTIONS",
  ];

  if (!response) {
    return {
      valid: false,
      errors: ["Response is null or undefined"],
      parsed: null,
    };
  }

  // Check for state field
  if (!response.state) {
    errors.push('Missing "state" field');
  } else if (!validStates.includes(response.state)) {
    errors.push(
      `Invalid state: ${response.state}. Must be one of: ${validStates.join(", ")}`,
    );
  }

  // Check confidence (optional but should be valid if present)
  if (response.confidence !== undefined) {
    if (
      typeof response.confidence !== "number" ||
      response.confidence < 0 ||
      response.confidence > 1
    ) {
      errors.push("Confidence must be a number between 0 and 1");
    }
  }

  // Normalize state to single letter
  const stateMap = {
    FREE_TERRITORY: "A",
    ENEMY_TERRITORY: "B",
    OWN_TERRITORY: "C",
    BUILDING_MENU: "D",
    BUILD_OPTIONS: "E",
  };

  const normalizedState = stateMap[response.state] || response.state;

  return {
    valid: errors.length === 0,
    errors,
    parsed:
      errors.length === 0
        ? {
            state: normalizedState,
            confidence: response.confidence || 0.5,
            reason: response.reason || "No reason provided",
          }
        : null,
  };
}

/**
 * Validate coordinate response
 * @param {object} response - LLM response to validate
 * @param {object} options - Validation options
 * @param {number} options.width - Image/viewport width
 * @param {number} options.height - Image/viewport height
 * @param {number} options.margin - Safe margin from edges
 * @returns {object} Validation result {valid, errors, parsed}
 */
export function validateCoordinateResponse(
  response,
  { width, height, margin = 10 },
) {
  const errors = [];

  if (!response) {
    return {
      valid: false,
      errors: ["Response is null or undefined"],
      parsed: null,
    };
  }

  // Check for found flag
  if (response.found === false) {
    return {
      valid: true,
      errors: [],
      parsed: { found: false, x: 0, y: 0 },
    };
  }

  // Check x coordinate
  if (response.x === undefined) {
    errors.push('Missing "x" coordinate');
  } else if (typeof response.x !== "number") {
    errors.push(`x must be a number, got ${typeof response.x}`);
  } else if (isNaN(response.x)) {
    errors.push("x is NaN");
  } else if (response.x < margin || response.x > width - margin) {
    errors.push(`x=${response.x} out of bounds (${margin}-${width - margin})`);
  }

  // Check y coordinate
  if (response.y === undefined) {
    errors.push('Missing "y" coordinate');
  } else if (typeof response.y !== "number") {
    errors.push(`y must be a number, got ${typeof response.y}`);
  } else if (isNaN(response.y)) {
    errors.push("y is NaN");
  } else if (response.y < margin || response.y > height - margin) {
    errors.push(`y=${response.y} out of bounds (${margin}-${height - margin})`);
  }

  return {
    valid: errors.length === 0,
    errors,
    parsed:
      errors.length === 0
        ? {
            found: true,
            x: Math.round(response.x),
            y: Math.round(response.y),
            price: response.price || null,
            target: response.target || null,
          }
        : null,
  };
}

/**
 * Validate action response
 * @param {object} response - LLM response to validate
 * @returns {object} Validation result {valid, errors, parsed}
 */
export function validateActionResponse(response) {
  const errors = [];
  const validActions = [
    "clickAt",
    "click",
    "drag",
    "type",
    "wait",
    "done",
    "none",
  ];

  if (!response) {
    return {
      valid: false,
      errors: ["Response is null or undefined"],
      parsed: null,
    };
  }

  // Check for done/none action
  if (response.action === "done" || response.action === "none") {
    return {
      valid: true,
      errors: [],
      parsed: {
        action: response.action,
        rationale: response.rationale || "No action needed",
      },
    };
  }

  // Check action field
  if (!response.action) {
    errors.push('Missing "action" field');
  } else if (!validActions.includes(response.action)) {
    errors.push(
      `Invalid action: ${response.action}. Must be one of: ${validActions.join(", ")}`,
    );
  }

  // Check coordinates for click actions
  if (["clickAt", "click"].includes(response.action)) {
    if (response.x === undefined || response.y === undefined) {
      errors.push("Click actions require x and y coordinates");
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    parsed:
      errors.length === 0
        ? {
            action: response.action,
            x: response.x,
            y: response.y,
            rationale: response.rationale || "No rationale provided",
          }
        : null,
  };
}

/**
 * Parse JSON from LLM response, handling common formatting issues
 * @param {string|object} rawText - Raw LLM response text or object
 * @returns {object|null} Parsed JSON or null
 */
export function parseLLMJson(rawText) {
  if (!rawText) return null;

  // If already an object, return it directly
  if (typeof rawText === "object") {
    return rawText;
  }

  // Ensure we have a string
  if (typeof rawText !== "string") {
    try {
      rawText = String(rawText);
    } catch (_e) {
      return null;
    }
  }

  let text = rawText.trim();

  // Remove markdown code blocks if present
  text = text.replace(/```json\s*/g, "").replace(/```\s*$/g, "");

  // Remove any text before the first {
  const firstBrace = text.indexOf("{");
  if (firstBrace === -1) return null;
  text = text.substring(firstBrace);

  // Try to find closing brace - if not found, we'll attempt repair
  const lastBrace = text.lastIndexOf("}");
  if (lastBrace !== -1) {
    text = text.substring(0, lastBrace + 1);
  }

  try {
    return JSON.parse(text);
  } catch (e) {
    // Attempt to repair common JSON issues
    try {
      let repaired = text;

      // Trim trailing whitespace/newlines
      repaired = repaired.trimEnd();

      // Convert single quotes to double quotes for property names
      repaired = repaired.replace(/([{,]\s*)'([^']+)'\s*:/g, '$1"$2":');
      // Convert single quotes for string values
      repaired = repaired.replace(/:\s*'([^']*)'/g, ': "$1"');
      // Handle arrays with single-quoted strings
      repaired = repaired.replace(/\['([^']*)'\]/g, '["$1"]');

      // Fix unquoted property names
      repaired = repaired.replace(
        /([{,]\s*)([a-zA-Z_][a-zA-Z0-9_]*)\s*:/g,
        '$1"$2":',
      );

      // Remove trailing commas before } or ]
      repaired = repaired.replace(/,\s*([}\]])/g, "$1");

      // Balance quotes
      const quoteCount = (repaired.match(/"/g) || []).filter(
        (c) => c !== '\\"',
      ).length;
      if (quoteCount % 2 !== 0) repaired += '"';

      // Balance brackets first
      const openBrackets = (repaired.match(/\[/g) || []).length;
      const closeBrackets = (repaired.match(/\]/g) || []).length;
      for (let i = 0; i < openBrackets - closeBrackets; i++) {
        repaired += "]";
      }

      // Balance braces
      const openBraces = (repaired.match(/{/g) || []).length;
      const closeBraces = (repaired.match(/}/g) || []).length;
      for (let i = 0; i < openBraces - closeBraces; i++) {
        repaired += "}";
      }

      // Fix truncated values
      if (repaired.match(/:\s*$/)) repaired += '""';

      return JSON.parse(repaired);
    } catch (repairError) {
      console.error("Failed to parse JSON:", e.message);
      console.error("Repair also failed:", repairError.message);
      return null;
    }
  }
}

/**
 * Create validation result object
 * @param {boolean} valid - Whether validation passed
 * @param {string[]} errors - Array of error messages
 * @param {object} parsed - Parsed data if valid
 * @returns {object} Validation result
 */
export function createValidationResult(valid, errors = [], parsed = null) {
  return { valid, errors, parsed };
}

export default {
  validateStateDetection,
  validateCoordinateResponse,
  validateActionResponse,
  parseLLMJson,
  createValidationResult,
};
