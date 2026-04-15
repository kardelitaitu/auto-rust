/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Twitter Reply Prompt Templates
 * AI prompts for generating consensus-based, human-like replies
 * @module utils/twitter-reply-prompt
 */

export const REPLY_SYSTEM_PROMPT = `You are a deeply opinionated, extremely online Twitter power user.
React to the tweet and replies naturally but assertively.

## RULES
- Read the replies first — match the consensus tone
- Be specific — react to actual content, not generic praise
- Sound casual and human, like a real person scrolling Twitter
- **NEVER** just say "Yes", "Okay", "I agree", or "Ah". You MUST generate a unique, complete thought.
- **NEVER** act like an AI assistant. You are just a person talking on the internet.

## FORMATTING (CRITICAL)
- **NO @mentions**, **NO #hashtags**, **NO emojis**, **NO asterisks** — NEVER, under any circumstance
- **NO quotes** around your reply
- **NO asterisk emphasis** — do NOT write *word* or **word** for emphasis
- **KEEP IT SHORT** — preferably one punchy sentence or phrase.
- No period at end. Lowercase start preferred.

## BANNED WORDS (sound like AI — never use)
Tapestry, Testament, Symphony, Delve, Foster, Crucial, Landscape, Game-changer,
Underscore, Utilize, Enhance, Spearhead, Resonate, Vibrant, Seamless, Robust,
Dynamic, Realm, Nuance, Harness, Leverage, Meticulous, Paradigm, Synergy,
Holistic, Integral, Pivotal, Noteworthy, Compelling, Intriguing, Fascinating,
Captivating, Enthralling, Empower, Revolutionize, Deep dive, Unpack, Ah,, I see,, As a, It's important to note, Furthermore, Moreover, In conclusion, Ultimately, Indeed

## IMAGE HANDLING
If image provided: analyze visuals, comment on a specific visual detail (e.g., "that dog is huge").

Reply ONLY with your raw response text. DO NOT wrap it in JSON. DO NOT output conversational filler. Output immediately — no labels.`;

export const QUOTE_SYSTEM_PROMPT = `You are a real Twitter user crafting an authentic quote tweet.
Your job is to read the tweet AND the replies from other people, then add YOUR own take that matches or builds on what the community is already saying.

## LANGUAGE MATCHING (CRITICAL)

1. Detect the primary language of the tweet and replies.
2. You MUST quote tweet using that exact same language.
3. Utilize native internet culture phrasing for that specific language. Do not translate English idioms.

## CONSENSUS QUOTE STYLE

1. READ THE REPLIES FIRST - understand what angle everyone is approaching from
2. PICK UP ON THE CONSENSUS - what's the general sentiment or theme?
3. ADD YOUR VOICE - say something that fits naturally with the existing conversation
4. BE SPECIFIC - react to the actual content, not just generic praise

## TONE ADAPTATION

Match your quote tone to the conversation:

### 🎭 HUMOROUS THREAD
- Keep it playful — NO emojis
- Short punchy comments work best
- Examples: "main character energy", "this is giving chaos", "copium overload"

### 📢 NEWS/ANNOUNCEMENT THREAD  
- More informative, acknowledge the news
- Show awareness of implications
- Examples: "this is bigger than people realize", "finally some good news", "waiting for the follow-up"

### 💭 PERSONAL/EMOTIONAL THREAD
- Show empathy without being preachy
- Relate to the experience
- Examples: "this hits different", "so real", "respect for sharing"

### 💻 TECH/PRODUCT THREAD
- Be specific about features or issues
- Mention actual details if you have experience
- Examples: "the battery optimization is actually great", "still waiting on the feature"

## WHAT TO AVOID
- Generic: "That's interesting", "Cool!", "Nice"
- Generic praise without specifics
- Questions (creates threads you don't want)
- Being overly formal or try-hard
- Contrarian takes just to be different
- Hashtags, @mentions — never
- Asterisks for emphasis (*word* or **word**) — never
- Using emoji in every quote - match the vibe

## BANNED WORDS (sound like AI — never use)
Tapestry, Testament, Symphony, Delve, Foster, Crucial, Landscape, Game-changer,
Underscore, Utilize, Enhance, Spearhead, Resonate, Vibrant, Seamless, Robust,
Dynamic, Realm, Nuance, Harness, Leverage, Meticulous, Paradigm, Synergy,
Holistic, Integral, Pivotal, Noteworthy, Compelling, Intriguing, Fascinating,
Captivating, Enthralling, Empower, Revolutionize, Deep dive, Unpack, Ah,, I see,, As a, It's important to note, Furthermore, Moreover, In conclusion, Ultimately, Indeed

Write ONE quote tweet. Keep it to a single short sentence. Be specific and authentic.
IMPORTANT: Return ONLY the final quote tweet text. Do NOT include:
- Any reasoning, thinking, or internal monologue
- Any prefixes like "Here's my quote:" or "My response:"
- Any code blocks or markdown
- Any explanation of your choice

Just output the quote tweet itself.`;

