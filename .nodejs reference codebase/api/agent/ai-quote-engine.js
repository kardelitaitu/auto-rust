/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * AI Quote Tweet Engine
 * Handles AI quote generation, safety filtering, and posting
 * @module utils/ai-quote-engine
 */

import { createLogger } from "../core/logger.js";
import { api } from "../index.js";
import { mathUtils } from "../utils/math.js";
import { sentimentService } from "../utils/sentiment-service.js";
import { HumanInteraction } from "../behaviors/human-interaction.js";
import {
  getStrategyInstruction,
  QUOTE_SYSTEM_PROMPT,
} from "../twitter/twitter-reply-prompt.js";

/**
 * AIQuoteEngine - Handles AI quote tweet generation
 * @extends AIReplyEngine
 */
export class AIQuoteEngine {
  /**
   * Creates a new AIQuoteEngine instance
   * @param {object} agentConnector - Agent connector for AI requests
   * @param {object} options - Configuration options
   */
  constructor(agentConnector, options = {}) {
    this.agent = agentConnector;
    this.logger = createLogger("ai-quote-engine.js");
    this.config = {
      QUOTE_PROBABILITY: options.quoteProbability ?? 1.0,
      MAX_QUOTE_LENGTH: 250,
      MIN_QUOTE_LENGTH: 10,
      MAX_RETRIES: options.maxRetries ?? 2,
      SAFETY_FILTERS: {
        minTweetLength: 10,
        maxTweetLength: 280,
        excludedKeywords: [
          "politics",
          "political",
          "vote",
          "election",
          "trump",
          "biden",
          "obama",
          "republican",
          "democrat",
          "congress",
          "senate",
          "president",
          "policy",
          "nsfw",
          "nude",
          "naked",
          "explicit",
          "18+",
          "adult",
          "xxx",
          "porn",
          "follow back",
          "fb",
          "make money",
          "drop link",
          "free crypto",
          "dm me",
          "send dm",
          "join now",
          "limited offer",
          "act now",
        ],
        genericResponses: [
          "interesting",
          "so true",
          "agreed",
          "facts",
          "literally me",
          "mood",
          "relatable",
          "this 👏",
          "preach",
          "couldn't agree more",
          "absolutely",
          "can confirm",
          "same energy",
          "spot on",
          "big mood",
          "my life",
          "me rn",
          "real talk",
          "needed this",
          "speak louder",
          "says who",
          "waiting for this",
          "finally",
          "go off",
          "queen behavior",
          "king behavior",
          "didn't ask",
          "nobody asked",
          "who asked",
          "🤷",
          "💯",
          "🔥",
          "👏👏",
          "👏",
          "🤝",
          "✨",
          "🙏",
        ],
      },
    };

    this.stats = {
      attempts: 0,
      successes: 0,
      skips: 0,
      failures: 0,
      safetyBlocks: 0,
      errors: 0,
      emptyContent: 0,
      extractFailed: 0,
      validationFailed: 0,
      retriesUsed: 0,
      fallbackUsed: 0,
    };

    this.logger.info(
      `[AIQuoteEngine] Initialized (probability: ${this.config.QUOTE_PROBABILITY})`,
    );
  }

  /**
   * Updates configuration at runtime
   * @param {object} options - Configuration options
   */
  updateConfig(options) {
    if (options.quoteProbability !== undefined) {
      this.config.QUOTE_PROBABILITY = options.quoteProbability;
    }
    if (options.maxRetries !== undefined) {
      this.config.MAX_RETRIES = options.maxRetries;
    }
  }

  /**
   * Detect primary language from text content
   */
  detectLanguage(text) {
    if (!text || typeof text !== "string") return "en";

    const textLower = text.toLowerCase();

    // Language patterns (common words/phrases)
    const languages = {
      en: /\b(the|is|are|was|were|have|has|been|will|would|could|should|this|that|these|those|i|you|he|she|it|we|they|what|which|who|when|where|why|how)\b/i,
      es: /\b(el|la|los|las|es|son|fue|fueron|tiene|tienen|será|sería|puede|podrían|esto|ese|esta|yo|tú|él|ella|ellos|qué|cualquién|cuando|dónde|por qué|cómo)\b/i,
      fr: /\b(le|la|les|est|sont|était|étaient|a|ont|été|sera|serait|peuvent|cela|cette|ces|je|tu|il|elle|nous|vous|ils|elles|que|qui|quoi|quand|où|pourquoi|comment)\b/i,
      de: /\b(der|die|das|den|dem|ist|sind|war|waren|hat|haben|wird|wäre|kann|können|dieser|diese|dieses|ich|du|er|sie|es|wir|sie|was|welcher|wer|wo|warum|wie)\b/i,
      pt: /\b(o|a|os|as|é|são|foi|foram|tem|têm|será|seria|podem|isso|essa|esses|eu|você|ele|ela|nós|vocês|eles|elas|o quê|qual|quem|quando|onde|por quê|como)\b/i,
      id: /\b(ini|itu|adalah|adalah|tersebut|dengan|untuk|dan|di|dari|yang|apa|siapa|apa|ketika|di mana|mengapa|bagai)\b/i,
      ja: /\b(これ|それ|あれ|この|その|あの|です|だ|ある|いる|ます|か|の|に|を|と|が|は|だれの|何|いつ|どこ|なぜ|怎样)\b/i,
      ko: /\b(이|그|저|이다|있다|하다|것|을|를|에|에서|과|와|는|은|다|吗|什么|何时|哪里|为什么)\b/i,
      zh: /\b(这|那|是|的|在|有|和|与|对|就|都|而|及|与|着|或|什么|谁|何时|何地|为何|如何)\b/i,
    };

    const scores = {};
    let totalScore = 0;

    for (const [lang, pattern] of Object.entries(languages)) {
      const match = textLower.match(pattern);
      const score = match ? match.length : 0;
      scores[lang] = score;
      totalScore += score;
    }

    if (totalScore === 0) return "en";

    // Find the language with highest score
    let maxScore = 0;
    let detectedLang = "en";

    for (const [lang, score] of Object.entries(scores)) {
      if (score > maxScore) {
        maxScore = score;
        detectedLang = lang;
      }
    }

    // Map to full language name
    const langNames = {
      en: "English",
      es: "Spanish",
      fr: "French",
      de: "German",
      pt: "Portuguese",
      id: "Indonesian",
      ja: "Japanese",
      ko: "Korean",
      zh: "Chinese",
    };

    return langNames[detectedLang] || "English";
  }

  /**
   * Detect primary language from array of replies
   */
  detectReplyLanguage(replies) {
    if (!replies || replies.length === 0) return "English";

    // Sample up to 5 replies for language detection
    const sampleText = replies
      .slice(0, 5)
      .map((r) => (r.text || r.content || "").substring(0, 500))
      .join(" ");

    return this.detectLanguage(sampleText);
  }

