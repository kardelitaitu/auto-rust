/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Enhanced AI Context Engine
 * Provides richer context extraction including sentiment, tone, and engagement metrics
 * @module utils/ai-context-engine
 */

import { createLogger } from '../core/logger.js';
import { mathUtils } from '../utils/math.js';
import { api } from '../index.js';

/**
 * AIContextEngine - Extracts enhanced context from tweets
 */
export class AIContextEngine {
    /**
     * Creates a new AIContextEngine instance
     * @param {object} options - Configuration options
     */
    constructor(options = {}) {
        this.logger = createLogger('ai-context-engine.js');

        // Sentiment indicators (simple keyword-based)
        this.sentimentIndicators = {
            positive: [
                'love',
                'great',
                'amazing',
                'awesome',
                'beautiful',
                'wonderful',
                'fantastic',
                'excellent',
                'happy',
                'excited',
                'glad',
                'blessed',
                'perfect',
                'best',
                'incredible',
                ' stunning',
                'brilliant',
                'favorite',
                'appreciate',
                'thanks',
                'thank',
                'grateful',
                'yay',
                'woohoo',
                '🎉',
                '🎊',
                '✨',
                '🌟',
                '💖',
                '💯',
                '🙌',
                '👏',
            ],
            negative: [
                'hate',
                'terrible',
                'awful',
                'horrible',
                'worst',
                'bad',
                'sad',
                'angry',
                'frustrated',
                'annoyed',
                'disappointed',
                'upset',
                'sucks',
                'pathetic',
                'disgusting',
                'ridiculous',
                'furious',
                'crying',
                'tears',
                'death',
                'died',
                'lost',
                'gone',
                'miss',
                '💔',
                '😢',
                '😭',
                '😞',
                '😠',
                '💢',
            ],
            humorous: [
                'lol',
                'lmao',
                'rofl',
                'haha',
                'ahaha',
                'funny',
                'lolol',
                '😂',
                '🤣',
                '💀',
                '🤡',
                '😂💀',
                ' 😭',
            ],
            informational: [
                'update',
                'breaking',
                'news',
                'report',
                'according to',
                'source',
                'announced',
                'revealed',
                'confirmed',
                'official',
                'statement',
                "here's why",
                "here's what",
                'did you know',
                'tip:',
                'guide',
            ],
            emotional: [
                'feel',
                'feeling',
                'heart',
                'soul',
                'struggle',
                'journey',
                'overcome',
                'proud',
                'accomplish',
                'dream',
                'hope',
                'wish',
                'pray',
                'believe',
                'faith',
                'moment',
                'memory',
                'remind',
            ],
        };

        this.config = {
            maxReplies: options.maxReplies ?? 50,
            sentimentThreshold: options.sentimentThreshold ?? 0.3,
            includeMetrics: options.includeMetrics ?? true,
        };
    }

    /**
     * Extract comprehensive context from tweet page
     * @param {object} page - Playwright page
     * @param {string} tweetUrl - Current tweet URL
     * @param {string} tweetText - Main tweet text
     * @param {string} authorUsername - Tweet author
     * @returns {Promise<object>} Enhanced context
     */
    async extractEnhancedContext(page, tweetUrl, tweetText, authorUsername) {
        const context = {
            url: tweetUrl,
            tweetText: tweetText,
            author: authorUsername,
            replies: [],
            metrics: null,
            sentiment: null,
            tone: null,
            conversationType: null,
            replySentiment: null,
            engagementLevel: 'unknown',
            extractedAt: new Date().toISOString(),
        };

        try {
            // Extract engagement metrics
            if (this.config.includeMetrics) {
                context.metrics = await this.extractMetrics(page);
                context.engagementLevel = this.calculateEngagementLevel(context.metrics);
            }

            // Extract replies
            const extractedReplies = await this.extractRepliesSmart(page);
            context.replies = extractedReplies.slice(0, this.config.maxReplies);

            // Analyze main tweet
            context.sentiment = this.analyzeSentiment(tweetText);
            context.tone = this.detectTone(tweetText);
            context.conversationType = this.classifyConversation(tweetText, context.replies);

            // Analyze replies
            context.replySentiment = this.analyzeReplySentiment(context.replies);

            // Check for images and capture screenshot if present
            const hasImage = await this.checkForTweetImage(page);
            if (hasImage) {
                context.screenshot = await this.captureTweetScreenshot(page);
                this.logger.info(`[Context] Captured screenshot of tweet with image`);
            }

            this.logger.info(
                `[Context] Enhanced context: ${context.sentiment?.overall}, tone: ${context.tone?.primary}, ${context.replies.length} replies, ${context.engagementLevel} engagement`
            );
        } catch (error) {
            this.logger.warn(`[Context] Enhanced extraction failed: ${error.message}`);
        }

        return context;
    }

