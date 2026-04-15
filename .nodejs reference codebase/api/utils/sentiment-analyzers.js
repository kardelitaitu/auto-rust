/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Individual Sentiment Dimension Analyzers (Phase 2)
 * Each analyzer is independently testable, cacheable, and unit-testable
 * @module utils/sentiment-analyzers
 */

import SentimentData from './sentiment-data.js';

/**
 * ValenceAnalyzer - Analyzes positive/negative intensity in text
 */
export class ValenceAnalyzer {
    /**
     * Creates a new ValenceAnalyzer instance
     */
    constructor() {
        this.positiveWords = this.buildWordWeights(SentimentData.POSITIVE_LEXICON);
        this.negativeWords = this.buildWordWeights(SentimentData.NEGATIVE_LEXICON);
    }

    buildWordWeights(lexicon) {
        const weights = new Map();
        for (const [intensity, words] of Object.entries(lexicon)) {
            const weight =
                intensity === 'strong'
                    ? 1.0
                    : intensity === 'moderate'
                      ? 0.6
                      : intensity === 'weak'
                        ? 0.3
                        : 0.5;

            for (const word of words) {
                weights.set(word.toLowerCase(), weight);
            }
        }
        return weights;
    }

    analyze(text) {
        const words = text.toLowerCase().split(/\s+/);
        let positiveScore = 0;
        let negativeScore = 0;
        let totalWeight = 0;
        const matchedWords = { positive: [], negative: [] };

        for (const word of words) {
            const cleanWord = word.replace(/[^\w]/g, '');

            if (this.positiveWords.has(cleanWord)) {
                const weight = this.positiveWords.get(cleanWord);
                positiveScore += weight;
                matchedWords.positive.push(cleanWord);
                totalWeight += 1;
            }
            if (this.negativeWords.has(cleanWord)) {
                const weight = this.negativeWords.get(cleanWord);
                negativeScore += weight;
                matchedWords.negative.push(cleanWord);
                totalWeight += 1;
            }
        }

        // Calculate net valence (-1 to +1)
        let valence = 0;
        if (totalWeight > 0) {
            valence = (positiveScore - negativeScore) / totalWeight;
        }

        // Emoji sentiment boost
        const emojiBoost = this.analyzeEmojiSentiment(text);
        valence = valence * 0.7 + emojiBoost * 0.3;

        return {
            valence: Math.max(-1, Math.min(1, valence)),
            positiveMatches: positiveScore,
            negativeMatches: negativeScore,
            totalWeight,
            emojiContribution: emojiBoost,
            matchedWords,
            confidence: totalWeight > 3 ? 'high' : totalWeight > 1 ? 'medium' : 'low',
        };
    }

    analyzeEmojiSentiment(text) {
        let score = 0;
        let count = 0;

        for (const [emoji, sentiment] of Object.entries(SentimentData.EMOJI_SENTIMENT)) {
            if (text.includes(emoji)) {
                score += sentiment;
                count++;
            }
        }

        return count > 0 ? score / count : 0;
    }
}

// ============================================================================
// AROUSAL ANALYZER (Calm/Excited Energy)
// ============================================================================
/**
 * ArousalAnalyzer - Analyzes energy/excitement level in text
 */
export class ArousalAnalyzer {
    analyze(text) {
        let score = 0.3; // Default: slightly calm
        const markers = {
            exclamations: 0,
            capsWords: 0,
            repetitions: 0,
            highEnergyWords: 0,
            lowEnergyWords: 0,
            emojis: 0,
        };

        // Exclamation marks
        markers.exclamations = (text.match(/!/g) || []).length;
        score += Math.min(0.3, markers.exclamations * 0.08);

        // All-caps words (not single letters)
        const words = text.split(/\s+/);
        const capsWords = words.filter((w) => w.length > 2 && w === w.toUpperCase());
        markers.capsWords = capsWords.length;
        const capsRatio = capsWords.length / Math.max(words.length, 1);
        score += Math.min(0.25, capsRatio * 0.5);

        // Repetition ("soooo", "omgggg")
        const repetitions = (text.match(/(.)\1{2,}/g) || []).length;
        markers.repetitions = repetitions;
        score += Math.min(0.2, repetitions * 0.1);

        // High-energy words
        for (const marker of SentimentData.AROUSAL_MARKERS.high.markers) {
            if (text.toLowerCase().includes(marker)) {
                markers.highEnergyWords++;
                score += 0.08;
            }
        }

        // Low-energy words reduce score
        for (const marker of SentimentData.AROUSAL_MARKERS.low.markers) {
            if (text.toLowerCase().includes(marker)) {
                markers.lowEnergyWords++;
                score -= 0.05;
            }
        }

        // Emoji arousal
        const emojiArousal = this.analyzeEmojiArousal(text);
        markers.emojis = emojiArousal.count;
        score = score * 0.7 + emojiArousal.score * 0.3;

        return {
            arousal: Math.max(0, Math.min(1, score)),
            ...markers,
            capsRatio: capsRatio.toFixed(2),
        };
    }