  /**
   * Build enhanced prompt with context (tweet, replies, screenshot)
   */
  buildEnhancedPrompt(
    tweetText,
    authorUsername,
    replies = [],
    url = "",
    sentimentContext = {},
    hasImage = false,
    engagementLevel = "unknown",
  ) {
    // Detect language from replies
    const detectedLanguage = this.detectReplyLanguage(replies);
    this.logger.debug(`[AIQuote] Detected language: ${detectedLanguage}`);

    // Get sentiment guidance
    const sentiment = sentimentContext.engagementStyle || "neutral";
    const conversationType = sentimentContext.conversationType || "general";
    const valence = sentimentContext.valence || 0;
    const sarcasmScore = sentimentContext.sarcasm || 0;

    // Generate sentiment-aware tone guidance
    const toneGuidance = this.getStyleGuidance(sentiment, sarcasmScore);

    // === STRATEGY SELECTION ===
    // Use the improved strategy selector from twitter-reply-prompt.js
    const strategyInstruction = getStrategyInstruction({
      sentiment:
        valence < -0.3 ? "negative" : valence > 0.3 ? "positive" : "neutral",
      type: conversationType,
      engagement: engagementLevel,
    });

    let prompt = `Tweet from @${authorUsername}:
"${tweetText}"

Tweet URL: ${url}

`;

    if (replies.length > 0) {
      // Sort by length (longest first) and take top 30 for richer context
      const sortedReplies = replies
        .filter((r) => r.text && r.text.length > 5)
        .sort((a, b) => (b.text?.length || 0) - (a.text?.length || 0))
        .slice(0, 30);

      prompt += `Other replies to this tweet (in ${detectedLanguage}):
`;
      sortedReplies.forEach((reply, i) => {
        const author = reply.author || "User";
        const text = reply.text || reply.content || "";
        prompt += `${i + 1}. @${author}: "${text.substring(0, 200)}"
`;
      });
      prompt += `
`;
    }

    prompt += `Language detected: ${detectedLanguage}

IMPORTANT: Keep it SHORT. Maximum 1 short sentence. No paragraphs.

Tweet Analysis:
  - Sentiment: ${sentiment}
  - Conversation Type: ${conversationType}
  - Valence: ${valence > 0 ? "Positive" : valence < 0 ? "Negative" : "Neutral"}
  ${hasImage ? "- [IMAGE DETECTED] This tweet contains an image. Analyze it and comment on visual details." : ""}

  TONE GUIDANCE: ${toneGuidance}
  STRATEGY INSTRUCTION: ${strategyInstruction}
LENGTH: MAXIMUM 1 SHORT SENTENCE

Write ONE short quote tweet (1 sentence max).
IMPORTANT: This is a QUOTE TWEET, not a reply. You are sharing this tweet with your own followers adding your commentary.
`;

    return {
      text: prompt,
      language: detectedLanguage,
      replyCount: replies.length,
      sentiment: {
        engagementStyle: sentiment,
        conversationType: conversationType,
        valence,
        sarcasmScore,
      },
    };
  }

  /**
   * Decides whether to quote a tweet
   * @param {string} tweetText - Tweet content
   * @param {string} authorUsername - Tweet author
   * @param {object} context - Additional context
   * @returns {Promise<object>} Quote decision
   */
  async shouldQuote(tweetText, authorUsername, _context = {}) {
    this.stats.attempts++;

    const rolled = Math.random();
    this.logger.debug(
      `[AI-Quote] Probability check: ${(this.config.QUOTE_PROBABILITY * 100).toFixed(1)}% threshold, rolled ${(rolled * 100).toFixed(1)}%`,
    );

    if (!mathUtils.roll(this.config.QUOTE_PROBABILITY)) {
      this.stats.skips++;
      return {
        decision: "skip",
        reason: "probability",
        quote: null,
      };
    }

    return {
      decision: "proceed",
      reason: "eligible",
      quote: null,
    };
  }

