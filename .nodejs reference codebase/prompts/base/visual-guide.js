/**
 * OWB Prompt System - Visual Guide Templates
 * Common visual elements and descriptions for the hexagonal territory game
 */

/**
 * Tile color definitions based on game screenshots
 */
export const TILE_COLORS = {
  OWN: {
    primary: "#4A90D9",
    description: "Solid blue colored hexagons",
    meaning: "Your territory - can build here",
  },
  FREE: {
    primary: "#808080",
    secondary: "#606060",
    description: "Grey/dark hexagons",
    meaning: "Unowned - purchasable if has price number",
  },
  ENEMY: {
    primary: "#FF6B6B",
    secondary: "#FF4444",
    description: "Red/pink colored hexagons",
    meaning: "Enemy territory - can attack",
  },
  MENU: {
    primary: "#FFFFFF",
    secondary: "#F0F0F0",
    description: "White/light colored hexagons",
    meaning: "Menu or UI element",
  },
};

/**
 * Price number visual characteristics
 */
export const PRICE_NUMBERS = {
  appearance: "White or light colored text centered in grey hex",
  examples: ["00", "50", "80", "100", "200", "1200"],
  location: "Centered within the hexagon",
  meaning: "Gold cost to purchase this tile",
};

/**
 * Building icons visual guide
 */
export const BUILDING_ICONS = {
  DEFENSIVE: {
    description: "Shield/fortress icon",
    cost: 300,
    color: "Blue/teal icon on white hex",
  },
  MELEE: {
    description: "Sword/arrow icon",
    cost: 200,
    color: "Blue/teal icon on white hex",
  },
  HEALER: {
    description: "Plus/cross icon",
    cost: 500,
    color: "Blue/teal icon on white hex",
  },
  UPGRADE: {
    description: "Upward chevrons icon",
    cost: 2400,
    color: "Blue icon on white hex with cost below",
  },
};

/**
 * Get visual guide for tile identification
 * @returns {string} Visual guide text
 */
export function getTileVisualGuide() {
  return `
<TILE VISUAL GUIDE>

BLUE TILES (YOUR TERRITORY):
- Solid blue colored hexagons (#4A90D9)
- These are YOUR owned territory
- You can BUILD on empty blue tiles
- Look for: Solid blue fill, no text

GREY TILES (FREE/UNOWNED):
- Grey/dark hexagons (#808080)
- WITH white number = PURCHASABLE (State A target)
- WITHOUT number = NOT purchasable (skip)
- Look for: Grey fill, white text if purchasable

RED/PINK TILES (ENEMY):
- Red or pink colored hexagons (#FF6B6B)
- These are ENEMY territory
- Can be ATTACKED if adjacent to your blue
- Look for: Red/pink fill

WHITE TILES (MENU/UI):
- Large white hexagons
- Usually part of building/upgrade menus
- Contains icons and cost numbers
- Look for: White fill, icons, large numbers
`.trim();
}

/**
 * Get visual guide for menu identification
 * @returns {string} Menu visual guide text
 */
export function getMenuVisualGuide() {
  return `
<MENU VISUAL GUIDE>

BUILDING MENU (State D):
- Large white hexagon in center
- Building icon at top
- Cost number below (e.g., "2400")
- May show level indicator (small number)
- Upgrade button visible

BUILD OPTIONS (State E):
- Three hexagonal options arranged in triangle
- Each has unique icon + cost:
  * Left: Defensive building (300 gold)
  * Center: Melee building (200 gold)
  * Right: Healer building (500 gold)
- Gold counter in bottom-left (e.g., "180")

GOLD COUNTER:
- Small hex icon with number
- Located in bottom-left corner
- Shows current gold amount
- Example: "180" means 180 gold available
`.trim();
}

/**
 * Get complete visual reference
 * @returns {string} Complete visual reference
 */
export function getCompleteVisualReference() {
  return `
${getTileVisualGuide()}

${getMenuVisualGuide()}

<COORDINATE REFERENCE>
- Image dimensions are provided in the prompt
- x: 0 = left edge, increases to right
- y: 0 = top edge, increases downward
- Target the CENTER of elements
- Stay within 10px margin from edges
`.trim();
}

export default {
  TILE_COLORS,
  PRICE_NUMBERS,
  BUILDING_ICONS,
  getTileVisualGuide,
  getMenuVisualGuide,
  getCompleteVisualReference,
};