    /**
     * Extract engagement metrics from tweet
     */
    async extractMetrics(page) {
        const metrics = {
            likes: 0,
            retweets: 0,
            replies: 0,
            views: 0,
            bookmarks: 0,
        };

        try {
            // Look for metrics in the DOM
            const metricsText = await page.evaluate(() => {
                const elements = document.querySelectorAll(
                    '[data-testid], [aria-label*="like"], [aria-label*="retweet"], [aria-label*="reply"], [aria-label*="view"]'
                );
                return Array.from(elements).map((el) => ({
                    aria: el.getAttribute('aria-label') || '',
                    text: el instanceof HTMLElement ? el.innerText : '',
                }));
            });

            // Parse metrics from aria-labels and text
            for (const item of metricsText) {
                const text = item.text || item.aria;

                // Extract numbers with K/M suffixes
                const numberMatch = text.match(/[\d,.]+[KM]?/);
                if (numberMatch) {
                    const num = this.parseNumber(numberMatch[0]);

                    if (text.toLowerCase().includes('like') && metrics.likes === 0) {
                        metrics.likes = num;
                    } else if (text.toLowerCase().includes('retweet') && metrics.retweets === 0) {
                        metrics.retweets = num;
                    } else if (text.toLowerCase().includes('reply') && metrics.replies === 0) {
                        metrics.replies = num;
                    } else if (
                        (text.toLowerCase().includes('view') ||
                            text.toLowerCase().includes('impression')) &&
                        metrics.views === 0
                    ) {
                        metrics.views = num;
                    } else if (text.toLowerCase().includes('bookmark') && metrics.bookmarks === 0) {
                        metrics.bookmarks = num;
                    }
                }
            }
        } catch (error) {
            this.logger.debug(`[Context] Metrics extraction: ${error.message}`);
        }

        return metrics;
    }

    /**
     * Parse number with K/M suffixes
     */
    parseNumber(str) {
        if (!str) return 0;
        str = str.toUpperCase().replace(/,/g, '');

        if (str.includes('K')) {
            return Math.floor(parseFloat(str) * 1000);
        } else if (str.includes('M')) {
            return Math.floor(parseFloat(str) * 1000000);
        }
        return parseInt(str) || 0;
    }

    /**
     * Calculate engagement level from metrics
     */
    calculateEngagementLevel(metrics) {
        if (!metrics) return 'unknown';

        const total = (metrics.likes || 0) + (metrics.retweets || 0) + (metrics.replies || 0);
        const views = metrics.views || 0;

        if (views > 100000) return 'viral';
        if (total > 1000) return 'high';
        if (total > 100) return 'medium';
        if (total > 10) return 'low';
        return 'minimal';
    }

