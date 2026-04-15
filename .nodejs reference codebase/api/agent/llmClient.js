/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview HTTP Client for Local LLM (OpenAI-compatible) with Vision Support.
 * Merged from local-agent/network/llmClient.js
 * @module api/agent/llmClient
 */

import { createLogger } from "../core/logger.js";
import { configManager } from "../core/config.js";

const logger = createLogger("api/agent/llmClient.js");

class LLMClient {
  constructor() {
    this.config = null;
    this.isRestarting = false;
    this.restartPromise = null;
  }

  async init() {
    if (this.config) return;

    try {
      await configManager.init();
      this.config = configManager.get("agent.llm");
    } catch (_e) {
      logger.warn("Failed to load agent config, using defaults");
      this.config = configManager._getDefaults().agent.llm;
    }
  }

  /**
   * Converts messages to Ollama format for vision models.
   * Ollama expects images as base64 in an 'images' array at message level,
   * not as content arrays (which is OpenAI format).
   * @param {Array} messages - Messages in OpenAI/vision format
   * @returns {Array} Messages in Ollama format
   */
  _convertToOllamaFormat(messages) {
    return messages.map((msg) => {
      if (Array.isArray(msg.content)) {
        let textContent = "";
        let images = [];

        for (const part of msg.content) {
          if (part.type === "text") {
            textContent += part.text + "\n";
          } else if (part.type === "image_url") {
            const imgUrl = part.image_url?.url || part.image_url;
            if (imgUrl && imgUrl.startsWith("data:image")) {
              const base64 = imgUrl.split(",")[1];
              if (base64) images.push(base64);
            }
          }
        }

        return {
          role: msg.role,
          content: textContent.trim(),
          ...(images.length > 0 && { images }),
        };
      }
      return msg;
    });
  }

  /**
   * Performs a health check on the LLM endpoint.
   * @returns {Promise<boolean>}
   */
  async checkAvailability() {
    await this.init();
    logger.info(`Checking LLM availability at ${this.config.baseUrl}...`);

    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 5000);

      const response = await fetch(`${this.config.baseUrl}/models`, {
        method: "GET",
        signal: controller.signal,
      });
      clearTimeout(timeoutId);

