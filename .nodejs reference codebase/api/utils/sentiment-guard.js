/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Sentiment Guard Module
 * Analyzes tweet content sentiment to avoid inappropriate bot interactions
 * Skips likes/retweets on negative/tragic content
 *
 * @module utils/sentiment-guard
 */

const NEGATIVE_KEYWORDS = {
    death: [
        'died',
        'passed away',
        'rip',
        'rest in peace',
        'gone too soon',
        'lost',
        'lost his',
        'lost her',
        'lost their',
        'death',
        'dead',
        'deceased',
        'obituary',
        'funeral',
        'mourn',
        'mourning',
    ],
    tragedy: [
        'tragedy',
        'tragic',
        'accident',
        'crash',
        'shooting',
        'attack',
        'murder',
        'homicide',
        'suicide',
        'overdose',
        'mass shooting',
        'terrorist',
        'violence',
        'victim',
    ],
    grief: [
        'grief',
        'sad',
        'heartbroken',
        'devastated',
        'tragic',
        'unfortunately',
        'sadly',
        'trouble',
        'worst',
        'horrible',
        'terrible',
        'awful',
        'pain',
        'suffering',
    ],
    scam: [
        'scam',
        'hacked',
        'stolen',
        'fraud',
        'fake',
        'phishing',
        'malware',
        'virus',
        'security breach',
        'compromised',
    ],
    controversy: [
        'scandal',
        'controversy',
        'accused',
        'allegations',
        'lawsuit',
        'sued',
        'investigation',
        'subpoena',
        'raid',
    ],
};

const NEGATIVE_PATTERNS = [
    /rip/i,
    /rest in peace/i,
    /passed away/i,
    /gone too soon/i,
    /so sad/i,
    /heartbreaking/i,
    /tragedy/i,
    /tragic/i,
    /unfortunately/i,
    /sadly/i,
    /lost (?:his|her|their|the)/i,
    /death of/i,
    /died (?:at|after|from)/i,
];

const SENTIMENT_THRESHOLDS = {
    skipLike: 0.15,
    skipRetweet: 0.15,
    skipReply: 0.2,
    skipQuote: 0.2,
    allowExpand: true,
};

function analyzeSentiment(text) {
    if (!text || typeof text !== 'string') {
        return { score: 0, isNegative: false, categories: [] };
    }

    const lowerText = text.toLowerCase();
    const categories = [];
    let negativeMatches = 0;

    for (const [category, keywords] of Object.entries(NEGATIVE_KEYWORDS)) {
        let categoryMatches = 0;
        for (const keyword of keywords) {
            if (lowerText.includes(keyword)) {
                categoryMatches++;
                negativeMatches++;
            }
        }
        if (categoryMatches > 0) {
            categories.push({ name: category, matches: categoryMatches });
        }
    }

    for (const pattern of NEGATIVE_PATTERNS) {
        if (pattern.test(text)) {
            negativeMatches++;
            if (!categories.some((c) => c.name === 'pattern')) {
                categories.push({ name: 'pattern', matches: 1 });
            }
        }
    }

    const wordCount = lowerText.split(/\s+/).length;
    const density = wordCount > 0 ? negativeMatches / wordCount : 0;
    const score = Math.min(1, density * 10);
    const isNegative = score >= SENTIMENT_THRESHOLDS.skipLike;

    return {
        score,
        isNegative,
        categories,
        shouldSkipLikes: score >= SENTIMENT_THRESHOLDS.skipLike,
        shouldSkipRetweets: score >= SENTIMENT_THRESHOLDS.skipRetweet,
        shouldSkipReplies: score >= SENTIMENT_THRESHOLDS.skipReply,
        shouldSkipQuotes: score >= SENTIMENT_THRESHOLDS.skipQuote,
        allowExpand: true,
    };
}

function shouldSkipAction(text, action) {
    const analysis = analyzeSentiment(text);

    switch (action.toLowerCase()) {
        case 'like':
            return analysis.shouldSkipLikes;
        case 'retweet':
            return analysis.shouldSkipRetweets;
        case 'reply':
            return analysis.shouldSkipReplies;
        case 'quote':
            return analysis.shouldSkipQuotes;
        case 'dive':
        case 'expand':
            return analysis.allowExpand;
        default:
            return false;
    }
}

function getSkipReason(text, action) {
    const analysis = analyzeSentiment(text);

    if (action.toLowerCase() === 'like' && analysis.shouldSkipLikes) {
        return {
            skipped: true,
            reason: 'Negative sentiment detected',
            categories: analysis.categories,
            score: analysis.score,
        };
    }

    return { skipped: false, reason: null };
}

function getSafeActions(text) {
    const analysis = analyzeSentiment(text);

    return {
        canLike: !analysis.shouldSkipLikes,
        canRetweet: !analysis.shouldSkipRetweets,
        canReply: !analysis.shouldSkipReplies,
        canQuote: !analysis.shouldSkipQuotes,
        canExpand: analysis.allowExpand,
        sentimentScore: analysis.score,
        isNegative: analysis.isNegative,
    };
}

function formatSentimentReport(text) {
    const analysis = analyzeSentiment(text);

    if (analysis.isNegative) {
        const blockedActions = ['like', 'retweet', 'reply', 'quote']
            .filter((a, i) => {
                const methods = [
                    'shouldSkipLikes',
                    'shouldSkipRetweets',
                    'shouldSkipReplies',
                    'shouldSkipQuotes',
                ];
                return analysis[methods[i]];
            })
            .join(', ');
        return (
            `[SentimentGuard] 🚫 NEGATIVE content detected (score: ${analysis.score.toFixed(2)})\n` +
            `[SentimentGuard] 🚫 Categories: ${analysis.categories.map((c) => c.name).join(', ') || 'unknown'} | Actions blocked: ${blockedActions || 'none'}`
        );
    }

    return `[SentimentGuard] ✅ Neutral/Positive content (score: ${analysis.score.toFixed(2)})`;
}

function shouldProcessContent(text, context = {}) {
    const analysis = analyzeSentiment(text);

    const { allowNegativeExpand = true } = context;

    if (analysis.isNegative) {
        if (allowNegativeExpand) {
            return {
                allowed: true,
                restrictions: {
                    expand: true,
                    like: false,
                    retweet: false,
                    reply: false,
                    quote: false,
                },
            };
        }
        return { allowed: false, reason: 'Negative content' };
    }

    return { allowed: true, restrictions: null };
}

export const sentimentGuard = {
    analyzeSentiment,
    shouldSkipAction,
    getSkipReason,
    getSafeActions,
    formatSentimentReport,
    shouldProcessContent,
    defaults: {
        thresholds: SENTIMENT_THRESHOLDS,
        keywords: NEGATIVE_KEYWORDS,
        patterns: NEGATIVE_PATTERNS,
    },
};

export default sentimentGuard;