    /**
     * Check if the main tweet contains an image
     * @param {object} page - Playwright page
     * @returns {Promise<boolean>}
     */
    async checkForTweetImage(page) {
        try {
            return await page.evaluate(() => {
                // Check for tweet photos or video thumbnails in the first article (main tweet)
                const mainTweet = document.querySelector('article');
                if (!mainTweet) return false;

                const hasPhoto = mainTweet.querySelector('[data-testid="tweetPhoto"]') !== null;
                const hasVideo = mainTweet.querySelector('[data-testid="videoPlayer"]') !== null;
                const hasCard = mainTweet.querySelector('[data-testid="card.wrapper"]') !== null;

                return hasPhoto || hasVideo || hasCard;
            });
        } catch (error) {
            this.logger.debug(`[Context] Image check failed: ${error.message}`);
            return false;
        }
    }

    /**
     * Capture screenshot of the main tweet
     * @param {object} page - Playwright page
     * @returns {Promise<Buffer>}
     */
    async captureTweetScreenshot(page) {
        try {
            // Scroll to top to ensure tweet is visible (natural 2s scroll)
            await api.scroll.toTop(2000);
            await api.wait(500); // Wait for scroll/render

            // Try to capture just the tweet element first
            const tweetElement = await page.$('article');
            if (tweetElement) {
                return await tweetElement.screenshot({ type: 'jpeg', quality: 80 });
            }

            // Fallback to viewport screenshot
            return await page.screenshot({ type: 'jpeg', quality: 80, fullPage: false });
        } catch (error) {
            this.logger.warn(`[Context] Screenshot capture failed: ${error.message}`);
            return null;
        }
    }