    analyzeEmojiArousal(text) {
        const highEnergy = SentimentData.AROUSAL_MARKERS.high.emojis;
        const lowEnergy = SentimentData.AROUSAL_MARKERS.low.emojis;

        const highCount = highEnergy.filter((e) => text.includes(e)).length;
        const lowCount = lowEnergy.filter((e) => text.includes(e)).length;

        return {
            score: (highCount - lowCount) * 0.1,
            count: highCount + lowCount,
        };
    }
}

// ============================================================================
// DOMINANCE ANALYZER (Submissive/Assertive)
// ============================================================================
/**
 * DominanceAnalyzer - Analyzes assertiveness/submissiveness in text
 */
export class DominanceAnalyzer {
    analyze(text) {
        let score = 0.5; // Default: neutral
        const markers = {
            assertiveWords: 0,
            submissiveWords: 0,
            questions: 0,
            statements: 0,
            patternsMatched: [],
        };

        const lowerText = text.toLowerCase();

        // Assertive words
        for (const word of SentimentData.DOMINANCE_MARKERS.assertive.words) {
            if (lowerText.includes(word)) {
                markers.assertiveWords++;
                score += 0.08;
            }
        }

        // Submissive words
        for (const word of SentimentData.DOMINANCE_MARKERS.submissive.words) {
            if (lowerText.includes(word)) {
                markers.submissiveWords++;
                score -= 0.08;
            }
        }

        // Pattern-based analysis
        for (const pattern of SentimentData.DOMINANCE_MARKERS.assertive.patterns) {
            if (pattern.test(text)) {
                markers.patternsMatched.push('assertive');
                score += 0.1;
            }
        }

        for (const pattern of SentimentData.DOMINANCE_MARKERS.submissive.patterns) {
            if (pattern.test(text)) {
                markers.patternsMatched.push('submissive');
                score -= 0.1;
            }
        }

        // Question marks (lower dominance)
        markers.questions = (text.match(/\?/g) || []).length;
        score -= markers.questions * 0.05;

        // Periods/statements (higher dominance)
        markers.statements = (text.match(/\./g) || []).length;
        score += markers.statements * 0.02;

        return {
            dominance: Math.max(0, Math.min(1, score)),
            ...markers,
        };
    }
}

// ============================================================================
// SARCASM ANALYZER (Literal/Sarcastic)
// ============================================================================
/**
 * SarcasmAnalyzer - Detects sarcasm in text
 */
export class SarcasmAnalyzer {
    analyze(text) {
        let score = 0;
        const markers = {
            explicitMarkers: [],
            emojis: [],
            contradictions: [],
            dryHumor: 0,
            confidence: 'low',
        };

        const lowerText = text.toLowerCase();

        // Explicit sarcasm markers
        for (const marker of SentimentData.SARCASM_MARKERS.explicit.markers) {
            if (lowerText.includes(marker)) {
                markers.explicitMarkers.push(marker);
                score += 0.2;
            }
        }

        // Sarcasm emojis
        for (const emoji of SentimentData.SARCASM_MARKERS.explicit.emojis) {
            if (text.includes(emoji)) {
                markers.emojis.push(emoji);
                score += 0.25;
            }
        }

        // Contradiction detection
        for (const pattern of SentimentData.SARCASM_MARKERS.contradiction.patterns) {
            if (pattern.test(text)) {
                markers.contradictions.push(pattern.source);
                score += 0.3;
            }
        }

        // Dry humor
        for (const marker of SentimentData.SARCASM_MARKERS.dry_humor.markers) {
            if (lowerText.includes(marker)) {
                markers.dryHumor++;
                score += 0.1;
            }
        }

        // Contradictory phrasing
        if (this.hasContradictoryPhrasing(text)) {
            score += 0.2;
        }

        // Set confidence level
        if (score > 0.7) markers.confidence = 'very_high';
        else if (score > 0.5) markers.confidence = 'high';
        else if (score > 0.3) markers.confidence = 'medium';
        else if (score > 0.1) markers.confidence = 'low';
        else markers.confidence = 'very_low';

        return {
            sarcasm: Math.max(0, Math.min(1, score)),
            ...markers,
        };
    }

    hasContradictoryPhrasing(text) {
        const positiveSuperlatives = [
            'wonderful',
            'fantastic',
            'amazing',
            'brilliant',
            'perfect',
            'excellent',
            'outstanding',
            'incredible',
        ];
        const negativeContext = [
            'disaster',
            'fail',
            'bad',
            'worse',
            'worst',
            'terrible',
            'awful',
            'hate',
            'sucks',
            'stupid',
        ];

        for (const pos of positiveSuperlatives) {
            if (text.toLowerCase().includes(pos)) {
                for (const neg of negativeContext) {
                    if (text.toLowerCase().includes(neg)) {
                        return true;
                    }
                }
            }
        }
        return false;
    }
}