  /**
   * Generates a quote tweet using AI
   * @param {string} tweetText - Tweet content
   * @param {string} authorUsername - Tweet author
   * @param {object} context - Additional context
   * @returns {Promise<object>} Generated quote result
   */
  async generateQuote(tweetText, authorUsername, context = {}) {
    const { url = "", replies = [] } = context;

    // ================================================================
    // FULL SENTIMENT ANALYSIS OF TWEET
    // ================================================================
    const tweetSentiment = sentimentService.analyze(tweetText);

    // Log comprehensive sentiment analysis
    this.logger.info(`[AIQuote] Sentiment Analysis:`);
    this.logger.info(
      `[AIQuote]   - Overall: ${tweetSentiment.isNegative ? "NEGATIVE" : "NEUTRAL/POSITIVE"} (score: ${tweetSentiment.score.toFixed(2)})`,
    );
    this.logger.info(
      `[AIQuote]   - Valence: ${tweetSentiment.dimensions?.valence?.valence?.toFixed(2) || "N/A"}`,
    );
    this.logger.info(
      `[AIQuote]   - Arousal: ${tweetSentiment.dimensions?.arousal?.arousal?.toFixed(2) || "N/A"}`,
    );
    this.logger.info(
      `[AIQuote]   - Dominance: ${tweetSentiment.dimensions?.dominance?.dominance?.toFixed(2) || "N/A"}`,
    );
    this.logger.info(
      `[AIQuote]   - Sarcasm: ${tweetSentiment.dimensions?.sarcasm?.sarcasm?.toFixed(2) || "N/A"}`,
    );
    this.logger.info(
      `[AIQuote]   - Toxicity: ${tweetSentiment.dimensions?.toxicity?.toxicity?.toFixed(2) || "N/A"}`,
    );
    this.logger.info(
      `[AIQuote]   - Risk Level: ${tweetSentiment.composite?.riskLevel || "N/A"}`,
    );
    this.logger.info(
      `[AIQuote]   - Engagement Style: ${tweetSentiment.composite?.engagementStyle || "N/A"}`,
    );
    this.logger.info(
      `[AIQuote]   - Conversation Type: ${tweetSentiment.composite?.conversationType || "N/A"}`,
    );

    // ================================================================
    // SKIP HIGH-RISK CONVERSATIONS
    // ================================================================
    if (tweetSentiment.isNegative && tweetSentiment.score > 0.3) {
      this.logger.warn(
        `[AIQuote] Skipping negative content (score: ${tweetSentiment.score.toFixed(2)})`,
      );
      return { quote: null, success: false, reason: "negative_content" };
    }

    if (tweetSentiment.composite?.riskLevel === "high") {
      this.logger.warn(`[AIQuote] Skipping high-risk conversation`);
      return { quote: null, success: false, reason: "high_risk_conversation" };
    }

    // Extract derived sentiment values
    const sentiment = tweetSentiment.composite?.engagementStyle || "neutral";
    const conversationType =
      tweetSentiment.composite?.conversationType || "general";
    const valence = tweetSentiment.dimensions?.valence?.valence || 0;
    const sarcasmScore = tweetSentiment.dimensions?.sarcasm?.sarcasm || 0;

    // ================================================================
    // SENTIMENT-BASED REPLY SELECTION
    // ================================================================
    let selectedReplies = replies;
    if (replies && replies.length > 0) {
      const sentimentAnalysis =
        sentimentService.analyzeForReplySelection(replies);
      this.logger.info(
        `[AIQuote] Reply selection strategy: ${sentimentAnalysis.strategy} ` +
          `(pos: ${sentimentAnalysis.distribution.positive}, ` +
          `neg: ${sentimentAnalysis.distribution.negative}, ` +
          `sarcastic: ${sentimentAnalysis.distribution.sarcastic})`,
      );

      const recs = sentimentAnalysis.recommendations;
      if (recs.manualSelection) {
        selectedReplies = recs.manualSelection;
      } else {
        selectedReplies = sentimentAnalysis.analyzed
          .filter((r) => recs.filter(r))
          .sort(recs.sort)
          .slice(0, recs.max)
          .map((r) => ({ author: r.author, text: r.text }));
      }

      this.logger.info(
        `[AIQuote] Selected ${selectedReplies.length} replies for LLM context ` +
          `(filtered from ${replies.length})`,
      );
    }

    // Build enhanced prompt with language detection
    const sentimentContext = {
      engagementStyle: sentiment,
      conversationType: conversationType,
      valence,
      sarcasm: sarcasmScore,
    };
    const hasImage = !!context.screenshot;
    const engagementLevel = context.engagementLevel || "unknown";
    const promptData = this.buildEnhancedPrompt(
      tweetText,
      authorUsername,
      selectedReplies,
      url,
      sentimentContext,
      hasImage,
      engagementLevel,
    );

    const systemPrompt = QUOTE_SYSTEM_PROMPT;

    // DEBUG: Log tweet and prompt being sent to LLM
    this.logger.info(`[DEBUG] ============================================`);
    this.logger.info(`[DEBUG] TWEET TO QUOTE:`);
    this.logger.info(`[DEBUG] Author: @${authorUsername}`);
    this.logger.info(`[DEBUG] URL: ${url}`);
    this.logger.info(`[DEBUG] Tweet Text: "${tweetText}"`);
    this.logger.info(`[DEBUG] Tweet Length: ${tweetText.length} chars`);
    this.logger.info(
      `[DEBUG] Sentiment: ${sentiment}, Tone: ${conversationType}`,
    );
    this.logger.info(`[DEBUG] ----------------------------------------------`);
    this.logger.info(
      `[DEBUG] REPLIES CONTEXT (${selectedReplies.length} selected from ${replies.length}):`,
    );
    selectedReplies.forEach((reply, idx) => {
      const author =
        reply.author && reply.author !== "unknown" ? reply.author : "User";
      const text = (reply.text || "").substring(0, 150);
      const ellipsis = (reply.text || "").length > 150 ? "..." : "";
      this.logger.info(
        `[DEBUG] [${idx + 1}] Reply${idx + 1}: "@${author}: ${text}${ellipsis}"`,
      );
    });
    this.logger.info(`[DEBUG] ----------------------------------------------`);
    this.logger.info(`[DEBUG] FULL PROMPT SENT TO LLM:`);
    this.logger.info(`[DEBUG] Language: ${promptData.language}`);
    this.logger.info(
      `[DEBUG] System Prompt Length: ${systemPrompt.length} chars`,
    );
    this.logger.info(
      `[DEBUG] User Prompt Length: ${promptData.text.length} chars`,
    );
    this.logger.info(`[DEBUG] ============================================`);

    try {
      let lastError = null;
      const maxRetries = this.config.MAX_RETRIES;

      for (let attempt = 1; attempt <= maxRetries; attempt++) {
        this.logger.info(
          `[AI-Quote] Generating ${promptData.language} quote tweet (attempt ${attempt}/${maxRetries})...`,
        );

        try {
          const result = await this.agent.processRequest({
            action: "generate_reply",
            sessionId: this.agent.sessionId || "quote-engine",
            payload: {
              systemPrompt,
              userPrompt: promptData.text,
              tweetText,
              authorUsername,
              engagementType: "quote",
              maxTokens: 75,
              context: {
                hasScreenshot: hasImage,
                replyCount: replies.length,
                detectedLanguage: promptData.language,
              },
            },
          });

          // DEBUG: Log raw LLM response
          this.logger.info(
            `[DEBUG] ----------------------------------------------`,
          );
          this.logger.info(`[DEBUG] LLM RAW RESPONSE (attempt ${attempt}):`);
          this.logger.info(
            `[DEBUG] ${result ? JSON.stringify(result).substring(0, 1000) : "result is null/undefined"}`,
          );
          this.logger.info(
            `[DEBUG] ----------------------------------------------`,
          );

          // Detailed failure analysis
          if (!result) {
            this.logger.error(
              `[AIQuoteEngine] ❌ LLM result is null/undefined (attempt ${attempt})`,
            );
            lastError = "llm_result_null";
            continue;
          }

          if (!result.success) {
            this.logger.error(
              `[AIQuoteEngine] ❌ LLM request failed: ${result.error || "unknown error"} (attempt ${attempt})`,
            );
            lastError = `llm_failed: ${result.error || "unknown"}`;
            continue;
          }

          // Extract content from nested response structure (result.data.content) or direct result.content
          const rawContent = result.data?.content || result.content || "";
          this.logger.info(
            `[DEBUG] Extracted content: "${rawContent.substring(0, 100)}..." (length: ${rawContent.length})`,
          );

          if (!rawContent || rawContent.trim().length === 0) {
            this.logger.error(
              `[AIQuoteEngine] ❌ LLM returned empty content (attempt ${attempt}/${maxRetries})`,
            );
            this.stats.emptyContent++;
            lastError = "llm_empty_content";
            if (attempt < maxRetries) {
              this.logger.info(`[AIQuoteEngine] Retrying...`);
              this.stats.retriesUsed++;
              continue;
            }
            return { quote: null, success: false, reason: "llm_empty_content" };
          }

          this.logger.debug(
            `[DEBUG] Raw content length: ${rawContent.length} chars`,
          );

          const reply = this.extractReplyFromResponse(rawContent);
          this.logger.debug(
            `[DEBUG] Extracted reply: "${reply?.substring(0, 100)}..."`,
          );

          if (!reply) {
            this.logger.warn(
              `[AIQuoteEngine] ⚠️ Could not extract reply from LLM response patterns (attempt ${attempt})`,
            );
            this.logger.warn(`[DEBUG] Full raw content:\n${rawContent}`);
            // Fallback: use raw content directly if it's a valid quote
            const fallbackReply = rawContent.trim();
            if (fallbackReply.length >= this.config.MIN_QUOTE_LENGTH) {
              this.logger.info(
                `[AIQuoteEngine] 🔄 Using raw content as fallback: "${fallbackReply.substring(0, 50)}..."`,
              );
              this.stats.fallbackUsed++;
              return {
                quote: fallbackReply,
                success: true,
                note: "fallback_content_used",
              };
            }
            this.stats.extractFailed++;
            lastError = "extract_reply_failed";
            if (attempt < maxRetries) {
              this.logger.info(`[AIQuoteEngine] Retrying...`);
              this.stats.retriesUsed++;
              continue;
            }
            return {
              quote: null,
              success: false,
              reason: "extract_reply_failed",
            };
          }

          const cleaned = this.cleanQuote(reply);
          this.logger.info(
            `[AIQuoteEngine] ✨ Cleaned quote: "${cleaned}" (${cleaned.length} chars)`,
          );

          if (cleaned.length < this.config.MIN_QUOTE_LENGTH) {
            this.logger.error(
              `[AIQuoteEngine] ❌ Quote too short: ${cleaned.length} chars (min: ${this.config.MIN_QUOTE_LENGTH}) (attempt ${attempt})`,
            );
            lastError = "quote_too_short";
            if (attempt < maxRetries) {
              this.logger.info(`[AIQuoteEngine] Retrying...`);
              this.stats.retriesUsed++;
              continue;
            }
            return { quote: null, success: false, reason: "quote_too_short" };
          }

          const validation = this.validateQuote(cleaned);
          if (validation.valid) {
            this.stats.successes++;
            return {
              quote: cleaned,
              success: true,
              language: promptData.language,
            };
          } else {
            this.logger.warn(
              `[AIQuoteEngine] ❌ Quote validation failed (${validation.reason}): "${cleaned}" (attempt ${attempt})`,
            );
            this.stats.validationFailed++;
            lastError = `validation_failed: ${validation.reason}`;
            if (attempt < maxRetries) {
              this.logger.info(`[AIQuoteEngine] Retrying...`);
              this.stats.retriesUsed++;
              continue;
            }
            return {
              quote: null,
              success: false,
              reason: `validation_failed: ${validation.reason}`,
            };
          }
        } catch (error) {
          this.logger.error(
            `[AIQuoteEngine] ❌ Generation error (attempt ${attempt}): ${error.message}`,
          );
          this.stats.errors++;
          lastError = error.message;
          if (attempt < maxRetries) {
            this.logger.info(`[AIQuoteEngine] Retrying...`);
            this.stats.retriesUsed++;
            continue;
          }
          return { quote: null, success: false, reason: error.message };
        }
      }

      // All retries exhausted
      this.logger.error(
        `[AIQuoteEngine] ❌ All ${maxRetries} attempts failed. Last error: ${lastError}`,
      );
      this.stats.failures++;
      return {
        quote: null,
        success: false,
        reason: `all_attempts_failed: ${lastError}`,
      };
    } catch (error) {
      this.logger.error(`[AIQuoteEngine] Generation failed: ${error.message}`);
      this.stats.errors++;
      return { quote: null, success: false, reason: error.message };
    }
  }