    /**
     * Smart reply extraction with deduplication
     * Now properly scrolls and extracts 15-30 replies for LLM context
     */
    async extractRepliesSmart(page) {
        const replies = [];
        const seenTexts = new Set();

        try {
            this.logger.info(`[Context] Starting comprehensive reply extraction...`);

            // Step 1: Use api.scroll.read to load replies naturally
            this.logger.debug(`[Context] Step 1: Using api.scroll.read to load replies...`);
            try {
                await api.scroll.read(null, { pauses: 10, scrollAmount: 500 });
            } catch (scrollError) {
                this.logger.error(
                    `[Context] Scroll read failed during reply extraction: ${scrollError.message}`
                );
                // We continue anyway, as some replies might already be visible on the screen
            }

            // Step 2: Wait for content to settle
            await api.wait(1000);

            // Step 3: Now scroll UP through the page while extracting
            this.logger.debug(`[Context] Step 2: Extracting while scrolling up...`);

            const viewportHeight = await page.evaluate(() => window.innerHeight);
            const scrollHeight = await page.evaluate(() => document.body.scrollHeight);
            const extractSteps = Math.min(20, Math.ceil(scrollHeight / viewportHeight));

            for (let step = 0; step < extractSteps; step++) {
                // Extract visible tweet texts at current scroll position
                const visibleTexts = await page.evaluate(() => {
                    const found = [];
                    const seen = new Set();

                    // Multiple selectors for current Twitter DOM
                    const selectors = [
                        '[data-testid="tweetText"]',
                        '[class*="tweetText"]',
                        '[class*="replyText"]',
                        'article [dir="auto"]',
                        '[role="article"] span',
                    ];

                    for (const selector of selectors) {
                        const elements = document.querySelectorAll(selector);
                        elements.forEach((el) => {
                            const text = el instanceof HTMLElement ? el.innerText.trim() : '';
                            // Relaxed requirement: allow any meaningful text, not just @mentions
                            // This helps with Korean/Japanese tweets that may not use @mentions
                            if (text && text.length > 3 && text.length < 300) {
                                const key = text.substring(0, 50).toLowerCase();
                                if (!seen.has(key)) {
                                    seen.add(key);
                                    found.push(text);
                                }
                            }
                        });
                    }
                    return found;
                });

                for (const text of visibleTexts) {
                    const key = text.toLowerCase().substring(0, 50);
                    if (seenTexts.has(key)) continue;

                    // Extract author from @mention if present, otherwise use DOM hierarchy
                    const mentionMatch = text.match(/@(\w+)/);
                    let author = mentionMatch ? mentionMatch[1] : 'unknown';

                    // If no @mention, try to extract author from DOM context
                    if (author === 'unknown') {
                        author = await this.extractAuthorFromVisibleText(page, text);
                    }

                    // Clean the reply text - preserve @mentions that are in-content
                    const cleaned = this.cleanReplyText(text);

                    // Accept shorter replies (3+ chars) to capture short responses
                    if (cleaned.length > 3) {
                        seenTexts.add(key);
                        replies.push({
                            author: author,
                            text: cleaned,
                            length: cleaned.length,
                            hasMention: mentionMatch !== null,
                        });
                    }
                }

                // Scroll up for next extraction (slow and human-like using API)
                if (step < extractSteps - 1) {
                    await api.scroll.back();
                    await api.think(mathUtils.randomInRange(600, 1200)); // Slower extraction
                }

                if (replies.length >= 30) {
                    this.logger.debug(`[Context] Target reached: ${replies.length} replies`);
                    break;
                }
            }

            // Step 4: Final natural scroll to top
            await api.scroll.toTop(3000);

            // Strategy 1: Use article elements (most reliable) - but filter out main tweet
            if (replies.length < 15) {
                this.logger.debug(`[Context] Strategy 1: Extracting from article elements...`);
                const articles = await page.$$('article');
                this.logger.debug(`[Context] Found ${articles.length} articles`);

                for (const article of articles.slice(0, 50)) {
                    try {
                        // Skip if this is the main tweet (usually first/largest article)
                        const articleHeight = await article.evaluate(
                            (el) => el.getBoundingClientRect().height
                        );
                        if (articleHeight < 100) continue; // Skip very small articles (likely engagement counts)

                        const data = await this.extractReplyFromArticle(article);
                        if (
                            data &&
                            data.text &&
                            data.text.length > 3 &&
                            data.author !== 'unknown'
                        ) {
                            const key = data.text.toLowerCase().substring(0, 50);
                            if (!seenTexts.has(key)) {
                                seenTexts.add(key);
                                replies.push(data);
                                this.logger.debug(
                                    `[Context] Extracted reply from article: @${data.author}: "${data.text.substring(0, 30)}..."`
                                );
                            }
                        }
                    } catch (e) {
                        this.logger.debug(`[Context] Article extraction error: ${e.message}`);
                    }
                }
                this.logger.debug(`[Context] Strategy 1 extracted ${replies.length} replies`);
            }

            // Strategy 2: Fallback to tweetText elements
            if (replies.length < 15) {
                this.logger.debug(`[Context] Strategy 2: Using tweetText selectors...`);
                const tweetTexts = await page.$$('[data-testid="tweetText"]');

                for (const el of tweetTexts.slice(1, 60)) {
                    try {
                        const text = await el.innerText().catch(() => '');
                        if (text && text.length > 3) {
                            const key = text.toLowerCase().substring(0, 50);
                            if (!seenTexts.has(key)) {
                                seenTexts.add(key);
                                const mentionMatch = text.match(/@(\w+)/);
                                replies.push({
                                    author: mentionMatch ? mentionMatch[1] : 'unknown',
                                    text: this.cleanReplyText(text),
                                    length: text.length,
                                    hasMention: text.includes('@'),
                                });
                            }
                        }
                    } catch (error) {
                        this.logger.debug(
                            `[Context] Tweet text extraction failed: ${error.message}`
                        );
                    }
                }
            }

            // Strategy 3: Deep DOM extraction if still few replies
            if (replies.length < 10) {
                this.logger.debug(`[Context] Strategy 3: Deep DOM extraction...`);

                // For deep extraction, accept any text that looks like a reply
                // Not just @mentions - includes Korean, Japanese, emoji-heavy replies
                const deepTexts = await page.evaluate(() => {
                    const found = [];
                    const seen = new Set();

                    // Look for text in article elements (replies are always in articles)
                    const articles = document.querySelectorAll('article');

                    articles.forEach((article, index) => {
                        // Skip the first article (main tweet)
                        if (index === 0) return;

                        // Get all text content from the article
                        const textContent = [];
                        const paragraphs = article.querySelectorAll(
                            '[data-testid="tweetText"], [dir="auto"]'
                        );

                        paragraphs.forEach((p) => {
                            if (!(p instanceof HTMLElement)) return;
                            const text = p.innerText.trim();
                            if (text && text.length > 3 && text.length < 300) {
                                const key = text.substring(0, 30).toLowerCase();
                                if (!seen.has(key)) {
                                    seen.add(key);
                                    textContent.push(text);
                                }
                            }
                        });

                        if (textContent.length > 0) {
                            // Join multiple paragraphs and take the longest
                            const joined = textContent.join(' ').substring(0, 280);
                            found.push(joined);
                        }
                    });

                    return found;
                });

                for (const text of deepTexts) {
                    const key = text.toLowerCase().substring(0, 50);
                    if (!seenTexts.has(key)) {
                        seenTexts.add(key);
                        // Extract author from @mention or use 'unknown'
                        const mentionMatch = text.match(/@([a-zA-Z0-9_]+)/);
                        const author = mentionMatch ? mentionMatch[1] : 'unknown';

                        // Filter out pure numbers/engagement metrics
                        if (!/^[\d,.\sK]+$/.test(text)) {
                            replies.push({
                                author: author,
                                text: this.cleanReplyText(text),
                                length: text.length,
                                hasMention: mentionMatch !== null,
                            });
                        }
                    }
                }
                this.logger.debug(
                    `[Context] Strategy 3 extracted ${deepTexts.length} potential replies`
                );
            }

            // Filter and sort replies - take longest for richer LLM context
            // Also allow shorter replies (2+ chars) to capture emoji reactions, short confirmations
            const seen = new Set();
            const finalReplies = replies
                .filter((r) => r.text && r.text.length > 2 && r.text.length < 280)
                // Remove duplicates based on text content - O(n) using Set instead of O(n²) findIndex
                .filter((reply) => {
                    const key = reply.text.toLowerCase().substring(0, 50);
                    if (seen.has(key)) return false;
                    seen.add(key);
                    return true;
                })
                .sort((a, b) => b.text.length - a.text.length)
                .slice(0, 30);

            this.logger.info(
                `[Context] Final reply count: ${finalReplies.length} (from ${replies.length} candidates)`
            );

            return finalReplies;
        } catch (error) {
            this.logger.debug(`[Context] Smart reply extraction: ${error.message}`);
            return replies.slice(0, 10);
        }
    }

