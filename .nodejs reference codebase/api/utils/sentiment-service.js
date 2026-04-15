/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unified Sentiment Service
 * Bridges sentiment-guard (basic safety) with sentiment-analyzers (advanced dimensions)
 * Provides a single interface for all sentiment analysis needs
 * @module utils/sentiment-service
 */

import { sentimentGuard } from './sentiment-guard.js';
import {
    ValenceAnalyzer,
    ArousalAnalyzer,
    DominanceAnalyzer,
    SarcasmAnalyzer,
    UrgencyAnalyzer,
    ToxicityAnalyzer,
} from './sentiment-analyzers.js';

/**
 * Unified Sentiment Service
 * Combines basic safety checks with advanced multi-dimensional analysis
 */
export class SentimentService {
    constructor() {
        // Initialize all advanced analyzers
        this.analyzers = {
            valence: new ValenceAnalyzer(),
            arousal: new ArousalAnalyzer(),
            dominance: new DominanceAnalyzer(),
            sarcasm: new SarcasmAnalyzer(),
            urgency: new UrgencyAnalyzer(),
            toxicity: new ToxicityAnalyzer(),
        };

        // Cache for repeated analysis (optional optimization)
        this.cache = new Map();
        this.cacheMaxSize = 100;
    }

    /**
     * Perform comprehensive sentiment analysis
     * @param {string} text - Text to analyze
     * @param {object} options - Analysis options
     * @returns {object} Complete sentiment analysis
     */
    analyze(text, options = {}) {
        if (!text || typeof text !== 'string') {
            return this.getNeutralAnalysis();
        }

        const { useCache = true, includeDebug = false } = options;

        // Check cache for repeated texts
        if (useCache) {
            const cached = this.getFromCache(text);
            if (cached) return cached;
        }

        // Step 1: Basic safety check (from sentiment-guard)
        const guardResult = sentimentGuard.analyzeSentiment(text);

        // Step 2: Advanced multi-dimensional analysis
        const advanced = {
            valence: this.analyzers.valence.analyze(text),
            arousal: this.analyzers.arousal.analyze(text),
            dominance: this.analyzers.dominance.analyze(text),
            sarcasm: this.analyzers.sarcasm.analyze(text),
            urgency: this.analyzers.urgency.analyze(text),
            toxicity: this.analyzers.toxicity.analyze(text),
        };

        // Step 3: Derive composite metrics
        const composite = this.deriveCompositeMetrics(advanced);

        // Step 4: Build unified result
        const result = {
            // Legacy compatibility (for existing code using sentiment-guard)
            isNegative: guardResult.isNegative,
            score: guardResult.score,
            categories: guardResult.categories,
            shouldSkipLikes: guardResult.shouldSkipLikes,
            shouldSkipRetweets: guardResult.shouldSkipRetweets,
            shouldSkipReplies: guardResult.shouldSkipReplies,
            shouldSkipQuotes: guardResult.shouldSkipQuotes,
            allowExpand: guardResult.allowExpand,

            // Advanced dimensions
            dimensions: advanced,

            // Composite/derived metrics
            composite: composite,

            // Engagement recommendations
            engagement: this.getEngagementRecommendations(guardResult, advanced, composite),

            // Debug info (if requested)
            ...(includeDebug && { debug: { guardResult, advanced } }),
        };

        // Cache result
        if (useCache) {
            this.addToCache(text, result);
        }

        return result;
    }

    /**
     * Quick analysis (legacy compatibility)
     * Returns basic sentiment info like sentiment-guard
     */
    analyzeBasic(text) {
        const full = this.analyze(text, { useCache: true });
        return {
            isNegative: full.isNegative,
            score: full.score,
            categories: full.categories,
            shouldSkipLikes: full.shouldSkipLikes,
            shouldSkipRetweets: full.shouldSkipRetweets,
            shouldSkipReplies: full.shouldSkipReplies,
            shouldSkipQuotes: full.shouldSkipQuotes,
            allowExpand: full.allowExpand,
        };
    }

