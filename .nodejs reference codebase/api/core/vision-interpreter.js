/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Vision Interpreter - Bridges the gap between raw LLM vision output and executable actions.
 * Converts unstructured text descriptions into structured JSON actions using Semantic Tree context.
 * @module core/vision-interpreter
 */

// import { createLogger } from '../core/logger.js';

// const logger = createLogger('vision-interpreter.js');

/**
 * @class VisionInterpreter
 * @description Interprets LLM vision responses into structured actions.
 */
class VisionInterpreter {
    constructor() {
        this.jsonBlockRegex = /```json\s*([\s\S]*?)\s*```/i;
        this.jsonRegex = /\{[\s\S]*\}/;
    }

    /**
     * Build a structured prompt for the Vision LLM (Llava/Qwen).
     * @param {object} context - Context context.
     * @param {string} context.goal - The user's goal.
     * @param {Array} context.semanticTree - List of interactive elements with coordinates.
     * @returns {string} The formatted prompt.
     */
    buildPrompt(context) {
        const { goal, semanticTree } = context;

        let elementsCallback = '';
        // Add top 30 elements (limited context)
        const elements = (semanticTree || []).slice(0, 30);

        if (elements.length === 0) {
            elementsCallback =
                'No interactive elements detected (Blind Mode). Rely purely on visual inspection.';
        } else {
            elements.forEach((el, index) => {
                const name = el.name || el.text || el.accessibilityId || 'Unknown';
                const role = el.role || 'element';
                const coords = el.coordinates
                    ? `(${el.coordinates.x},${el.coordinates.y})`
                    : '(0,0)';
                elementsCallback += `${index}. [${role}] "${name}" @ ${coords}\n`;
            });
        }

        return `You are an intelligent browser automation agent.
Your Task: Analyze the webpage image and the providing List of Elements to determine the sequence of actions needed to achieve the User Goal.

User Goal: "${goal}"

Interactive Elements List:
${elementsCallback}

Instructions:
1.  **Analyze**: Look at the image and the list. Identify which element (by ID) matches the goal.
2.  **Plan**: Decide if you need to CLICK or TYPE.
3.  **Strict JSON**: Output ONLY a valid JSON object. No other text.

Valid Action Types:
- "click": Use for buttons, links, or focusing input fields.
- "type": Use for inputting text into fields.
- "scroll": Use to move the page view. Params: "direction" ("up"/"down"), "amount" (pixels).
- "move": Use to hover over an element without clicking.
- "read": Use to simulate reading/thinking (simulates random mouse movement). Params: "duration" (ms).
- "wait": Use if the page needs time to load.

JSON Format Example (Few-Shot):
Goal: "Search for apples"
{
  "thought": "I see a search input field (ID 5). I need to click it and then type 'apples'.",
  "actions": [
    {
      "type": "click",
      "elementId": 5,
      "coordinates": { "x": 500, "y": 200 },
      "description": "Click search box"
    },
    {
      "type": "type",
      "text": "apples",
      "elementId": 5,
      "coordinates": { "x": 500, "y": 200 },
      "description": "Type search query"
    },
     {
      "type": "wait",
      "duration": 1000,
      "description": "Wait for suggestions"
    }
  ]
}

Now, generate the JSON plan for the User Goal: "${goal}"`;
    }

    /**
     * Parse the raw text response from the LLM into a structured object.
     * @param {string} rawText - The raw string response from the LLM.
     * @returns {object} The parsed result { success: boolean, data: object, error: string }
     */
    parseResponse(rawText) {
        if (!rawText) {
            return { success: false, error: 'Empty response' };
        }

        let jsonString = null;

        // Try to find JSON in markdown blocks
        const codeBlockMatch = rawText.match(this.jsonBlockRegex);
        if (codeBlockMatch && codeBlockMatch[1]) {
            jsonString = codeBlockMatch[1];
        } else {
            // Try to find raw JSON object
            const jsonMatch = rawText.match(this.jsonRegex);
            if (jsonMatch) {
                jsonString = jsonMatch[0];
            }
        }

        if (!jsonString) {
            // Fallback: If minimal response, maybe it failed to output JSON
            // But we can try to interpret a simple description if needed.
            // For now, return failure to parse.
            return {
                success: false,
                error: 'No JSON found in response',
                raw: rawText,
            };
        }

        try {
            const data = JSON.parse(jsonString);

            // Validate structure
            if (!data.actions || !Array.isArray(data.actions)) {
                return {
                    success: false,
                    error: "Invalid JSON structure: missing 'actions' array",
                    data,
                    raw: rawText,
                };
            }

            return { success: true, data };
        } catch (e) {
            return { success: false, error: 'JSON parse error: ' + e.message, raw: rawText };
        }
    }
}

export default VisionInterpreter;