    /**
     * Extract author from visible text when no @mention is present
     * Useful for Korean/Japanese tweets
     */
    async extractAuthorFromVisibleText(page, text) {
        try {
            // Try to find the author from the DOM context near tweet text
            const authorInfo = await page.evaluate((_searchText) => {
                const elements = document.querySelectorAll(
                    '[data-testid="User-Name"], [class*="UserName"], [class*="author"]'
                );
                for (const el of elements) {
                    const elText = el instanceof HTMLElement ? el.innerText : '';
                    if (elText.includes('@')) {
                        const match = elText.match(/@([a-zA-Z0-9_]+)/);
                        if (match) return match[1];
                    }
                }
                return null;
            }, text);

            return authorInfo || 'unknown';
        } catch (error) {
            this.logger.debug(`[Context] Visible author extraction: ${error.message}`);
            return 'unknown';
        }
    }

    /**
     * Extract reply data from article element
     */
    async extractReplyFromArticle(article) {
        try {
            // Get tweet text
            const textEl = await article.$('[data-testid="tweetText"], [dir="auto"]');
            const text = textEl ? await textEl.innerText().catch(() => '') : '';

            if (!text || text.length < 5) {
                this.logger.debug(`[extractReplyFromArticle] No text found or too short`);
                return null;
            }

            // Get author
            const author = await this.extractAuthorFromArticle(article);
            this.logger.debug(
                `[extractReplyFromArticle] Extracted: @${author}: "${text.substring(0, 30)}..."`
            );

            // Clean text
            const cleaned = this.cleanReplyText(text);

            // Filter out engagement metrics (just numbers)
            if (/^[\d,.\sK]+$/.test(cleaned)) {
                this.logger.debug(
                    `[extractReplyFromArticle] Skipping engagement metrics: "${cleaned}"`
                );
                return null;
            }

            return {
                author: author,
                text: cleaned,
                length: cleaned.length,
                hasMention: cleaned.includes('@'),
            };
        } catch (error) {
            this.logger.debug(`[extractReplyFromArticle] Error: ${error.message}`);
            return null;
        }
    }