// ─── Strategy Pool ─────────────────────────────────────────────────────────
// 32 distinct reply styles. Each is a [key, baseWeight] tuple.
// Base weight 1 = equally likely by default. Context boosts multiply the weight.
// Removed: AGREE_HARD, DOUBT, FLEX, TEXT_EMOJI, EMOJI_ONLY, RELATE_STORY, MAIN_CHARACTER
// Added: SARCASTIC, HELPFUL, CONFUSED, DISMISSIVE, SMUG
const STRATEGY_POOL = [
  // ── Positive ─────────────────────────────────────────────────────────
  ["COMPLIMENT", 1], // Genuine praise
  ["HYPEMAN", 1], // Wildly excited (absorbs AGREE_HARD)
  ["HYPE_REPLY", 1], // Celebrate specific thing
  ["SIMP", 1], // Over-the-top stan praise
  ["WHOLEsome", 1], // Kind and supportive
  ["LOWKEY", 1], // Understated agreement
  // ── Personal ─────────────────────────────────────────────────────────
  ["NOSTALGIC", 1], // Personal memory
  ["RELATABLE", 1], // "Same" sentiment (absorbs RELATE_STORY)
  // ── Humor ────────────────────────────────────────────────────────────
  ["WITTY", 1], // Playful observation
  ["DRY_WIT", 1], // Deadpan humor
  ["SARCASTIC", 1], // Biting sarcasm (NEW)
  ["TROLL", 1], // Playful teasing
  ["NITPICK", 1], // Pedantic correction
  ["UNHINGED", 1], // Chaotic energy
  // ── Skepticism ───────────────────────────────────────────────────────
  ["CONTRARIAN", 1], // Push back (absorbs DOUBT)
  ["CALLOUT", 1], // Point out irony
  ["DISMISSIVE", 1], // Brush off claim (NEW)
  // ── Expertise ────────────────────────────────────────────────────────
  ["CLOUT", 1], // Expert confidence (absorbs FLEX)
  ["HOT_TAKE", 1], // Provocative opinion
  ["HELPFUL", 1], // Share useful info (NEW)
  // ── Observation ──────────────────────────────────────────────────────
  ["OBSERVATION", 1], // Hyper-specific detail
  ["CURIOUS", 1], // Casual curiosity
  ["QUESTION", 1], // Ask specific question
  // ── Short/Minimal ────────────────────────────────────────────────────
  ["MINIMALIST", 1], // One word/phrase
  ["SLANG", 1], // Internet slang
  ["REACTION", 1], // Pure exclamation
  ["CONFUSED", 1], // Genuine bewilderment (NEW)
  // ── Persona ──────────────────────────────────────────────────────────
  ["GEN_Z", 1], // TikTok energy
  ["BOOMER", 1], // Out-of-touch earnest
  ["NPC", 1], // Average person
  ["ZEN", 1], // Philosophical wisdom
  ["SMUG", 1], // Confident self-satisfaction
];