      if (response.ok) {
        const data = await response.json();
        logger.info(
          `LLM Service Available. Found ${data.data ? data.data.length : 0} models.`,
        );
        return true;
      } else {
        const text = await response.text();
        logger.error(
          `LLM Service responded with Error: ${response.status}. Body: ${text}`,
        );
        return false;
      }
    } catch (error) {
      if (this.config.bypassHealthCheck) {
        logger.warn(
          `Health check failed but bypassHealthCheck is enabled. Proceeding...`,
        );
        return true;
      }
      logger.error(`LLM Connection Failed: ${error.message}`);
      return false;
    }
  }

  /**
   * Ensures the model is running by checking availability.
   * @returns {Promise<void>}
   */
  async ensureModelRunning() {
    await this.init();
    logger.info("Checking if model is running...");

    const isAvailable = await this.checkAvailability();
    if (isAvailable) {
      logger.info("Model is already running.");
      return;
    }

    logger.warn("Model not running. Please start the model server manually.");
  }

  /**
   * Generates a completion via HTTP API with Vision Support.
   * @param {Array<{role: string, content: string|Array}>} messages - Chat history
   * @returns {Promise<object>} The JSON parsed response from the LLM
   */
  async generateCompletion(messages) {
    await this.init();

    if (this.isRestarting && this.restartPromise) {
      await this.restartPromise;
    }

    let url, payload;

    if (this.config.serverType === "ollama") {
      url = `${this.config.baseUrl}/api/chat`;

      const ollamaMessages = this._convertToOllamaFormat(messages);

      payload = {
        model: this.config.model,
        messages: ollamaMessages,
        stream: false,
        think: false,
        format: "json",
        options: {
          temperature: this.config.temperature,
          num_ctx: this.config.contextLength,
          num_predict: this.config.maxTokens,
          repeat_penalty: this.config.repeatPenalty || 1.1,
        },
      };
    } else {
      url = `${this.config.baseUrl}/chat/completions`;
      payload = {
        model: this.config.model,
        messages: messages,
        temperature: this.config.temperature,
        stream: false,
        max_tokens: this.config.maxTokens,
      };
    }

    const visionStatus = this.config.useVision
      ? "Vision Enabled"
      : "Vision Disabled";
    logger.info(`Sending Request to ${this.config.model} [${visionStatus}]...`);
    const startTime = Date.now();

    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(
        () => controller.abort(),
        this.config.timeoutMs,
      );

      const response = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`HTTP Error ${response.status}: ${errorText}`);
      }

      const data = await response.json();
      const elapsedMs = Date.now() - startTime;

      if (data.usage) {
        const { prompt_tokens, completion_tokens, total_tokens } = data.usage;
        const tokensPerSec = completion_tokens / (elapsedMs / 1000);
        logger.info(
          `Tokens: ${prompt_tokens} + ${completion_tokens} = ${total_tokens} total | ${tokensPerSec.toFixed(1)} tok/s | ${elapsedMs}ms`,
        );
      } else {
        logger.info(`Response time: ${elapsedMs}ms`);
      }

      let content;
      if (data.choices && data.choices.length > 0) {
        content = data.choices[0].message.content;
      } else if (data.message) {
        content = data.message.content;
      } else {
        logger.error("Unexpected API response format:", JSON.stringify(data));
        throw new Error("Unexpected API response format");
      }

      logger.info(
        `Raw LLM Output: ${content.substring(0, 500)}${content.length > 500 ? "..." : ""}`,
      );

      content = content.trim();

      // Clean up special tokens from model output
      content = content
        .replace(/<\|im_start\|>/gi, "")
        .replace(/<\|im_end\|>/gi, "")
        .replace(/<\|endoftext\|>/gi, "")
        .replace(/<\|pad\|>/gi, "")
        .replace(/<\|repo_name\|>.*?<\|end\|>/gi, "")
        .replace(/<\|file_separator\|>/gi, "")
        .replace(/<\|time\|>.*?<\|end\|>/gi, "")
        .trim();

      let jsonResult;
      try {
        // Try direct parse first
        jsonResult = JSON.parse(content);
      } catch (e) {
        logger.warn(
          `Initial JSON parse failed, attempting robust extraction...`,
        );

        // Aggressive cleanup: strip everything before first { or [
        let cleaned = content;
        const firstBrace = cleaned.indexOf("{");
        const firstBracket = cleaned.indexOf("[");
        let startIdx = -1;

        if (firstBrace !== -1 && firstBracket !== -1) {
          startIdx = Math.min(firstBrace, firstBracket);
        } else if (firstBrace !== -1) {
          startIdx = firstBrace;
        } else if (firstBracket !== -1) {
          startIdx = firstBracket;
        }

        if (startIdx > 0) {
          cleaned = cleaned.substring(startIdx);
        }

        // Remove markdown code blocks
        cleaned = cleaned
          .replace(/^```json\s*/i, "")
          .replace(/^```\s*/i, "")
          .replace(/\s*```$/, "")
          .trim();

        // Now find the start again in cleaned string
        const cleanStart = cleaned.indexOf("{");
        const cleanBracket = cleaned.indexOf("[");
        let cleanIdx = -1;
        if (cleanStart !== -1 && cleanBracket !== -1) {
          cleanIdx = Math.min(cleanStart, cleanBracket);
        } else if (cleanStart !== -1) {
          cleanIdx = cleanStart;
        } else if (cleanBracket !== -1) {
          cleanIdx = cleanBracket;
        }

        if (cleanIdx !== -1) {
          let braceCount = 0;
          let lastBrace = -1;
          let inString = false;
          let escaped = false;

          for (let i = cleanIdx; i < cleaned.length; i++) {
            const char = cleaned[i];

            if (char === '"' && !escaped) {
              inString = !inString;
            }

            if (!inString) {
              if (char === "{" || char === "[") braceCount++;
              if (char === "}" || char === "]") {
                braceCount--;
                if (braceCount === 0) {
                  lastBrace = i;
                  break;
                }
              }
            }

            escaped = char === "\\" && !escaped;
          }

          if (lastBrace !== -1) {
            const extracted = cleaned.substring(cleanIdx, lastBrace + 1);
            try {
              jsonResult = JSON.parse(extracted);
            } catch (extractError) {
              logger.warn(
                `Extracted JSON parse failed, attempting repair: ${extractError.message}`,
              );

              // 3. Heuristic JSON Repair for truncated and malformed responses
              let repaired = extracted;

              // Fix common truncation: if ends in a colon or property name
              if (repaired.match(/:\s*$/)) repaired += '""';

              // Convert single quotes to double quotes for property names and string values
              // Handle property names: 'key': -> "key":
              repaired = repaired.replace(/([{,]\s*)'([^']+)'\s*:/g, '$1"$2":');
              // Handle string values: : 'value' -> : "value"
              repaired = repaired.replace(/:\s*'([^']*)'/g, ': "$1"');
              // Handle arrays with single-quoted strings: ['a', 'b'] -> ["a", "b"]
              repaired = repaired.replace(/\['([^']*)'\]/g, '["$1"]');
              // Handle commas in single-quoted strings: 'a, b' -> leave as is (already handled above)

              // Fix unquoted property names: {key: -> {"key":
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

              // Balance braces
              const openBraces = (repaired.match(/{/g) || []).length;
              const closeBraces = (repaired.match(/}/g) || []).length;
              for (let i = 0; i < openBraces - closeBraces; i++) {
                repaired += "}";
              }

              // Balance brackets
              const openBrackets = (repaired.match(/\[/g) || []).length;
              const closeBrackets = (repaired.match(/\]/g) || []).length;
              for (let i = 0; i < openBrackets - closeBrackets; i++) {
                repaired += "]";
              }

              try {
                jsonResult = JSON.parse(repaired);
                logger.info("JSON successfully repaired and parsed");
              } catch (_re) {
                logger.error(`Repair failed: ${_re.message}`);
                // Last resort: try to extract just the values
                logger.warn("Attempting emergency extraction...");
                throw e;
              }
            }
          } else {
            // Truncated response that never closed the first brace
            logger.warn(
              "Detected truncated JSON with no closing brace, attempting emergency repair...",
            );
            let repaired = cleaned;

            // Trim trailing whitespace/newlines
            repaired = repaired.trimEnd();

            // Remove trailing commas before } or ]
            repaired = repaired.replace(/,(\s*[}\]])/g, "$1");

            // Balance quotes
            if (
              !repaired.endsWith('"') &&
              (repaired.match(/"/g) || []).length % 2 !== 0
            )
              repaired += '"';

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

            try {
              jsonResult = JSON.parse(repaired);
              logger.info("Emergency repair successful");
            } catch (_finalError) {
              logger.warn(`Emergency repair failed: ${_finalError.message}`);
              // Fallback: Extract all coordinates from raw text
              logger.info("Attempting regex extraction from raw output...");

              // Extract all target objects with x, y, and optionally price
              // Pattern: {"x": NNN, "y": NNN, "price": "..."}  or  {x: NNN, y: NNN, ...}
              const targetPattern =
                /["']?x["']?\s*:\s*(\d+)\s*,\s*["']?y["']?\s*:\s*(\d+)(?:\s*,\s*["']?price["']?\s*:\s*["']?(\w+)["']?)?/g;
              const targets = [];
              let match;

              while ((match = targetPattern.exec(cleaned)) !== null) {
                targets.push({
                  x: parseInt(match[1], 10),
                  y: parseInt(match[2], 10),
                  price: match[3] || null,
                });
              }

              if (targets.length > 0) {
                logger.info(
                  `Regex extraction successful: found ${targets.length} target(s)`,
                );
                targets.forEach((t, i) => {
                  logger.info(
                    `  Target ${i + 1}: x=${t.x}, y=${t.y}, price=${t.price}`,
                  );
                });

                // Return in the expected format
                jsonResult = {
                  targets: targets,
                  found: true,
                  _extracted: true, // Flag to indicate this was extracted via regex
                };
              } else {
                // No coordinates found, re-throw original error
                logger.error("Regex extraction failed - no coordinates found");
                throw e;
              }
            }
          }
        } else {
          throw e; // No brace found
        }
      }

      return jsonResult;
    } catch (error) {
      logger.error(`LLM Request Failed: ${error.message}`);
      throw error;
    }
  }

  /**
   * Generates a completion with retry logic and exponential backoff
   * @param {Array<{role: string, content: string|Array}>} messages - Chat history
   * @param {number} maxRetries - Maximum retry attempts (default: 3)
   * @returns {Promise<object>} The JSON parsed response from the LLM
   */
  async generateCompletionWithRetry(messages, maxRetries = 3) {
    let lastError;

    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        return await this.generateCompletion(messages);
      } catch (error) {
        lastError = error;

        // Don't retry on validation/auth errors (permanent failures)
        if (
          error.message.includes("400") ||
          error.message.includes("401") ||
          error.message.includes("403")
        ) {
          throw error;
        }

        if (attempt < maxRetries) {
          // Exponential backoff: 1s, 2s, 4s, etc.
          const delay = Math.min(1000 * Math.pow(2, attempt - 1), 10000);
          logger.warn(
            `LLM attempt ${attempt}/${maxRetries} failed, retrying in ${delay}ms...`,
          );
          await new Promise((resolve) => setTimeout(resolve, delay));
        }
      }
    }

    logger.error(`LLM failed after ${maxRetries} attempts`);
    throw lastError;
  }

  /**
   * Generates a completion with structured output (function calling for OpenAI, format for Ollama)
   * @param {Array<{role: string, content: string|Array}>} messages - Chat history
   * @returns {Promise<object>} The parsed action object
   */
  async generateCompletionStructured(messages) {
    await this.init();

    let url, payload;

    // Define the action schema for function calling
    const _actionFunction = {
      type: "function",
      function: {
        name: "execute_action",
        description: "Execute a game automation action",
        parameters: {
          type: "object",
          properties: {
            action: {
              type: "string",
              enum: [
                "click",
                "clickAt",
                "type",
                "press",
                "scroll",
                "wait",
                "verify",
                "done",
                "drag",
                "navigate",
              ],
              description: "The action to perform",
            },
            selector: {
              type: "string",
              description: "CSS selector for the target element",
            },
            x: {
              type: "number",
              description: "X coordinate for clickAt",
            },
            y: {
              type: "number",
              description: "Y coordinate for clickAt",
            },
            clickType: {
              type: "string",
              enum: ["single", "double", "long"],
              description: "Type of click",
            },
            value: {
              type: "string",
              description: "Value for type/wait/scroll actions",
            },
            key: {
              type: "string",
              description: "Key to press",
            },
            description: {
              type: "string",
              description: "Description for verify action",
            },
            rationale: {
              type: "string",
              description: "Explanation of tactical decision",
            },
          },
          required: ["action", "rationale"],
        },
      },
    };

    if (this.config.serverType === "ollama") {
      // Ollama uses format: 'json' for structured output
      url = `${this.config.baseUrl}/api/chat`;
      const ollamaMessages = this._convertToOllamaFormat(messages);

      payload = {
        model: this.config.model,
        messages: ollamaMessages,
        stream: false,
        think: false,
        format: "json",
        options: {
          temperature: this.config.temperature,
          num_ctx: this.config.contextLength,
          num_predict: this.config.maxTokens,
        },
      };
    } else {
      // OpenAI-compatible: use response_format for structured output
      url = `${this.config.baseUrl}/chat/completions`;
      payload = {
        model: this.config.model,
        messages: messages,
        temperature: this.config.temperature,
        stream: false,
        max_tokens: this.config.maxTokens,
        response_format: { type: "json_object" },
      };
    }

    logger.info(`Sending structured request to ${this.config.model}...`);
    const startTime = Date.now();

    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(
        () => controller.abort(),
        this.config.timeoutMs,
      );

      const response = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`HTTP Error ${response.status}: ${errorText}`);
      }

      const data = await response.json();
      const elapsedMs = Date.now() - startTime;
      logger.info(`Structured response received in ${elapsedMs}ms`);

      let content;
      if (data.choices && data.choices.length > 0) {
        content = data.choices[0].message.content;
      } else if (data.message) {
        content = data.message.content;
      } else {
        throw new Error("Unexpected API response format");
      }

      // Parse the JSON response
      content = content.trim();
      const jsonResult = JSON.parse(content);
      return jsonResult;
    } catch (error) {
      logger.error(`Structured LLM Request Failed: ${error.message}`);
      throw error;
    }
  }

  /**
   * Get usage statistics
   * @returns {object} Config and status
   */
  getUsageStats() {
    return {
      model: this.config?.model,
      baseUrl: this.config?.baseUrl,
      useVision: this.config?.useVision,
      isRestarting: this.isRestarting,
    };
  }
}

const llmClient = new LLMClient();

export { llmClient, LLMClient };
export default llmClient;