    /**
     * Extract author from article
     */
    async extractAuthorFromArticle(article) {
        try {
            // Strategy 1: Find username in header links (most reliable)
            const headerSelectors = [
                'a[href^="/"][role="link"]',
                'a[href*="/status/"]',
                '[class*="UserName"] a',
                '[class*="userName"] a',
                '[data-testid="User-Name"] a',
            ];

            for (const selector of headerSelectors) {
                try {
                    const headerLink = await article.$(selector);
                    if (headerLink) {
                        const href = await headerLink.getAttribute('href');
                        if (href) {
                            // Handle various URL formats: /username, /username/status/123
                            const username = href.replace(/^\/|\/$/g, '').split('/')[0];
                            // Username validation: alphanumeric + underscores, 4-15 chars
                            if (
                                username &&
                                /^[a-zA-Z0-9_]{4,15}$/.test(username) &&
                                !username.match(/^\d+$/)
                            ) {
                                // Exclude pure numbers
                                return username;
                            }
                        }
                    }
                } catch (error) {
                    this.logger.debug(`[Context] Header selector lookup failed: ${error.message}`);
                }
            }

            // Strategy 2: Extract from display name containing @username
            const nameEl = await article.$(
                '[data-testid="User-Name"], [class*="username"], [class*="screenName"]'
            );
            if (nameEl) {
                try {
                    const nameText = await nameEl.innerText();
                    const match = nameText.match(/@([a-zA-Z0-9_]+)/);
                    if (match && match[1].length >= 4) return match[1];
                } catch (error) {
                    this.logger.debug(`[Context] Display name lookup failed: ${error.message}`);
                }
            }

            // Strategy 3: Look for any link with /username pattern
            try {
                const allLinks = await article.$$('a[href^="/"]');
                for (const link of allLinks.slice(0, 5)) {
                    const href = await link.getAttribute('href');
                    if (href && href.startsWith('/') && !href.includes('status')) {
                        const username = href.replace(/^\/|\/$/g, '');
                        if (/^[a-zA-Z0-9_]{4,15}$/.test(username) && !username.match(/^\d+$/)) {
                            return username;
                        }
                    }
                }
            } catch (error) {
                this.logger.debug(`[Context] Username link scan failed: ${error.message}`);
            }

            // Strategy 4: Fallback - try to find any @mention in the article
            try {
                const articleText = await article.innerText();
                const match = articleText.match(/@([a-zA-Z0-9_]{4,15})/);
                if (match) return match[1];
            } catch (error) {
                this.logger.debug(`[Context] Timeline reply extraction failed: ${error.message}`);
            }

            return 'unknown';
        } catch {
            return 'unknown';
        }
    }