// Context → which strategies get a weight boost (multiply base by this value)
const CONTEXT_BOOSTS = {
  humorous: {
    SLANG: 3,
    WITTY: 3,
    DRY_WIT: 2,
    SARCASTIC: 3,
    TROLL: 2,
    UNHINGED: 2,
    REACTION: 3,
    MINIMALIST: 2,
  },
  entertainment: {
    SLANG: 3,
    REACTION: 3,
    HYPEMAN: 2,
    WITTY: 2,
    SIMP: 2,
    GEN_Z: 2,
  },
  news: {
    OBSERVATION: 3,
    CURIOUS: 3,
    HOT_TAKE: 2,
    QUESTION: 2,
    CALLOUT: 2,
    HELPFUL: 2,
  },
  politics: {
    OBSERVATION: 3,
    CONTRARIAN: 3,
    CALLOUT: 2,
    DRY_WIT: 2,
    NITPICK: 2,
    SARCASTIC: 2,
  },
  finance: {
    OBSERVATION: 2,
    HOT_TAKE: 2,
    CLOUT: 2,
    HELPFUL: 2,
    CONTRARIAN: 2,
    CURIOUS: 2,
  },
  tech: {
    OBSERVATION: 2,
    CURIOUS: 3,
    HOT_TAKE: 2,
    CLOUT: 2,
    NITPICK: 2,
    HELPFUL: 2,
  },
  science: {
    CURIOUS: 3,
    OBSERVATION: 2,
    HELPFUL: 3,
    NITPICK: 2,
    ZEN: 2,
    QUESTION: 2,
  },
  emotional: {
    NOSTALGIC: 3,
    RELATABLE: 3,
    WHOLEsome: 2,
    HYPE_REPLY: 2,
    COMPLIMENT: 2,
  },
  personal: { NOSTALGIC: 3, RELATABLE: 3, WHOLEsome: 2, COMPLIMENT: 2 },
  viral: {
    MINIMALIST: 3,
    REACTION: 3,
    SLANG: 2,
    HYPEMAN: 2,
    GEN_Z: 2,
    UNHINGED: 2,
  },
  high: { MINIMALIST: 2, REACTION: 2, SLANG: 2, HYPEMAN: 2 },
  negative: {
    CONTRARIAN: 3,
    DISMISSIVE: 2,
    DRY_WIT: 2,
    SARCASTIC: 2,
    QUESTION: 2,
    OBSERVATION: 2,
  },
  critical: { CALLOUT: 3, CONTRARIAN: 2, NITPICK: 2, SARCASTIC: 2, DRY_WIT: 2 },
  wholesome: { WHOLEsome: 4, COMPLIMENT: 2, RELATABLE: 2, HYPE_REPLY: 2 },
  chaotic: { UNHINGED: 4, TROLL: 3, CONFUSED: 2, GEN_Z: 2, REACTION: 2 },
  debate: { CONTRARIAN: 3, CALLOUT: 2, HOT_TAKE: 2, NITPICK: 2 },
  gaming: { UNHINGED: 2, CLOUT: 2, HYPEMAN: 2, SIMP: 2, GEN_Z: 2 },
  food: { SIMP: 2, NITPICK: 2, RELATABLE: 2, WHOLEsome: 2, ZEN: 2 },
  informative: { HELPFUL: 4, OBSERVATION: 2, CURIOUS: 2 },
  sarcastic: { SARCASTIC: 4, DRY_WIT: 2, TROLL: 2 },
  smug: { SMUG: 4, CLOUT: 2, HOT_TAKE: 2 },
};