    /**
     * Analyze sentiment for reply selection
     * Returns metrics useful for picking which replies to show LLM
     */
    analyzeForReplySelection(replies) {
        if (!Array.isArray(replies) || replies.length === 0) {
            return { strategy: 'none', replies: [] };
        }

        // Analyze all replies
        const analyzed = replies.map((reply) => ({
            ...reply,
            sentiment: this.analyze(reply.text || reply.content || '', { useCache: false }),
        }));

        // Calculate conversation sentiment distribution
        const distribution = {
            positive: analyzed.filter((r) => r.sentiment.dimensions.valence.valence > 0.2).length,
            negative: analyzed.filter((r) => r.sentiment.dimensions.valence.valence < -0.2).length,
            neutral: analyzed.filter((r) => Math.abs(r.sentiment.dimensions.valence.valence) <= 0.2)
                .length,
            sarcastic: analyzed.filter((r) => r.sentiment.dimensions.sarcasm.sarcasm > 0.4).length,
            toxic: analyzed.filter((r) => r.sentiment.dimensions.toxicity.toxicity > 0.5).length,
        };

        // Determine selection strategy based on conversation sentiment
        let strategy = 'longest'; // default
        const total = analyzed.length;

        if (distribution.toxic > total * 0.3) {
            strategy = 'neutral-only'; // Avoid toxic threads
        } else if (distribution.sarcastic > total * 0.4) {
            strategy = 'match-sarcasm'; // Match sarcastic tone
        } else if (distribution.positive > total * 0.6) {
            strategy = 'positive-biased'; // Positive thread
        } else if (distribution.negative > total * 0.6) {
            strategy = 'balanced'; // Controversial - show both sides
        }

        return {
            strategy,
            distribution,
            analyzed,
            recommendations: this.getReplyRecommendations(strategy, analyzed),
        };
    }

    /**
     * Derive composite metrics from individual dimensions
     */
    deriveCompositeMetrics(dimensions) {
        const { valence, arousal, dominance, sarcasm, toxicity } = dimensions;

        // Overall emotional intensity
        const intensity = (Math.abs(valence.valence) + arousal.arousal) / 2;

        // Engagement style
        let engagementStyle = 'neutral';
        if (sarcasm.sarcasm > 0.5) {
            engagementStyle = 'sarcastic';
        } else if (toxicity.toxicity > 0.5) {
            engagementStyle = 'hostile';
        } else if (valence.valence > 0.3 && arousal.arousal > 0.6) {
            engagementStyle = 'enthusiastic';
        } else if (valence.valence < -0.3 && arousal.arousal > 0.6) {
            engagementStyle = 'angry';
        } else if (dominance.dominance > 0.7) {
            engagementStyle = 'assertive';
        } else if (dominance.dominance < 0.3) {
            engagementStyle = 'questioning';
        }

        // Conversation type
        let conversationType = 'general';
        if (toxicity.toxicity > 0.4) {
            conversationType = 'controversial';
        } else if (sarcasm.sarcasm > 0.4) {
            conversationType = 'humorous';
        } else if (valence.valence > 0.5) {
            conversationType = 'positive';
        } else if (valence.valence < -0.3) {
            conversationType = 'negative';
        }

        // Risk level for bot engagement
        let riskLevel = 'low';
        if (toxicity.toxicity > 0.6 || this.hasNegativePattern(dimensions)) {
            riskLevel = 'high';
        } else if (toxicity.toxicity > 0.4 || sarcasm.sarcasm > 0.6) {
            riskLevel = 'medium';
        }

        return {
            intensity: Math.max(0, Math.min(1, intensity)),
            engagementStyle,
            conversationType,
            riskLevel,
            confidence: this.calculateConfidence(dimensions),
        };
    }

    /**
     * Get engagement recommendations
     */
    getEngagementRecommendations(guard, advanced, composite) {
        const recommendations = {
            canLike: !guard.shouldSkipLikes,
            canRetweet: !guard.shouldSkipRetweets,
            canReply: !guard.shouldSkipReplies,
            canQuote: !guard.shouldSkipQuotes,
            shouldEngage: true,
            recommendedTone: 'neutral',
            warnings: [],
        };

        // Add warnings based on advanced analysis
        if (advanced.sarcasm.sarcasm > 0.5) {
            recommendations.warnings.push('sarcasm-detected');
            recommendations.recommendedTone = 'playful';
        }

        if (advanced.toxicity.toxicity > 0.4) {
            recommendations.warnings.push('toxicity-detected');
            recommendations.shouldEngage = false;
        }

        if (composite.riskLevel === 'high') {
            recommendations.warnings.push('high-risk-content');
            recommendations.shouldEngage = false;
        }

        if (advanced.dominance.dominance > 0.7) {
            recommendations.recommendedTone = 'assertive';
        } else if (advanced.dominance.dominance < 0.3) {
            recommendations.recommendedTone = 'collaborative';
        }

        return recommendations;
    }