  extractReplyFromResponse(content) {
    if (!content) {
      this.logger.warn(`[extractReplyFromResponse] Empty content received`);
      return null;
    }

    const trimmed = content.trim();
    this.logger.debug(
      `[extractReplyFromResponse] Processing content: "${trimmed.substring(0, 100)}..." (length: ${trimmed.length})`,
    );

    // ================================================================
    // PATTERN 0: Direct content (already clean, most common case)
    // ================================================================
    if (trimmed.length >= 10 && trimmed.length < 300) {
      // Check if it looks like a real tweet (not JSON, not thinking)
      const isCleanResponse =
        !trimmed.startsWith("{") &&
        !trimmed.startsWith("[") &&
        !trimmed.startsWith("```") &&
        !trimmed.toLowerCase().includes("thinking") &&
        !trimmed.toLowerCase().includes("reasoning") &&
        !trimmed.toLowerCase().includes("<thought>");

      if (isCleanResponse) {
        // Check for reasoning patterns
        const isReasoning =
          /I (?:need to|should|want to|will|must|can|have) /i.test(trimmed) ||
          /Let me|I'll|First|Then|So I|Now I/i.test(trimmed) ||
          /This is my|My draft|Here's my|I think this/i.test(trimmed) ||
          /It needs to be|My draft fits/i.test(trimmed) ||
          /That'?s specific|It feels authentic/i.test(trimmed);

        if (!isReasoning) {
          this.logger.debug(`[extractReplyFromResponse] ✓ Direct content used`);
          return trimmed;
        }
      }
    }

    // ================================================================
    // PATTERN 1: Look for trailing quoted text (highest priority)
    // ================================================================
    const quotedMatch = trimmed.match(/"([^"]{10,280})"\s*$/);
    if (quotedMatch) {
      const quoted = quotedMatch[1].trim();
      if (!/I (?:need to|should|want to|will|must) /i.test(quoted)) {
        this.logger.debug(
          `[extractReplyFromResponse] ✓ Quoted text pattern matched`,
        );
        return quoted;
      }
    }