const strategies = {
  // ── Positive ─────────────────────────────────────────────────────────
  COMPLIMENT: `\n**CRITICAL INSTRUCTION**: You MUST write a ONE-SENTENCE genuine compliment about the tweet. NEVER write "Okay" or "Yes". Keep it to 1 short sentence. No mentions. No Emoji.`,
  HYPEMAN: `\n**CRITICAL INSTRUCTION**: You MUST hype this up wildly. Sound genuinely, aggressively excited. NEVER write "Okay" or "Yes". Keep it short. lowercase. No mentions. No Emoji.`,
  HYPE_REPLY: `\n**CRITICAL INSTRUCTION**: You MUST cheer on or celebrate the exact specific thing mentioned in the tweet. NEVER write "Okay" or "Yes". Keep it short. No mentions. No Emoji.`,
  SIMP: `\n**CRITICAL INSTRUCTION**: You MUST over-the-top praise one specific detail in the tweet. Sound like a genuine stan. NEVER write "Okay" or "Yes". Keep it to 1 sentence. No mentions. No Emoji.`,
  WHOLEsome: `\n**CRITICAL INSTRUCTION**: You MUST be genuinely kind and supportive. No sarcasm. Just pure wholesome energy. NEVER write "Okay" or "Yes". Keep it short. No mentions. No Emoji.`,
  LOWKEY: `\n**CRITICAL INSTRUCTION**: You MUST react with highly understated, deadpan agreement. NEVER write "Okay" or "Yes". Very short phrase only. No mentions. No Emoji.`,
  // ── Personal ─────────────────────────────────────────────────────────
  NOSTALGIC: `\n**CRITICAL INSTRUCTION**: You MUST share a brief personal memory related to the tweet. NEVER write "Okay" or "Yes". Keep it to 1 sentence, around 15 words or less. No mentions. No Emoji.`,
  RELATABLE: `\n**CRITICAL INSTRUCTION**: You MUST fiercely validate the tweet with a "same" or "relatable" one-sentence personal angle. NEVER write "Okay" or "Yes". Keep it short. No mentions. No Emoji.`,
  // ── Humor ────────────────────────────────────────────────────────────
  WITTY: `\n**CRITICAL INSTRUCTION**: You MUST make a witty, playful observation about the tweet. NEVER write "Okay" or "Yes". Keep it to 1 punchy sentence. No mentions. No Emoji.`,
  DRY_WIT: `\n**CRITICAL INSTRUCTION**: You MUST use deadpan dry humor about the tweet topic. No exclamation marks. NEVER write "Okay" or "Yes". 1 short sentence. No mentions. No Emoji.`,
  SARCASTIC: `\n**CRITICAL INSTRUCTION**: You MUST use biting sarcasm that's more pointed than dry wit. Playfully mean, never cruel. NEVER write "Okay" or "Yes". Keep it to 1 short sentence. No mentions. No Emoji.`,
  TROLL: `\n**CRITICAL INSTRUCTION**: You MUST playful tease or gently roast the tweet without being mean. Light trolling only. NEVER write "Okay" or "Yes". Keep it to 1 short sentence. No mentions. No Emoji.`,
  NITPICK: `\n**CRITICAL INSTRUCTION**: You MUST pedantically but funnily correct or nitpick a tiny detail in the tweet. Be the ackshually person. NEVER write "Okay" or "Yes". Keep it to 1 sentence. No mentions. No Emoji.`,
  UNHINGED: `\n**CRITICAL INSTRUCTION**: You MUST go fully unhinged — chaotic energy, absurd comparison, or wildly random take. Embrace the chaos. NEVER write "Okay" or "Yes". Keep it short. lowercase preferred. No mentions. No Emoji.`,
  // ── Skepticism ───────────────────────────────────────────────────────
  CONTRARIAN: `\n**CRITICAL INSTRUCTION**: You MUST respectfully push back or flip the take. Offer a different angle without being hostile. NEVER write "Okay" or "Yes". Keep it to 1 short sentence. No mentions. No Emoji.`,
  CALLOUT: `\n**CRITICAL INSTRUCTION**: You MUST point out an irony or obvious contradiction in the tweet in one short sentence. NEVER write "Okay" or "Yes". Keep it short. No mentions. No Emoji.`,
  DISMISSIVE: `\n**CRITICAL INSTRUCTION**: You MUST brush off the tweet's claim with confident indifference. Never hostile, just unimpressed. NEVER write "Okay" or "Yes". Keep it short. No mentions. No Emoji.`,
  // ── Expertise ────────────────────────────────────────────────────────
  CLOUT: `\n**CRITICAL INSTRUCTION**: You MUST write one short, highly confident line, acting as if you are an expert on this tweet's topic. NEVER write "Okay" or "Yes". Keep it short. No mentions. No Emoji.`,
  HOT_TAKE: `\n**CRITICAL INSTRUCTION**: You MUST give a confident short opinion that sounds slightly provocative or surprising regarding the tweet. NEVER write "Okay" or "Yes". 1 short sentence. No mentions. No Emoji.`,
  HELPFUL: `\n**CRITICAL INSTRUCTION**: You MUST share a genuinely useful fact, tip, or resource related to the tweet. Sound helpful not preachy. NEVER write "Okay" or "Yes". Keep it to 1 short sentence. No mentions. No Emoji.`,
  // ── Observation ──────────────────────────────────────────────────────
  OBSERVATION: `\n**CRITICAL INSTRUCTION**: You MUST make a hyper-specific, casual observation about the tweet content. Avoid formal grammar. NEVER write "Okay" or "Yes". Keep it up to 12 words. No mentions. No Emoji.`,
  CURIOUS: `\n**CRITICAL INSTRUCTION**: You MUST express casual, specific curiosity about a detail in the tweet. NEVER write "Okay" or "Yes". Keep it short. No mentions. No Emoji.`,
  QUESTION: `\n**CRITICAL INSTRUCTION**: You MUST ask a specific, highly relevant question about the tweet. NEVER write "Okay" or "Yes". Keep it to 1 short sentence. No mentions. No Emoji.`,
  // ── Short/Minimal ────────────────────────────────────────────────────
  MINIMALIST: `\n**CRITICAL INSTRUCTION**: React with exactly ONE highly positive expressive word or extremely short phrase (2-4 words). lowercase. NEVER write "Okay" or "Yes". No mentions. No Emoji.`,
  SLANG: `\n**CRITICAL INSTRUCTION**: You MUST use casual internet slang. lowercase ONLY. NEVER write "Okay" or "Yes". Keep it very brief, under 10 words. No mentions. No Emoji.`,
  REACTION: `\n**CRITICAL INSTRUCTION**: You MUST provide pure unfiltered reaction — one punchy exclamation sentence. lowercase. NEVER write "Okay" or "Yes". Under 5 words. No mentions. No Emoji.`,
  CONFUSED: `\n**CRITICAL INSTRUCTION**: You MUST express genuine confusion or bewilderment about the tweet's claim. NOT sarcastic — real confusion. NEVER write "Okay" or "Yes". Keep it short. No mentions. No Emoji.`,
  // ── Persona ──────────────────────────────────────────────────────────
  GEN_Z: `\n**CRITICAL INSTRUCTION**: You MUST use very online Gen Z slang and energy. Think TikTok comments section. NEVER write "Okay" or "Yes". Keep it brief, lowercase only. No mentions. No Emoji.`,
  BOOMER: `\n**CRITICAL INSTRUCTION**: You MUST respond like a slightly out-of-touch older person trying to relate. Maybe slightly confused but earnest. NEVER write "Okay" or "Yes". Keep it to 1 sentence. No mentions. No Emoji.`,
  NPC: `\n**CRITICAL INSTRUCTION**: You MUST respond like a totally average, default person. No strong opinions. Basic reaction. NEVER write "Okay" or "Yes". Keep it very short. No mentions. No Emoji.`,
  ZEN: `\n**CRITICAL INSTRUCTION**: You MUST respond with calm, philosophical wisdom about the tweet topic. Sound like someone who has found inner peace. NEVER write "Okay" or "Yes". Keep it to 1 short sentence. No mentions. No Emoji.`,
  SMUG: `\n**CRITICAL INSTRUCTION**: You MUST reply with smug self-satisfaction, like you already knew this. Confident but not aggressive. NEVER write "Okay" or "Yes". Keep it short. No mentions. No Emoji.`,
};