// ============================================================================
// URGENCY ANALYZER (Relaxed/Time-Sensitive)
// ============================================================================
/**
 * UrgencyAnalyzer - Analyzes time-sensitivity in text
 */
export class UrgencyAnalyzer {
    analyze(text) {
        let score = 0.3; // Default: relaxed
        const markers = {
            urgentMarkers: [],
            timeSensitiveMarkers: [],
            scheduledMarkers: [],
            relaxedMarkers: [],
            hasTime: false,
        };

        const lowerText = text.toLowerCase();

        // Urgent markers
        for (const marker of SentimentData.URGENCY_MARKERS.urgent) {
            if (lowerText.includes(marker)) {
                markers.urgentMarkers.push(marker);
                score += 0.12;
            }
        }

        // Time-sensitive topics
        for (const marker of SentimentData.URGENCY_MARKERS.timeSensitive) {
            if (lowerText.includes(marker)) {
                markers.timeSensitiveMarkers.push(marker);
                score += 0.1;
            }
        }

        // Scheduled topics
        for (const marker of SentimentData.URGENCY_MARKERS.scheduled) {
            if (lowerText.includes(marker)) {
                markers.scheduledMarkers.push(marker);
                score += 0.08;
            }
        }

        // Relaxed markers
        for (const marker of SentimentData.URGENCY_MARKERS.relaxed) {
            if (lowerText.includes(marker)) {
                markers.relaxedMarkers.push(marker);
                score -= 0.1;
            }
        }

        // Time patterns (hours, dates)
        if (/\d{1,2}(?:am|pm|hours|mins|seconds)/.test(text)) {
            markers.hasTime = true;
            score += 0.15;
        }

        return {
            urgency: Math.max(0, Math.min(1, score)),
            ...markers,
        };
    }
}

// ============================================================================
// TOXICITY ANALYZER (Friendly/Hostile)
// ============================================================================
/**
 * ToxicityAnalyzer - Detects hostile/toxic language in text
 */
export class ToxicityAnalyzer {
    constructor() {
        this.profanities = [
            'fuck',
            'shit',
            'damn',
            'ass',
            'crap',
            'bitch',
            'asshole',
            'bastard',
            'goddamn',
            'hell',
            'piss',
        ];
        // Pre-compile regexes once for performance (was creating 11 regex objects per analyze call)
        this._profanityRegexes = this.profanities.map((p) => new RegExp(`\\b${p}\\b`, 'i'));
    }

    analyze(text) {
        let score = 0;
        const markers = {
            slurs: [],
            hostileWords: [],
            personalAttacks: [],
            profanities: [],
            allCapsAgg: false,
            dehumanization: [],
        };

        const lowerText = text.toLowerCase();

        // Slurs and insults
        for (const slur of SentimentData.TOXICITY_MARKERS.slurs_insults) {
            if (lowerText.includes(slur)) {
                markers.slurs.push(slur);
                score += 0.15;
            }
        }

        // Hostile words
        for (const word of SentimentData.TOXICITY_MARKERS.hostility) {
            if (lowerText.includes(word)) {
                markers.hostileWords.push(word);
                score += 0.12;
            }
        }

        // Personal attacks
        for (const pattern of SentimentData.TOXICITY_MARKERS.personalAttacks) {
            if (pattern.test(text)) {
                markers.personalAttacks.push(pattern.source);
                score += 0.25;
            }
        }

        // Aggression markers
        for (const marker of SentimentData.TOXICITY_MARKERS.aggression.markers) {
            if (lowerText.includes(marker)) {
                score += SentimentData.TOXICITY_MARKERS.aggression.intensity * 0.1;
            }
        }

        // Dehumanization
        for (const marker of SentimentData.TOXICITY_MARKERS.dehumanization) {
            if (lowerText.includes(marker)) {
                markers.dehumanization.push(marker);
                score += 0.2;
            }
        }

        // ALL CAPS + toxic markers = extra toxic
        if (text.match(/[A-Z]{10,}/) && score > 0.2) {
            markers.allCapsAgg = true;
            score += 0.1;
        }

        // Profanity detection - use pre-compiled regexes
        for (let i = 0; i < this.profanities.length; i++) {
            if (this._profanityRegexes[i].test(text)) {
                markers.profanities.push(this.profanities[i]);
                score += 0.1;
            }
        }

        return {
            toxicity: Math.max(0, Math.min(1, score)),
            ...markers,
            severityLevel: score > 0.7 ? 'severe' : score > 0.4 ? 'moderate' : 'low',
        };
    }
}

export default {
    ValenceAnalyzer,
    ArousalAnalyzer,
    DominanceAnalyzer,
    SarcasmAnalyzer,
    UrgencyAnalyzer,
    ToxicityAnalyzer,
};
