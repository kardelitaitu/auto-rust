# OWB Prompt System - Tuning Guide

## Overview

This guide explains how to tune and optimize the OWB prompt system for better game automation performance.

---

## Prompt Structure

### Directory Layout

```
prompts/
├── index.js                 # Central export and utilities
├── TUNING-GUIDE.md          # This file
├── base/
│   ├── system.js           # Base system prompts
│   └── visual-guide.js     # Visual element descriptions
├── states/
│   ├── state-a.js          # Free Territory (buy land)
│   ├── state-b.js          # Enemy Territory (attack)
│   ├── state-c.js          # Own Territory (build)
│   ├── state-d.js          # Building Menu (upgrade)
│   └── state-e.js          # Build Options (select)
└── shared/
    ├── coordinates.js      # Coordinate utilities
    └── validation.js       # Response validation
```

---

## Tuning Parameters

### 1. V-PREP Settings

Located in `owb-agents.js` and `api/utils/vision-preprocessor.js`:

```javascript
vprepConfig: {
    targetWidth: 640,      // Image width (lower = faster, less detail)
    targetHeight: 360,     // Image height
    contrast: 1.25,        // Contrast boost (1.0-2.0)
    brightness: 5,         // Brightness adjustment
    sharpness: 1.2,        // Sharpness filter
    edgeEnhance: true,     // Edge enhancement
    quality: 78,           // JPEG quality (1-100)
    autoROI: false,        // Auto region of interest
}
```

**Tuning Tips:**

- Increase `targetWidth` for better accuracy (slower)
- Decrease `targetWidth` for faster processing (less accurate)
- Increase `contrast` if tiles are hard to distinguish
- Enable `edgeEnhance` for better hex detection

### 2. LLM Settings

Located in `owb-config.js`:

```javascript
LLM_CONFIG: {
    defaultModel: 'gemma3:4b',    // Primary model
    fallbackModel: 'qwen2.5vl:3b', // Fallback model
    visionEnabled: true,
    maxTokens: 4096,              // Max response tokens
    contextLength: 8192,          // Context window
}
```

**Tuning Tips:**

- Use `gemma3:4b` for reliable JSON output
- Use `qwen2.5vl:3b` for better vision (but less reliable JSON)
- Increase `maxTokens` for complex prompts
- Increase `contextLength` for longer visual context

### 3. Coordinate Margins

In prompt files, adjust margin values:

```javascript
// In state-*.js files
formatCoordinateConstraints(vprepWidth, vprepHeight, 50); // 50px margin
```

**Tuning Tips:**

- Increase margin if clicks are near edges
- Decrease margin to allow more click area
- Typical range: 30-70 pixels

---

## State-Specific Tuning

### State A: Free Territory

**Common Issues:**

- LLM returns wrong coordinates
- LLM clicks on grey tiles without numbers
- LLM clicks on red tiles

**Tuning:**

1. Check visual guide in `prompts/base/visual-guide.js`
2. Update color definitions if game colors changed
3. Adjust price number examples in prompt
4. Increase margin if clicks are off-center

**Example Adjustment:**

```javascript
// In prompts/states/state-a.js
// Add more specific visual descriptions
GREY TILES WITH NUMBER (TARGET):
- Grey hexagon with WHITE number: "Free", "40", "160", "1200", "2400"
- Number is CENTERED in the hex
- Must TOUCH a blue hex (share an edge)
- Number color is BRIGHT WHITE (#FFFFFF)
```

### State B: Enemy Territory

**Common Issues:**

- LLM doesn't find red tiles
- LLM clicks on blue tiles instead

**Tuning:**

1. Verify red color definition matches game
2. Add more specific adjacency rules
3. Adjust confidence threshold

### State C: Own Territory

**Common Issues:**

- LLM clicks on tiles with buildings
- LLM doesn't find empty tiles

**Tuning:**

1. Clarify "empty" vs "occupied" distinction
2. Add building icon descriptions
3. Adjust targeting preference (edge vs center)

### State D: Building Menu

**Common Issues:**

- LLM doesn't recognize upgrade button
- LLM clicks wrong area

**Tuning:**

1. Add more specific button location hints
2. Include cost number in detection
3. Adjust close menu coordinates

### State E: Build Options

**Common Issues:**

- LLM selects wrong building
- LLM can't distinguish the three options

**Tuning:**

1. Add position hints (left/center/right)
2. Include icon descriptions
3. Adjust strategy logic

---

## Testing & Validation

### Using qwen-tester.js

```bash
# Test current prompts
node qwen-tester.js

# Test specific state
# Modify tester to use specific state prompt
```

### Debug Screenshots

Check `logs/debug-*.png` files to see:

- What the LLM sees (V-PREP output)
- Where the LLM clicked
- Result of the action

### Validation Metrics

Track these metrics in logs:

- Detection accuracy (correct state identified)
- Action success rate (click hit target)
- Response time (LLM inference duration)

---

## Common Tuning Scenarios

### Scenario 1: LLM Returns Wrong Coordinates

**Symptoms:**

- Clicks are outside valid area
- Clicks miss the target

**Solutions:**

1. Check V-PREP output dimensions
2. Verify coordinate scaling factor
3. Increase coordinate margins
4. Add explicit bounds in prompt

### Scenario 2: State Detection Fails

**Symptoms:**

- Wrong state detected
- Low confidence scores

**Solutions:**

1. Update visual descriptions
2. Add more detection criteria
3. Adjust confidence threshold
4. Check for game UI changes

### Scenario 3: LLM Ignores Instructions

**Symptoms:**

- Returns wrong JSON format
- Includes extra text
- Missing required fields

**Solutions:**

1. Simplify prompt
2. Add more explicit output format
3. Use XML delimiters
4. Add "IMPORTANT: Only return JSON" reminder

---

## Prompt Template Variables

Available variables in state prompts:

| Variable         | Description             | Example    |
| ---------------- | ----------------------- | ---------- |
| `vprepWidth`     | V-PREP image width      | 640        |
| `vprepHeight`    | V-PREP image height     | 360        |
| `viewportWidth`  | Browser viewport width  | 1280       |
| `viewportHeight` | Browser viewport height | 720        |
| `gold`           | Current gold amount     | 180        |
| `strategy`       | Building strategy       | 'balanced' |

---

## Best Practices

1. **Keep prompts focused** - One task per prompt
2. **Use visual descriptions** - Colors, shapes, positions
3. **Include examples** - Show expected output format
4. **Add negative constraints** - What NOT to click
5. **Test incrementally** - Change one thing at a time
6. **Log everything** - Track what LLM returns
7. **Use debug screenshots** - See what LLM sees

---

## Troubleshooting

### LLM Returns Empty Response

- Check image is being sent
- Verify model is running
- Check token limits

### Coordinates Out of Bounds

- Verify V-PREP dimensions
- Check scaling factor calculation
- Add explicit bounds in prompt

### State Always Detected as A

- Check other state prompts
- Verify visual differences
- Adjust detection criteria

---

## Version History

- **v1.0** - Initial modular prompt system
- States A-E implemented
- Visual-first approach
- Three-check verification