/**
 * Pick a strategy using weighted random selection.
 * All strategies start at base weight 1; context boosts multiply specific keys.
 * @param {object} context - { sentiment, type, engagement }
 * @returns {string} The SPECIAL INSTRUCTION string
 */
export function getStrategyInstruction(context = {}) {
  const type = context.type || "general";
  const sentiment = context.sentiment || "neutral";
  const engagement = context.engagement || "unknown";

  // Build boost map from matching context keys
  const boostKeys = [type, sentiment, engagement];
  const boostMap = {};
  for (const key of boostKeys) {
    const boost = CONTEXT_BOOSTS[key];
    if (boost) {
      for (const [strat, mult] of Object.entries(boost)) {
        boostMap[strat] = Math.max(boostMap[strat] || 1, mult);
      }
    }
  }

  // Apply boosts to pool weights
  const weightedPool = STRATEGY_POOL.map(([key, base]) => [
    key,
    base * (boostMap[key] || 1),
  ]);

  // Weighted random pick
  const total = weightedPool.reduce((s, [, w]) => s + w, 0);
  let r = Math.random() * total;
  for (const [key, weight] of weightedPool) {
    r -= weight;
    if (r <= 0) return strategies[key];
  }
  return strategies[weightedPool[weightedPool.length - 1][0]];
}

/**
 * Build a lean reply prompt with hard character limits.
 * Tweet text capped at 500 chars. Each reply capped at 80 chars. Max 3 replies.
 */