    /**
     * Extract replies from timeline
     */
    async extractFromTimeline(page) {
        const replies = [];

        try {
            const tweetTexts = await page.$$('[data-testid="tweetText"]');

            for (const el of tweetTexts.slice(1, 40)) {
                // Skip first (main tweet)
                try {
                    const text = await el.innerText().catch(() => '');
                    if (text && text.length > 3) {
                        const mentionMatch = text.match(/@(\w+)/);
                        replies.push({
                            author: mentionMatch ? mentionMatch[1] : 'unknown',
                            text: this.cleanReplyText(text),
                        });
                    }
                } catch (error) {
                    this.logger.debug(`[Context] Timeline item extraction: ${error.message}`);
                }
            }
        } catch (error) {
            this.logger.debug(`[Context] Timeline extraction: ${error.message}`);
        }

        return replies;
    }

    /**
     * Clean reply text - preserve meaningful content
     */
    cleanReplyText(text) {
        if (!text) return '';

        // Preserve @mentions that are in-content (not just leading)
        // Remove only excessive whitespace and truncate
        return text
            .replace(/\n+/g, ' ') // Normalize newlines to spaces
            .replace(/\s+/g, ' ') // Collapse multiple spaces
            .trim()
            .substring(0, 280);
    }

    /**
     * Analyze sentiment of text
     */
    analyzeSentiment(text) {
        if (!text) return { overall: 'neutral', positive: 0, negative: 0, score: 0 };

        const lower = text.toLowerCase();
        let positiveCount = 0;
        let negativeCount = 0;

        for (const word of this.sentimentIndicators.positive) {
            if (lower.includes(word)) positiveCount++;
        }
        for (const word of this.sentimentIndicators.negative) {
            if (lower.includes(word)) negativeCount++;
        }

        const total = positiveCount + negativeCount;
        let score = 0;
        let overall = 'neutral';

        if (total > 0) {
            score = (positiveCount - negativeCount) / total;
            overall = score > 0.2 ? 'positive' : score < -0.2 ? 'negative' : 'neutral';
        }

        return {
            overall,
            positive: positiveCount,
            negative: negativeCount,
            score: Math.round(score * 100) / 100,
        };
    }

    /**
     * Detect tone of text
     */
    detectTone(text) {
        if (!text) return { primary: 'neutral', secondary: null, confidence: 0 };

        const lower = text.toLowerCase();
        const tones = {
            humorous: 0,
            informational: 0,
            emotional: 0,
            serious: 0,
            promotional: 0,
        };

        // Count indicators
        for (const word of this.sentimentIndicators.humorous) {
            if (lower.includes(word)) tones.humorous++;
        }
        for (const word of this.sentimentIndicators.informational) {
            if (lower.includes(word)) tones.informational++;
        }
        for (const word of this.sentimentIndicators.emotional) {
            if (lower.includes(word)) tones.emotional++;
        }

        // Check for promotional content
        if (
            lower.includes('link in bio') ||
            lower.includes('check out') ||
            lower.includes('sign up') ||
            lower.includes('buy now') ||
            lower.includes('limited time') ||
            lower.includes('offer')
        ) {
            tones.promotional++;
        }

        // Default to serious if nothing detected
        if (Object.values(tones).every((v) => v === 0)) {
            tones.serious = 1;
        }

        // Find primary tone
        let primary = 'neutral';
        let maxScore = 0;
        for (const [tone, score] of Object.entries(tones)) {
            if (score > maxScore) {
                maxScore = score;
                primary = tone;
            }
        }

        return {
            primary,
            secondary: maxScore > 2 ? null : 'neutral',
            confidence: Math.min(maxScore / 3, 1),
            scores: tones,
        };
    }

    /**
     * Classify conversation type
     */
    classifyConversation(tweetText, replies) {
        if (!tweetText) return 'general';

        const lower = tweetText.toLowerCase();

        // Question detection
        if (
            lower.includes('?') ||
            lower.includes('what') ||
            lower.includes('how') ||
            lower.includes('why') ||
            lower.includes('when') ||
            lower.includes('who')
        ) {
            return 'question';
        }

        // News/Announcement
        if (
            lower.includes('breaking') ||
            lower.includes('update') ||
            lower.includes('just announced') ||
            lower.includes('official statement')
        ) {
            return 'news';
        }

        // Opinion/Thought
        if (
            replies.length > 3 &&
            replies.some(
                (r) =>
                    r.text.toLowerCase().includes('agree') ||
                    r.text.toLowerCase().includes('disagree') ||
                    r.text.toLowerCase().includes('think')
            )
        ) {
            return 'discussion';
        }

        // Fan/Reaction
        if (
            replies.length > 5 &&
            replies.some((r) => r.author !== 'unknown' && r.text.length < 50)
        ) {
            return 'reaction';
        }

        return 'general';
    }