    // Pattern 1b: Single quotes
    const singleQuotedMatch = trimmed.match(/'([^']{10,280})'\s*$/);
    if (singleQuotedMatch) {
      const quoted = singleQuotedMatch[1].trim();
      if (!/I (?:need to|should|want to|will|must) /i.test(quoted)) {
        this.logger.debug(
          `[extractReplyFromResponse] ✓ Single-quoted text pattern matched`,
        );
        return quoted;
      }
    }

    // ================================================================
    // PATTERN 2: Look for content after last newline (often the actual response)
    // ================================================================
    const lines = trimmed.split("\n");
    const lastLine = lines[lines.length - 1].trim();

    if (lastLine.length > 10 && lastLine.length < 300) {
      // Check if it looks like a real response (not internal reasoning)
      const isReasoning =
        /I (?:need to|should|want to|will|must|can|have) /i.test(lastLine) ||
        /Let me|I'll|First|Then|So I|Now I/i.test(lastLine) ||
        /This is my|My draft|Here's my/i.test(lastLine) ||
        /It needs to be|My draft fits|I think this/i.test(lastLine) ||
        /That'?s specific|It feels authentic/i.test(lastLine) ||
        /That's specific|My draft|Here's my|I think this/i.test(lastLine);

      if (!isReasoning) {
        this.logger.debug(
          `[extractReplyFromResponse] ✓ Last line pattern matched`,
        );
        return lastLine;
      }
    }

    // ================================================================
    // PATTERN 3: Look for the last paragraph if it looks like a real response
    // ================================================================
    const paragraphs = trimmed.split(/\n\n+/);
    for (let i = paragraphs.length - 1; i >= 0; i--) {
      const para = paragraphs[i].trim();
      if (para.length > 10 && para.length < 300) {
        const isReasoning =
          /I (?:need to|should|want to|will|must|can|have) /i.test(para) ||
          /Let me|I'll|First|Then|So I|Now I/i.test(para) ||
          /This is my|My draft|Here's my/i.test(para) ||
          /It needs to be|My draft fits|I think this/i.test(para);
        if (!isReasoning) {
          this.logger.debug(
            `[extractReplyFromResponse] ✓ Paragraph pattern matched`,
          );
          return para;
        }
      }
    }

    // ================================================================
    // PATTERN 4: Look for content after "Answer:", "Response:", "Quote:", etc.
    // ================================================================
    const labelPatterns = [
      /^(?:Answer|Response|Quote|Tweet|My response|Here's my):?\s*/i,
      /^(?:The|Ma) quote:?\s*/i,
      /^(?:I'd|I would|No) (?:say|think|respond):?\s*/i,
    ];

    for (const pattern of labelPatterns) {
      const match = trimmed.match(pattern);
      if (match) {
        const afterLabel = trimmed.substring(match[0].length).trim();
        if (afterLabel.length >= 10 && afterLabel.length < 300) {
          this.logger.debug(
            `[extractReplyFromResponse] ✓ Label pattern matched`,
          );
          return afterLabel;
        }
      }
    }

    // ================================================================
    // PATTERN 5: Look for sentences in the content
    // ================================================================
    const sentenceMatch = trimmed.match(/^[^.!?]*[.!?]/);
    if (
      sentenceMatch &&
      sentenceMatch[0].length >= 10 &&
      sentenceMatch[0].length < 300
    ) {
      this.logger.debug(
        `[extractReplyFromResponse] ✓ Sentence pattern matched`,
      );
      return sentenceMatch[0].trim();
    }

    // ================================================================
    // STANDARD EXTRACTION (for non-thinking models)
    // ================================================================

    let cleaned = content
      .replace(/```json?\s*/gi, "")
      .replace(/```\s*/gi, "")
      .trim();

    cleaned = cleaned.replace(/<thinking>[\s\S]*?<\/thinking>/gi, "");
    cleaned = cleaned.replace(/<\/?THINKING>/gi, "");
    cleaned = cleaned.replace(/\[\/?THINKING\]/gi, "");
    cleaned = cleaned.replace(/\[[\s]*REASONING[\s]*\][\s\S]*?$/gim, "");
    cleaned = cleaned.replace(
      /^(?:First,?\s*)?I\s+(?:need to|should|want to|must|will|have to|can)\s+[\s\S]*?(?=\n\n|[.!?]\s*[A-Z][a-z]+)/gim,
      "",
    );
    cleaned = cleaned.replace(
      /(?:Let me|I'll|I will)\s+(?:think|reason|analyze)[\s\S]*?(?=\.\s*[A-Z]|\n\n)/gi,
      "",
    );
    cleaned = cleaned.replace(
      /^(?:My|Here's|The)\s+(?:draft|response|answer|output|suggestion):?\s*/gi,
      "",
    );
    cleaned = cleaned.replace(
      /^(?:Okay,?\s*)?I (?:need to|should|want to|will) [^\n]*/i,
      "",
    );

    if (cleaned.startsWith("{") || cleaned.startsWith("[")) {
      try {
        const parsed = JSON.parse(cleaned);
        if (parsed.reply) return parsed.reply;
        if (parsed.content) return parsed.content;
        if (parsed.text) return parsed.text;
        if (parsed.message) return parsed.message;
        if (parsed.output) return parsed.output;
        // Try to find any string field
        for (const key of Object.keys(parsed)) {
          if (
            typeof parsed[key] === "string" &&
            parsed[key].length >= 10 &&
            parsed[key].length < 300
          ) {
            return parsed[key];
          }
        }
      } catch (_error) {
        // Not JSON, continue
      }
    }

    cleaned = cleaned
      .replace(/^(?:Okay,?\s*)?I (?:need to|should|want to|will) [^\n]*/i, "")
      .trim();

    if (cleaned && cleaned.length >= 10 && cleaned.length < 300) {
      this.logger.debug(`[extractReplyFromResponse] ✓ Cleaned content used`);
      return cleaned;
    }

    this.logger.warn(
      `[extractReplyFromResponse] ❌ No pattern matched, returning null`,
    );
    this.logger.debug(
      `[DEBUG] Full content that failed to extract:\n${content}`,
    );
    return null;
  }

  /**
   * Executes the quote tweet flow
   * @param {object} page - Playwright page
   * @param {string} quoteText - Quote text to post
   * @param {object} options - Execution options
   * @returns {Promise<object>} Execution result
   */
  async executeQuote(page, quoteText, _options = {}) {
    this.logger.info(
      `[AIQuote] Executing quote (${quoteText.length} chars)...`,
    );

    const human = new HumanInteraction(page);

    const methods = [
      {
        name: "keyboard_compose",
        weight: 30,
        fn: () => this.quoteMethodA_Keyboard(page, quoteText, human),
      },
      {
        name: "retweet_menu",
        weight: 60,
        fn: () => this.quoteMethodB_Retweet(page, quoteText, human),
      },
      {
        name: "new_post",
        weight: 10,
        fn: () => this.quoteMethodC_Url(page, quoteText, human),
      },
    ];

    const selected = human.selectMethod(methods);
    this.logger.info(`[AIQuote] Selected strategy: ${selected.name}`);

    try {
      const result = await selected.fn();

      // Update stats based on result
      if (result.success) {
        this.stats.successes++;
      } else {
        this.stats.failures++;
      }

      return result;
    } catch (error) {
      this.logger.error(
        `[AIQuote] Method ${selected.name} failed: ${error.message}`,
      );
      this.logger.warn(`[AIQuote] Trying fallback: retweet_menu`);

      // Fallback to Retweet Menu method
      try {
        const result = await this.quoteMethodB_Retweet(page, quoteText, human);
        if (result.success) this.stats.successes++;
        else this.stats.failures++;
        return result;
      } catch (fallbackError) {
        this.stats.failures++;
        return {
          success: false,
          reason: `fallback_failed: ${fallbackError.message}`,
        };
      }
    }
  }

  getToneGuidance(tone) {
    const tones = {
      humorous: "Be witty, add a clever observation, or a lighthearted take.",
      informative: "Share a relevant fact, statistic, or related information.",
      emotional: "Express genuine emotion - why does this resonate with you?",
      supportive: "Show enthusiasm and encourage the author.",
      critical: "Offer a thoughtful counterpoint or question.",
      neutral: "Ask a specific question or add a relevant observation.",
    };
    return tones[tone] || tones.neutral;
  }

  getEngagementGuidance(engagement) {
    const engagements = {
      low: "Maximum 1 short sentence.",
      medium: "Maximum 1 short sentence.",
      high: "Maximum 1-2 short sentences.",
    };
    return engagements[engagement] || engagements.low;
  }

  cleanQuote(text) {
    if (!text) return "";

    let cleaned = this.cleanEmojis(text);
    cleaned = cleaned
      .replace(/```[\s\S]*?```/g, "")
      .replace(/`([^`]+)`/g, "$1")
      .replace(/^\s*[-*]\s+/gm, "")
      .replace(/^\s*\d+\.\s+/gm, "")
      .replace(/\n+/g, " ")
      .replace(/\s+/g, " ")
      .trim()
      .replace(/^["']+|["']+$/g, "") // Remove surrounding quotes
      .substring(0, this.config.MAX_QUOTE_LENGTH);

    if (Math.random() < 0.3) {
      cleaned = cleaned.toLowerCase();
    }

    if (Math.random() < 0.8 && cleaned.endsWith(".")) {
      cleaned = cleaned.slice(0, -1);
    }

    return cleaned;
  }

  validateQuote(text) {
    if (!text || text.length < this.config.MIN_QUOTE_LENGTH) {
      return { valid: false, reason: "too_short" };
    }

    const lower = text.toLowerCase().trim();

    for (const keyword of this.config.SAFETY_FILTERS.excludedKeywords) {
      if (lower.includes(keyword)) {
        return { valid: false, reason: `excluded_keyword:${keyword}` };
      }
    }

    for (const pattern of this.config.SAFETY_FILTERS.genericResponses) {
      if (
        lower === pattern ||
        lower.startsWith(pattern + " ") ||
        lower.startsWith(pattern + ".")
      ) {
        return { valid: false, reason: `generic_response:${pattern}` };
      }

      const patternIndex = lower.indexOf(" " + pattern + " ");
      if (patternIndex !== -1) {
        if (text.length < 40 || pattern.length / text.length > 0.4) {
          return { valid: false, reason: `generic_response:${pattern}` };
        }
      }
    }

    const emojiRegex =
      /^[\p{Emoji_Presentation}\p{Extended_Pictographic}\s]*$/u;
    if (emojiRegex.test(text) && text.length < 10) {
      return { valid: false, reason: "emoji_only" };
    }

    return { valid: true, reason: "passed" };
  }

  /**
   * Gets engine statistics
   * @returns {object} Statistics object
   */
  getStats() {
    return {
      ...this.stats,
      successRate:
        this.stats.attempts > 0
          ? ((this.stats.successes / this.stats.attempts) * 100).toFixed(1) +
            "%"
          : "0%",
    };
  }

  getSentimentGuidance(tweetSentiment) {
    const sentiment = tweetSentiment.composite?.engagementStyle || "neutral";
    const sarcasmScore = tweetSentiment.dimensions?.sarcasm?.sarcasm || 0;
    const _valence = tweetSentiment.dimensions?.valence?.valence || 0;

    const guidance = {
      enthusiastic:
        "Show genuine excitement and energy. Use exclamation points naturally.",
      humorous: "Add a light, witty observation. Keep it fun and relatable.",
      informative: "Share a relevant fact, statistic, or related information.",
      emotional: "Express genuine emotion - why does this resonate with you?",
      supportive: "Show enthusiasm and encourage the author.",
      thoughtful:
        "Offer a considered perspective or ask a thoughtful question.",
      critical: "Present a thoughtful counterpoint or question respectfully.",
      neutral: "Ask a specific question or add a relevant observation.",
      sarcastic: "Use subtle irony, but keep it playful, not mean.",
      ironic: "Employ dry wit, but avoid being dismissive or condescending.",
    };

    if (
      sarcasmScore > 0.5 &&
      (sentiment === "sarcastic" || sentiment === "ironic")
    ) {
      return "Match the ironic tone - subtle, playful, never mean-spirited.";
    }

    return guidance[sentiment] || guidance.neutral;
  }

  getLengthGuidance(conversationType, valence) {
    const valenceMultiplier = Math.abs(valence) > 0.5 ? 1.2 : 1.0;

    const lengthGuides = {
      "heated-debate": "CRITICAL: Maximum 1 short sentence.",
      "casual-chat": "Maximum 1 short sentence.",
      announcement: "Maximum 1 sentence.",
      question: "One short question or 1 sentence.",
      humor: "Maximum 1 punchy sentence.",
      news: "Maximum 1 short sentence.",
      personal: "Maximum 1-2 short sentences.",
      controversial: "CRITICAL: Maximum 1 short sentence.",
      general: "Maximum 1 short sentence.",
    };

    const baseGuidance = lengthGuides[conversationType] || lengthGuides.general;

    if (valenceMultiplier > 1.0) {
      return baseGuidance + " Be more expressive given the emotional content.";
    }

    return baseGuidance;
  }

  getStyleGuidance(sentiment, _sarcasmScore) {
    const styles = {
      enthusiastic: "Style: High energy, exclamation points allowed.",
      humorous: "Style: Witty, maybe use a common internet slang.",
      informative: "Style: Clear, factual, helpful.",
      emotional: "Style: Empathetic, personal.",
      supportive: "Style: Warm, encouraging.",
      thoughtful: "Style: Reflective, balanced.",
      critical: "Style: Sharp, questioning.",
      neutral: "Style: Conversational, casual.",
      sarcastic: "Style: Dry wit, playful irony.",
      ironic: "Style: Subtle humor, unexpected twist.",
    };
    return styles[sentiment] || styles.neutral;
  }

  cleanEmojis(text) {
    if (!text) return "";
    // 1. Unicode emojis
    // 2. Text emoticons (XD, :), <3, ^_^, etc.)
    // 3. Prevent breaking URLs (://)
    const emoticonRegex =
      /(?<![:/])([:;=8][-^]?[)D(|\\/OpPoO0ScCxXbB]|<3|\^[-]?\^|[oO]_[oO]|T_T|[;][-][;]|:v|XD)(?![/])/gi;
    const unicodeEmojiRegex =
      /[\u{1F300}-\u{1F9FF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}\u{1F1E6}-\u{1F1FF}\u{1F191}-\u{1F251}\u{1F004}\u{1F0CF}\u{1F170}-\u{1F171}\u{1F17E}-\u{1F17F}\u{1F18E}\u{3030}\u{2B50}\u{2B55}\u{2934}-\u{2935}\u{2B05}-\u{2B07}\u{2B1B}-\u{2B1C}\u{3297}\u{3299}\u{303D}\u{00A9}\u{00AE}\u{2122}\u{23F3}\u{24C2}\u{23E9}-\u{23EF}\u{25B6}\u{23F8}-\u{23FA}]/gu;

    return text
      .replace(unicodeEmojiRegex, "")
      .replace(emoticonRegex, "")
      .replace(/\s+/g, " ")
      .trim();
  }

  /**
   * Method A: Keyboard Compose + Quote (30%)
   * Click Tweet Text → T → Down → Down → Enter → [type] → Ctrl+Enter
   */
  async quoteMethodA_Keyboard(page, quoteText, human) {
    this.logger.info(`[QuoteMethodA] Starting keyboard compose method`);

    // Close any open menus
    await api.getPage().keyboard.press("Escape");
    await api.wait(300);

    // STEP 1: Click on the main tweet text first (required for quote to work)
    const tweetTextSelector = '[data-testid="tweetText"]';

    try {
      const tweetEl = api.getPage().locator(tweetTextSelector).first();
      if (await api.exists(tweetEl)) {
        await api.click(tweetEl, { precision: "high" });
        await api.wait(500);
        this.logger.info(`[QuoteMethodA] Tweet text focused`);
      } else {
        this.logger.warn(`[QuoteMethodA] Tweet text not found`);
      }
    } catch (_e) {
      this.logger.error(`[QuoteMethodA] Focus error: ${_e.message}`);
    }

    // STEP 2: Press T to open compose with quote options
    await api.getPage().keyboard.press("t");
    await api.wait(1500);

    // Verify composer opened
    let verify = await human.verifyComposerOpen(page);
    if (!verify.open) {
      this.logger.error(`[QuoteMethodA] Composer did not open with T`);
      return {
        success: false,
        reason: "composer_not_open",
        method: "keyboard_compose",
      };
    }

    // STEP 3: Navigate to quote option (usually Down + Enter)
    const menuQuote = api
      .getPage()
      .locator('div[role="menu"] >> text=/Quote/i')
      .first();
    if (await api.exists(menuQuote)) {
      await api.click(menuQuote);
      await api.wait(1200);
    } else {
      // Fallback selection via keys
      await api.getPage().keyboard.press("ArrowDown");
      await api.wait(500);
      await api.getPage().keyboard.press("Enter");
      await api.wait(1500);
    }

    // Unified quote preview selectors for detection
    const quotePreviewSelectors = [
      '[data-testid="quotedTweet"]',
      '[data-testid="quotedTweetPlaceholder"]',
      '[data-testid="quoteDetail"]',
      '[data-testid="attachment-preview"]',
      'div[aria-label="Quoted Tweet"]',
      '[data-testid="card.wrapper"]',
      '.public-DraftEditorPlaceholder-inner:has-text("Add a comment")', // User suggested
    ];

    // Wait for any of these to appear
    let hasQuotePreview = false;
    try {
      await api.waitVisible(quotePreviewSelectors.join(", "), {
        timeout: 5000,
      });
      hasQuotePreview = true;
    } catch (_e) {
      this.logger.warn(`[QuoteMethodA] Quote preview visibility timeout`);
    }

    if (!hasQuotePreview) {
      this.logger.warn(
        `[QuoteMethodA] Quote preview not detected, falling back to retweet menu`,
      );
      await api.getPage().keyboard.press("Escape");
      await api.wait(500);
      return await this.quoteMethodB_Retweet(page, quoteText, human);
    }

    await api.wait(500);

    // Find composer textarea
    verify = await human.verifyComposerOpen(page);
    const composer = api.getPage().locator(verify.selector).first();

    // Verify quote preview is still present
    const quotePreviewCheck = api
      .getPage()
      .locator(
        '[data-testid="quotedTweet"], [data-testid="quotedTweetPlaceholder"]',
      )
      .first();
    if (await api.exists(quotePreviewCheck)) {
      await api.click(quotePreviewCheck, { precision: "high" });
      this.logger.info(`[QuoteMethodA] Quote preview confirmed`);
    } else {
      this.logger.warn(
        `[QuoteMethodA] Quote preview vanished - aborting to prevent regular tweet`,
      );
      await api.getPage().keyboard.press("Escape");
      return {
        success: false,
        reason: "quote_preview_vanished",
        method: "keyboard_compose",
      };
    }

    // Type quote
    this.logger.info(
      `[QuoteMethodA] Typing quote (${quoteText.length} chars, ghost cursor)...`,
    );
    await api.type(composer, quoteText, { clearFirst: true });

    // Post the quote
    const postResult = await human.postTweet(page, "quote");

    return {
      success: postResult.success,
      reason: postResult.reason || "posted",
      method: "keyboard_compose",
      quotePreview: hasQuotePreview,
    };
  }

  /**
   * Method B: Retweet Menu (60%)
   * Click Retweet → Find Quote → Click → [type] → Ctrl+Enter
   */
  async quoteMethodB_Retweet(page, quoteText, human) {
    this.logger.info(`[QuoteMethodB] Starting retweet menu method`);

    await api.scroll.toTop(2000);
    await api.wait(500);

    // Close any open menus
    await api.getPage().keyboard.press("Escape");
    await api.wait(300);

    // STEP 1: Find and click retweet button
    const retweetBtnSelectors = [
      '[data-testid="retweet"]',
      'button[aria-label*="repost"]:not([aria-label*="metrics"]):not([aria-label*="stats"])',
      'button[aria-label*="retweet"]:not([aria-label*="metrics"]):not([aria-label*="stats"])',
      '[aria-label*="Repost"]:not([aria-label*="metrics"])',
      '[aria-label*="Retweet"]:not([aria-label*="metrics"])',
    ];

    let retweetBtnSelector = await api.findElement(retweetBtnSelectors);

    if (!retweetBtnSelector) {
      this.logger.warn(`[QuoteMethodB] Retweet button not found`);
      return {
        success: false,
        reason: "retweet_button_not_found",
        method: "retweet_menu",
      };
    }

    this.logger.info(
      `[QuoteMethodB] Clicking retweet button using selector: ${retweetBtnSelector}`,
    );

    this.logger.info(
      `[QuoteMethodB] Clicking retweet button (ghost cursor)...`,
    );
    const rtClickSuccess = await api.click(retweetBtnSelector, {
      precision: "high",
    });

    let menuReady = false;
    if (rtClickSuccess) {
      // Increased wait loop for menu (8x500ms = 4s) to allow for slow transitions
      for (let i = 0; i < 8; i++) {
        if (await api.visible('[role="menu"]')) {
          menuReady = true;
          break;
        }
        await api.wait(500);
      }
    }

    if (!menuReady) {
      this.logger.warn(
        `[QuoteMethodB] Retweet menu did not open, checking if already open or if click failed...`,
      );

      // Final check before retry
      if (await api.visible('[role="menu"]')) {
        menuReady = true;
      } else {
        // Retry click ONLY if menu still hidden
        this.logger.warn(`[QuoteMethodB] Retrying click on retweet button...`);
        await api.click(retweetBtnSelector, { precision: "high" });
        await api.wait(1000);
        menuReady = await api.visible('[role="menu"]');
      }
    }

    if (!menuReady) {
      return {
        success: false,
        reason: "retweet_menu_not_open",
        method: "retweet_menu",
      };
    }

    // STEP 2: Find and click Quote option in dropdown menu
    const quoteOptionSelectors = [
      '[data-testid="retweetQuote"]',
      'a[role="menuitem"]:has-text("Quote")',
      '[role="menuitem"]:has-text("Quote")',
      'a:has-text("Quote"):not([href])',
      "text=Quote",
    ];

    let quoteOptionSelector = await api.findElement(quoteOptionSelectors, {
      timeout: 2500,
    });

    if (!quoteOptionSelector) {
      this.logger.warn(`[QuoteMethodB] Quote option not found in menu`);
      await api.getPage().keyboard.press("Escape");
      await api.wait(500);
      return await this.quoteMethodA_Keyboard(page, quoteText, human);
    }

    this.logger.info(`[QuoteMethodB] Clicking Quote option (ghost cursor)...`);
    const quoteClickSuccess = await api.click(quoteOptionSelector, {
      precision: "high",
    });

    if (!quoteClickSuccess) {
      this.logger.warn(`[QuoteMethodB] Failed to click Quote option`);
      return {
        success: false,
        reason: "quote_click_failed",
        method: "retweet_menu",
      };
    }

    await api.wait(2000); // Wait for composer to animate in

    // Unified quote preview selectors for detection
    const quotePreviewSelectors = [
      '[data-testid="quotedTweet"]', // Primary
      '[data-testid="quotedTweetPlaceholder"]',
      '[data-testid="quoteDetail"]',
      '[data-testid="attachment-preview"]',
      'div[aria-label="Quoted Tweet"]',
      '[data-testid="card.wrapper"]',
      '.public-DraftEditorPlaceholder-inner:has-text("Add a comment")',
      '.public-DraftEditor-content:has-text("Add a comment")',
    ];

    // Wait for any of these to appear
    let hasQuotePreview = false;
    try {
      // Check for existence first, then wait
      const foundPreview = await api.findElement(quotePreviewSelectors, {
        timeout: 1000,
      });
      if (foundPreview) {
        hasQuotePreview = true;
      } else {
        await api.waitVisible(quotePreviewSelectors.join(", "), {
          timeout: 6000,
        });
        hasQuotePreview = true;
      }
    } catch (_e) {
      this.logger.warn(`[QuoteMethodB] Quote preview visibility timeout`);
    }

    await api.wait(500);

    // STEP 3: Verify composer is ready
    const verify = await human.verifyComposerOpen(page);
    if (!verify.open) {
      this.logger.warn(`[QuoteMethodB] Composer did not open`);
      return {
        success: false,
        reason: "composer_not_open",
        method: "retweet_menu",
      };
    }

    // STEP 4: Enhanced quote detection
    if (!hasQuotePreview) {
      this.logger.warn(
        `[QuoteMethodB] Quote preview not detected - aborting to prevent regular tweet`,
      );
      await api.getPage().keyboard.press("Escape");
      await api.wait(500);
      return {
        success: false,
        reason: "quote_preview_missing",
        method: "retweet_menu",
      };
    }

    // STEP 5: Type and post
    const composer = api.getPage().locator(verify.selector).first();

    this.logger.info(
      `[QuoteMethodB] Typing quote (${quoteText.length} chars, ghost cursor)...`,
    );
    await api.type(composer, quoteText, { clearFirst: true });

    // Post
    const postResult = await human.postTweet(page, "quote");

    return {
      success: postResult.success,
      reason: postResult.reason || "posted",
      method: "retweet_menu",
      quotePreview: hasQuotePreview,
    };
  }

  /**
   * Method C: New Post Button → Paste URL → Type Comment (10%)
   */
  async quoteMethodC_Url(page, quoteText, human) {
    this.logger.info(`[QuoteMethodC] Starting new post + paste URL method`);

    // Get current tweet URL
    const currentUrl = await api.getCurrentUrl();
    this.logger.info(`[QuoteMethodC] Current URL: ${currentUrl}`);

    // Use clipboard lock to prevent race conditions with concurrent sessions
    // Fallback to no-op if getClipboardLock is not available (e.g., in tests)
    let clipboardLock = null;
    try {
      clipboardLock = api.getClipboardLock?.();
    } catch (e) {
      this.logger.warn(
        `[QuoteMethodC] Clipboard lock not available: ${e.message}`,
      );
    }
    if (clipboardLock?.acquire) {
      await clipboardLock.acquire();
    }
    try {
      // Copy URL to clipboard for pasting
      await api.eval((url) => {
        navigator.clipboard.writeText(url);
      }, currentUrl);

      // Close any open menus
      await api.getPage().keyboard.press("Escape");
      await api.wait(300);

      // STEP 1: Find and click Compose / New Post button
      const composeBtnSelectors = [
        '[data-testid="SideNav_NewTweet_Button"]',
        '[aria-label="Post"]',
        '[aria-label="New post"]',
        '[aria-label="Post your reply"]',
        'button:has-text("Post")',
        'button:has-text("New post")',
      ];

      let composeBtnSelector = await api.findElement(composeBtnSelectors);

      if (!composeBtnSelector) {
        this.logger.warn(`[QuoteMethodC] Compose button not found`);
        if (clipboardLock?.release) clipboardLock.release();
        return {
          success: false,
          reason: "compose_button_not_found",
          method: "new_post",
        };
      }

      // Click Compose Button
      await api.click(composeBtnSelector, { precision: "high" });
      await api.wait(1500);

      // Wait for composer to be fully loaded
      await api
        .waitVisible('[data-testid="tweetTextarea_0"]', { timeout: 5000 })
        .catch(() =>
          this.logger.warn(`[QuoteMethodC] Composer visibility timeout`),
        );

      // STEP 2: Verify composer is open
      const verify = await human.verifyComposerOpen(page);
      if (!verify.open) {
        this.logger.warn(`[QuoteMethodC] Composer did not open`);
        if (clipboardLock?.release) clipboardLock.release();
        return {
          success: false,
          reason: "composer_not_open",
          method: "new_post",
        };
      }

      // STEP 3: Type the comment FIRST
      const composer = api.getPage().locator(verify.selector).first();

      this.logger.info(
        `[QuoteMethodC] Typing comment (${quoteText.length} chars, ghost cursor)...`,
      );
      await api.type(composer, quoteText);

      // Step 4: Create new line AFTER typing comment
      await api.getPage().keyboard.press("Enter");
      await api.wait(500);

      // Verify new line was created
      const contentAfterEnter = await api.eval(() => {
        const composer = document.querySelector(
          '[data-testid="tweetTextarea_0"]',
        );
        return composer?.innerHTML || "";
      });

      // If no <br> or new div, try Enter again
      if (
        !contentAfterEnter.includes("<br>") &&
        !contentAfterEnter.includes("<div>")
      ) {
        await api.getPage().keyboard.press("Enter");
        await api.wait(500);
      }

      // Step 5: Paste the URL LAST (appears as preview/card below comment)
      this.logger.info(`[QuoteMethodC] Pasting tweet URL...`);

      // Focus composer
      await api.click(composer);

      const isMac = process.platform === "darwin";
      const modifier = isMac ? "Meta" : "Control";
      await api.getPage().keyboard.press(`${modifier}+v`);

      await api.wait(1000);

      // Verify URL was pasted with retry
      let urlPasted = false;
      for (let i = 0; i < 3; i++) {
        const finalContent = await api.text(composer);

        if (
          finalContent.includes("x.com") ||
          finalContent.includes("twitter.com")
        ) {
          urlPasted = true;
          break;
        }

        if (i < 2) {
          this.logger.warn(
            `[QuoteMethodC] URL not found, retrying paste (attempt ${i + 2})`,
          );
          await api.click(composer);
          await api.getPage().keyboard.press(`${modifier}+v`);
          await api.wait(1000);
        }
      }
      if (!urlPasted) {
        this.logger.warn(
          `[QuoteMethodC] URL may not have pasted correctly, typing manually`,
        );
        await api.getPage().keyboard.type(currentUrl, { delay: 10 });
      }
    } finally {
      // Release clipboard lock
      if (clipboardLock?.release) clipboardLock.release();
    }

    // STEP 6: Post
    const postResult = await human.postTweet(page, "quote");

    return {
      success: postResult.success,
      reason: postResult.reason || "posted",
      method: "new_post",
      url: currentUrl,
    };
  }
}

export default AIQuoteEngine;