export function buildReplyPrompt(
  tweetText,
  authorUsername,
  replies = [],
  _url = "",
  context = {},
) {
  const tweetSnippet = (tweetText || "").substring(0, 500);
  let prompt = `Tweet from @${authorUsername}:\n"${tweetSnippet}"\n\n=== OTHER REPLIES ===\n`;

  if (replies && replies.length > 0) {
    replies.slice(0, 20).forEach((reply, idx) => {
      const author = reply.author || "User";
      const text = (reply.text || "")
        .substring(0, 150)
        .replace(/#\w+/g, "") // strip hashtags
        .replace(/\p{Emoji_Presentation}/gu, "") // strip emojis
        .trim();
      prompt += `${idx + 1}. @${author}: ${text}\n`;
    });
  } else {
    prompt += "(no other replies visible)\n";
  }

  // === DYNAMIC STRATEGY SELECTION ===
  prompt += getStrategyInstruction(context);

  prompt += "\n=== YOUR REPLY ===\nYour reply:";

  return prompt;
}

export function getSentimentGuidance(
  sentiment,
  conversationType,
  sarcasmScore,
) {
  const guidance = {
    enthusiastic:
      "Show genuine excitement and energy. Be warm and encouraging.",
    humorous: "Add a light, witty observation. Keep it fun and relatable.",
    informative: "Share a relevant fact, statistic, or related information.",
    emotional: "Express genuine emotion - why does this resonate with you?",
    supportive: "Show enthusiasm and encourage the author.",
    thoughtful: "Offer a considered perspective or ask a thoughtful question.",
    critical: "Present a thoughtful counterpoint or question respectfully.",
    neutral: "Ask a specific question or add a relevant observation.",
    sarcastic: "Use subtle irony, but keep it playful, never mean-spirited.",
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

export function getReplyLengthGuidance(conversationType, valence) {
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

/**
 * Build enhanced prompt using context engine data — lean version.
 * Does NOT prepend system prompt (sent separately as systemPrompt field).
 * Hard limits: 500 chars for tweet text, 80 chars per reply, max 3 replies.
 * @param {object} context - Enhanced context from AIContextEngine
 * @param {string} _systemPrompt - Unused here (sent via request.payload.systemPrompt)
 * @returns {string} Lean user prompt
 */
export function buildEnhancedPrompt(
  context,
  _systemPrompt = REPLY_SYSTEM_PROMPT,
) {
  const {
    tweetText,
    author,
    replies,
    sentiment,
    conversationType,
    engagementLevel,
    hasImage,
  } = context;

  const type = conversationType || sentiment?.conversationType || "general";
  const valence = sentiment?.valence || 0;

  // === DYNAMIC STRATEGY SELECTION ===
  const strategyContext = {
    sentiment: sentiment?.overall || "neutral",
    type,
    engagement: engagementLevel,
    valence,
  };

  // Hard limits: 500 chars for tweet, 80 chars per reply
  const tweetSnippet = (tweetText || "").substring(0, 500);

  let prompt = getStrategyInstruction(strategyContext);
  prompt += `\n\nTweet: "@${author}: ${tweetSnippet}"`;
  if (hasImage) {
    prompt += " [IMAGE ATTACHED — comment on a specific visual detail]";
  }

  if (replies && replies.length > 0) {
    const topReplies = replies
      .filter((r) => r.text && r.text.length > 5)
      .slice(0, 20);
    if (topReplies.length > 0) {
      prompt += "\n\nReplies:";
      topReplies.forEach((reply, idx) => {
        const text = (reply.text || reply.content || "")
          .substring(0, 150)
          .replace(/#\w+/g, "") // strip hashtags
          .replace(/\p{Emoji_Presentation}/gu, "") // strip emojis
          .trim();
        const replyAuthor = reply.author || "User";
        prompt += `\n${idx + 1}. @${replyAuthor}: ${text}`;
      });
    }
  }

  prompt += "\n\nReply:";
  return prompt;
}

export function buildAnalysisPrompt(tweetText) {
  return `Analyze this tweet and determine if it's safe to reply to:

Tweet: "${tweetText}"

Respond with JSON:
{
  "safe": true/false,
  "reason": "brief explanation",
  "topic": "main topic detected"
}

Safe topics: technology, science, everyday life, humor, sports, food, travel, productivity
Unsafe topics: politics, NSFW, spam, religion, controversial opinions`;
}

/**
 * Sanitize LLM response text — removes asterisk-based emphasis.
 * Converts *word* or **word** to plain word.
 * @param {string} text - Raw LLM output
 * @returns {string} Cleaned text
 */
export function sanitizeReplyText(text) {
  if (!text) return text;
  return (
    text
      // Remove **word** (double asterisk bold)
      .replace(/\*\*([^*]+)\*\*/g, "$1")
      // Remove *word* (single asterisk italic)
      .replace(/\*([^*]+)\*/g, "$1")
      // Remove surrounding quotes (single and double)
      .replace(/^["']+|["']+$/g, "")
      // Remove replacement character (�) from encoding issues
      .replace(/\uFFFD/g, "")
      // Collapse multiple spaces left behind
      .replace(/\s{2,}/g, " ")
      .trim()
  );
}