    /**
     * Analyze sentiment of all replies
     */
    analyzeReplySentiment(replies) {
        if (!replies || replies.length === 0) {
            return {
                overall: 'neutral',
                positive: 0,
                negative: 0,
                distribution: { positive: 0, neutral: 0, negative: 0 },
            };
        }

        let positive = 0;
        let negative = 0;
        let neutral = 0;

        for (const reply of replies) {
            const sentiment = this.analyzeSentiment(reply.text);
            if (sentiment.overall === 'positive') positive++;
            else if (sentiment.overall === 'negative') negative++;
            else neutral++;
        }

        const total = replies.length;

        return {
            overall:
                positive > negative ? 'positive' : negative > positive ? 'negative' : 'neutral',
            positive: Math.round((positive / total) * 100),
            negative: Math.round((negative / total) * 100),
            neutral: Math.round((neutral / total) * 100),
            distribution: { positive, neutral, negative },
        };
    }

    /**
     * Build enhanced prompt with all context
     */
    buildEnhancedPrompt(context, systemPrompt) {
        const {
            author,
            replies,
            sentiment,
            tone,
            conversationType,
            replySentiment,
            engagementLevel,
            metrics,
        } = context;

        let enhancedPrompt = systemPrompt + '\n\n=== CONTEXT ===\n';

        // Author info
        enhancedPrompt += `Tweet from: @${author}\n`;

        // Sentiment
        enhancedPrompt += `Tweet sentiment: ${sentiment.overall} (score: ${sentiment.score})\n`;

        // Tone
        enhancedPrompt += `Tweet tone: ${tone.primary}${tone.confidence > 0.5 ? ' (confident)' : ''}\n`;

        // Conversation type
        enhancedPrompt += `Conversation type: ${conversationType}\n`;

        // Engagement level
        if (metrics) {
            enhancedPrompt += `Engagement: ${engagementLevel} (${metrics.likes} likes, ${metrics.retweets} RTs, ${metrics.replies} replies)\n`;
        }

        // Reply sentiment summary
        if (replySentiment) {
            enhancedPrompt += `Reply vibe: ${replySentiment.overall} (${replySentiment.positive}% positive, ${replySentiment.negative}% negative)\n`;
        }

        // Add reply examples - pick longest 30 for richer context
        if (replies.length > 0) {
            const sortedReplies = replies
                .filter((r) => r.text && r.text.length > 5)
                .sort((a, b) => (b.text?.length || 0) - (a.text?.length || 0))
                .slice(0, 30);

            enhancedPrompt += `\nRecent replies:\n`;
            for (const reply of sortedReplies) {
                enhancedPrompt += `- @${reply.author}: "${reply.text.substring(0, 150)}"\n`;
            }
        }

        enhancedPrompt += '\n=== TASK ===\n';
        enhancedPrompt += `Generate a reply that:\n`;
        enhancedPrompt += `- Matches the ${tone.primary} tone\n`;
        enhancedPrompt += `- Fits the ${conversationType} conversation type\n`;
        enhancedPrompt += `- Aligns with the ${sentiment.overall} sentiment\n`;
        enhancedPrompt += `- Is appropriate given the reply sentiment (${replySentiment.overall})\n`;
        enhancedPrompt += `- Stays within 280 characters\n`;
        enhancedPrompt += `- Sounds natural and human-like\n`;

        return enhancedPrompt;
    }
}

export default AIContextEngine;