    /**
     * Get reply selection recommendations
     */
    getReplyRecommendations(strategy, analyzed) {
        switch (strategy) {
            case 'neutral-only':
                return {
                    filter: (r) => Math.abs(r.sentiment.dimensions.valence.valence) <= 0.2,
                    sort: (a, b) => b.text.length - a.text.length,
                    max: 30,
                };

            case 'match-sarcasm':
                return {
                    filter: (r) => r.sentiment.dimensions.sarcasm.sarcasm > 0.3,
                    sort: (a, b) =>
                        b.sentiment.dimensions.sarcasm.sarcasm -
                        a.sentiment.dimensions.sarcasm.sarcasm,
                    max: 30,
                };

            case 'balanced': {
                // Pick 15 most positive + 15 most negative
                const positive = analyzed
                    .filter((r) => r.sentiment.dimensions.valence.valence > 0)
                    .sort(
                        (a, b) =>
                            b.sentiment.dimensions.valence.valence -
                            a.sentiment.dimensions.valence.valence
                    )
                    .slice(0, 15);
                const negative = analyzed
                    .filter((r) => r.sentiment.dimensions.valence.valence < 0)
                    .sort(
                        (a, b) =>
                            a.sentiment.dimensions.valence.valence -
                            b.sentiment.dimensions.valence.valence
                    )
                    .slice(0, 15);
                return {
                    manualSelection: [...positive, ...negative],
                    max: 30,
                };
            }

            case 'positive-biased':
                return {
                    filter: (r) => r.sentiment.dimensions.valence.valence >= -0.1,
                    sort: (a, b) => b.text.length - a.text.length,
                    max: 30,
                };

            default: // longest
                return {
                    filter: () => true,
                    sort: (a, b) => b.text.length - a.text.length,
                    max: 30,
                };
        }
    }

    /**
     * Check for negative patterns (legacy compatibility)
     */
    hasNegativePattern(dimensions) {
        return dimensions.valence.valence < -0.5 && dimensions.arousal.arousal > 0.6;
    }

    /**
     * Calculate overall confidence score
     */
    calculateConfidence(dimensions) {
        const confidences = [dimensions.valence.confidence, dimensions.sarcasm.confidence];

        const score =
            confidences.reduce((sum, c) => {
                return sum + (c === 'high' ? 1 : c === 'medium' ? 0.5 : 0.25);
            }, 0) / confidences.length;

        if (score > 0.7) return 'high';
        if (score > 0.4) return 'medium';
        return 'low';
    }

    /**
     * Get neutral analysis (for empty/invalid text)
     */
    getNeutralAnalysis() {
        return {
            isNegative: false,
            score: 0,
            dimensions: {
                valence: { valence: 0, confidence: 'low' },
                arousal: { arousal: 0.5 },
                dominance: { dominance: 0.5 },
                sarcasm: { sarcasm: 0, confidence: 'very_low' },
                urgency: { urgency: 0.3 },
                toxicity: { toxicity: 0 },
            },
            composite: {
                intensity: 0.25,
                engagementStyle: 'neutral',
                conversationType: 'general',
                riskLevel: 'low',
            },
            engagement: {
                canLike: true,
                canRetweet: true,
                canReply: true,
                canQuote: true,
                shouldEngage: true,
                recommendedTone: 'neutral',
                warnings: [],
            },
        };
    }

    /**
     * Cache management
     */
    getFromCache(text) {
        const key = text.toLowerCase().trim().substring(0, 200);
        return this.cache.get(key);
    }

    addToCache(text, result) {
        const key = text.toLowerCase().trim().substring(0, 200);

        // Simple LRU: clear if too big
        if (this.cache.size >= this.cacheMaxSize) {
            const firstKey = this.cache.keys().next().value;
            this.cache.delete(firstKey);
        }

        this.cache.set(key, result);
    }

    clearCache() {
        this.cache.clear();
    }
}

// Singleton instance
export const sentimentService = new SentimentService();

// Legacy compatibility - export same interface as sentiment-guard
export const analyzeSentiment = (text) => sentimentService.analyzeBasic(text);
export const shouldSkipAction = (text, action) => sentimentGuard.shouldSkipAction(text, action);
export const getSafeActions = (text) => sentimentGuard.getSafeActions(text);
export const formatSentimentReport = (text) => sentimentGuard.formatSentimentReport(text);

export default sentimentService;
