/**
 * OWB Prompt System - Coordinate Helpers
 * Utilities for coordinate formatting, scaling, and validation
 */

/**
 * Scale coordinates from V-PREP space to browser viewport space
 * @param {number} x - X coordinate in V-PREP space
 * @param {number} y - Y coordinate in V-PREP space
 * @param {object} options - Scaling options
 * @param {number} options.vprepWidth - V-PREP output width
 * @param {number} options.vprepHeight - V-PREP output height
 * @param {number} options.viewportWidth - Browser viewport width
 * @param {number} options.viewportHeight - Browser viewport height
 * @returns {object} Scaled coordinates {x, y}
 */
export function scaleToViewport(
  x,
  y,
  { vprepWidth, vprepHeight, viewportWidth, viewportHeight },
) {
  const scaleX = viewportWidth / vprepWidth;
  const scaleY = viewportHeight / vprepHeight;

  return {
    x: Math.round(x * scaleX),
    y: Math.round(y * scaleY),
    scale: { x: scaleX, y: scaleY },
  };
}

/**
 * Scale coordinates from browser viewport to V-PREP space
 * @param {number} x - X coordinate in viewport space
 * @param {number} y - Y coordinate in viewport space
 * @param {object} options - Scaling options
 * @returns {object} Scaled coordinates {x, y}
 */
export function scaleToVPrep(
  x,
  y,
  { vprepWidth, vprepHeight, viewportWidth, viewportHeight },
) {
  const scaleX = vprepWidth / viewportWidth;
  const scaleY = vprepHeight / viewportHeight;

  return {
    x: Math.round(x * scaleX),
    y: Math.round(y * scaleY),
    scale: { x: scaleX, y: scaleY },
  };
}

/**
 * Validate coordinates are within bounds
 * @param {number} x - X coordinate
 * @param {number} y - Y coordinate
 * @param {object} options - Validation options
 * @param {number} options.width - Image/viewport width
 * @param {number} options.height - Image/viewport height
 * @param {number} options.margin - Margin from edges (default: 10)
 * @returns {boolean} True if valid
 */
export function validateCoordinates(x, y, { width, height, margin = 10 }) {
  if (typeof x !== "number" || typeof y !== "number") {
    return false;
  }
  if (isNaN(x) || isNaN(y)) {
    return false;
  }
  if (x < margin || x > width - margin) {
    return false;
  }
  if (y < margin || y > height - margin) {
    return false;
  }
  return true;
}

/**
 * Format coordinates for prompt injection
 * @param {number} width - Image width
 * @param {number} height - Image height
 * @param {number} margin - Safe margin from edges
 * @returns {string} Formatted coordinate constraints
 */
export function formatCoordinateConstraints(width, height, margin = 50) {
  return `
<COORDINATE CONSTRAINTS>
- Image size: ${width}x${height} pixels
- Valid x range: ${margin} to ${width - margin}
- Valid y range: ${margin} to ${height - margin}
- Use whole numbers only (integers)
- Target the CENTER of elements
`.trim();
}

/**
 * Parse LLM response coordinates from various formats
 * @param {object} llmResult - Raw LLM response
 * @returns {object|null} Parsed coordinates {x, y} or null
 */
export function parseCoordinates(llmResult) {
  if (!llmResult) return null;

  // Direct x,y properties
  if (llmResult.x !== undefined && llmResult.y !== undefined) {
    return { x: llmResult.x, y: llmResult.y };
  }

  // Array format [{x, y}]
  if (Array.isArray(llmResult) && llmResult.length > 0) {
    const first = llmResult[0];
    if (first.x !== undefined && first.y !== undefined) {
      return { x: first.x, y: first.y };
    }
  }

  // Coordinates array format {coordinates: [{x, y}]}
  if (
    llmResult.coordinates &&
    Array.isArray(llmResult.coordinates) &&
    llmResult.coordinates.length > 0
  ) {
    const first = llmResult.coordinates[0];
    if (first.x !== undefined && first.y !== undefined) {
      return { x: first.x, y: first.y };
    }
  }

  // Action format {action: 'clickAt', x, y}
  if (
    llmResult.action === "clickAt" &&
    llmResult.x !== undefined &&
    llmResult.y !== undefined
  ) {
    return { x: llmResult.x, y: llmResult.y };
  }

  return null;
}

/**
 * Calculate distance between two points
 * @param {number} x1 - First point X
 * @param {number} y1 - First point Y
 * @param {number} x2 - Second point X
 * @param {number} y2 - Second point Y
 * @returns {number} Distance in pixels
 */
export function calculateDistance(x1, y1, x2, y2) {
  return Math.sqrt(Math.pow(x2 - x1, 2) + Math.pow(y2 - y1, 2));
}

/**
 * Get center of image
 * @param {number} width - Image width
 * @param {number} height - Image height
 * @returns {object} Center coordinates {x, y}
 */
export function getImageCenter(width, height) {
  return {
    x: Math.round(width / 2),
    y: Math.round(height / 2),
  };
}

export default {
  scaleToViewport,
  scaleToVPrep,
  validateCoordinates,
  formatCoordinateConstraints,
  parseCoordinates,
  calculateDistance,
  getImageCenter,
};
