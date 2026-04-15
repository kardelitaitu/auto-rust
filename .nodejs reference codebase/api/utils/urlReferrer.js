/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview High-End Anti-Detect Referrer Engine (2025 Standard) - Naturalized
 * @module strategies/EntropyReferrer
 * @description
 * - Generates mathematically diverse, platform-compliant Referrer URLs.
 * - IMPORTS "Natural Privacy" logic: Truncates deep paths for Social Media to match
 * modern browser 'strict-origin-when-cross-origin' policies.
 * - Includes 'Direct' (No Referrer) traffic for realism.
 * - Calculates correct Sec-Fetch-* headers.
 * * @author System Architect
 * @version 6.1.0 (Niche-Entropy-Expanded)
 */

import { api } from '../index.js';
import { randomBytes } from 'crypto';
import { URL } from 'url';
import fs from 'fs';
import path from 'path';

// Load captured VEDs if available
let REAL_VEDS = [];
try {
    const vedPath = path.resolve('config/ved_data.json');
    if (fs.existsSync(vedPath)) {
        REAL_VEDS = JSON.parse(fs.readFileSync(vedPath, 'utf8'));
    }
} catch {
    console.warn('[ReferrerEngine] VED Dictionary not loaded. Using synthetic fallback.');
}

// Load Dictionary
let DICT = {};
let SUBREDDITS = [];

try {
    const dictPath = path.resolve('config/referrer_dict.json');
    if (fs.existsSync(dictPath)) {
        const data = JSON.parse(fs.readFileSync(dictPath, 'utf8'));
        DICT = {
            TOPICS: data.TOPICS || ['tech', 'news'],
            ACTIONS: data.ACTIONS || ['read'],
            CONTEXT: data.CONTEXT || ['thread'],
        };
        SUBREDDITS = data.SUBREDDITS || ['technology'];
    } else {
        console.warn('[ReferrerEngine] Dictionary file missing. Using minimal fallback.');
        // Minimal Fallback to prevent crash
        DICT = {
            TOPICS: ['technology', 'news'],
            ACTIONS: ['update', 'analysis'],
            CONTEXT: ['discussion'],
        };
        SUBREDDITS = ['technology'];
    }
} catch (e) {
    console.error('[ReferrerEngine] Error loading dictionary:', e);
}

// Load Real t.co Links
let REAL_TCO = [];
try {
    const tcoPath = path.resolve('config/tco_links.json');
    if (fs.existsSync(tcoPath)) {
        REAL_TCO = JSON.parse(fs.readFileSync(tcoPath, 'utf8')).filter((l) => l.includes('t.co/'));
    }
} catch {
    console.warn('[ReferrerEngine] t.co Dictionary not loaded.');
}

// --- CONFIGURATION ---

const DEVICE_TYPES = {
    DESKTOP: 'desktop',
};

// --- ENTROPY UTILS ---

const getRandom = (length, type = 'alphanum') => {
    if (type === 'hex')
        return randomBytes(Math.ceil(length / 2))
            .toString('hex')
            .slice(0, length);
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    const bytes = randomBytes(length);
    let res = '';
    for (let i = 0; i < length; i++) res += chars[bytes[i] % chars.length];
    return res;
};

const pick = (arr) => arr[Math.floor(Math.random() * arr.length)];

// --- CONTEXT AWARENESS ---

/**
 * Extracts username and status ID from target URL.
 * Supports: x.com/user, twitter.com/user/status/id, etc.
 */
// Pre-create Set for O(1) lookup instead of O(n) Array.includes()
const RESERVED_WORDS = new Set([
    'home',
    'explore',
    'notifications',
    'messages',
    'search',
    'settings',
]);
const PROFILE_SUBPAGES = new Set(['media', 'with_replies', 'highlights', 'likes']);

const _extractContext = (targetUrl) => {
    try {
        const u = new URL(targetUrl);
        const parts = u.pathname.split('/').filter((p) => p);
        if (parts.length === 0) return null;

        // Case 1: /username/status/123456
        if (parts.length >= 3 && parts[1] === 'status') {
            return { type: 'status', username: parts[0], id: parts[2] };
        }

        // Case 2: /username (Profile) or /username/media
        // Filter out reserved words - O(1) Set lookup instead of O(n) Array.includes()
        if (parts.length >= 1 && !RESERVED_WORDS.has(parts[0])) {
            // Check if it's a profile or profile sub-page
            if (parts.length === 1 || (parts.length === 2 && PROFILE_SUBPAGES.has(parts[1]))) {
                return { type: 'profile', username: parts[0] };
            }
        }

        return null;
    } catch (_e) {
        return null;
    }
};

const generateQuery = (targetUrl = '') => {
    // 1. Contextual Query (High Trust)
    const ctx = _extractContext(targetUrl);

    if (ctx) {
        if (ctx.type === 'status') {
            const templates = [
                // Standard Search
                `${ctx.username} twitter status`,
                `${ctx.username} status`,
                `twitter ${ctx.username} status`,
                `${ctx.username} twitter post`,

                // Natural Language / Questions
                `what did ${ctx.username} tweet`,
                `tweet from ${ctx.username}`,
                `latest tweet by ${ctx.username}`,
                `did ${ctx.username} tweet this`,
                `${ctx.username} twitter update`,
                `${ctx.username} tweet reply`,

                // Media / Content Guessing
                `${ctx.username} twitter video`,
                `${ctx.username} twitter image`,
                `${ctx.username} twitter thread`,
                `${ctx.username} twitter analysis`,
                `${ctx.username} twitter controversy`,

                // Short / Mobile Style
                `${ctx.username} tweet`,
                `${ctx.username} x`,
                `${ctx.username} x post`,
                `${ctx.username} x status`,

                // Advanced / Operators
                `site:twitter.com ${ctx.username} status`,
                `site:x.com ${ctx.username} status`,
                `site:twitter.com ${ctx.username} tweet`,
                `source:twitter ${ctx.username}`,
                `related:twitter.com/${ctx.username}`,
            ];
            return encodeURIComponent(pick(templates));
        }

        if (ctx.type === 'profile') {
            const templates = [
                // Identity Search
                `${ctx.username} twitter`,
                `${ctx.username} twitter profile`,
                `${ctx.username} x profile`,
                `${ctx.username} x.com`,

                // Discovery / Verification
                `who is ${ctx.username} twitter`,
                `is ${ctx.username} on twitter`,
                `${ctx.username} twitter official`,
                `${ctx.username} twitter real`,
                `${ctx.username} twitter account`,

                // Engagement lookups
                `${ctx.username} tweets`,
                `${ctx.username} twitter media`,
                `${ctx.username} twitter followers`,
                `${ctx.username} twitter bio`,

                // Navigational
                `twitter com ${ctx.username}`,
                `x com ${ctx.username}`,
                `goto ${ctx.username} twitter`,

                // Advanced
                `site:twitter.com ${ctx.username}`,
                `site:x.com ${ctx.username}`,
                `@${ctx.username} twitter`,
            ];
            return encodeURIComponent(pick(templates));
        }
    }

    // 2. Generic Fallback
    const topic = pick(DICT.TOPICS);
    const action = pick(DICT.ACTIONS);
    const context = Math.random() > 0.5 ? pick(DICT.CONTEXT) : '';
    return encodeURIComponent(`${topic} ${action} ${context}`.trim());
};

// --- PROTOCOL SIMULATORS ---

const EMERGENCY_VEDS = [
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQ67oDCAU',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQ-0EIBw',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQ3tUDCAo',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQ4G8IDA',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQ39UDCA0',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQ05YFCA4',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQvs8DCA8',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQhqEICBA',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQ4dUDCBE',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQ4d8ICBI',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQ4tUDCBM',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQ68cECBQ',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQlokGCBU',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQmIkGCBY',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQ6scECBk',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQ6psICBo',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQnJQPCBs',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQtsANCBw',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQ8JsICB0',
    '0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQ8ZsICB4',
];

const generateVED = () => {
    // 1. Prefer REAL VEDs if available
    if (REAL_VEDS.length > 0) {
        return pick(REAL_VEDS);
    }

    // 2. Emergency Fallback (Better than synthetic)
    return pick(EMERGENCY_VEDS);
};

const generateSnowflake = () => {
    const twitterEpoch = 1288834974657n;
    const now = BigInt(Date.now());
    const id =
        ((now - twitterEpoch) << 22n) |
        (BigInt(Math.floor(Math.random() * 1024)) << 12n) |
        BigInt(Math.floor(Math.random() * 4096));
    return id.toString();
};

// --- REFERRER STRATEGIES ---

const Strategies = {
    // --- DIRECT TRAFFIC (No Referrer) ---
    // Simulates bookmarks, typed URLs, or strict privacy settings
    direct: () => '',

    // --- SEARCH ENGINES (High Trust, Full Path often allowed) ---
    google_search: (targetUrl) => {
        const tld = 'com'; // STRICT: Only use .com to avoid geo-mismatches
        const query = generateQuery(targetUrl);
        const ved = generateVED();
        const ei = getRandom(12, 'alphanum');
        return `https://www.google.${tld}/search?q=${query}&sourceid=chrome&ie=UTF-8&ved=${ved}&ei=${ei}`;
    },

    bing_search: (targetUrl) => {
        const query = generateQuery(targetUrl);
        const cvid = getRandom(32, 'hex').toUpperCase();
        return `https://www.bing.com/search?q=${query}&form=QBLH&sp=-1&ghc=1&cvid=${cvid}`;
    },

    duckduckgo: (targetUrl) => {
        const query = generateQuery(targetUrl);
        return `https://duckduckgo.com/?q=${query}&t=h_&ia=web`;
    },

    // --- SOCIAL (Naturalized: Deep paths exist but often truncated by policy) ---

    // Twitter t.co (External Wrapper): ALWAYS Full Path
    twitter_tco: () => {
        if (REAL_TCO.length > 0) return pick(REAL_TCO);
        return `https://t.co/${getRandom(10)}`;
    },

    // Reddit: Full path internally, but likely truncated to Origin by browser
    reddit_thread: () => {
        const sub = pick(SUBREDDITS);
        const threadId = getRandom(6, 'alphanum').toLowerCase();
        const slug = pick(DICT.TOPICS).replace(/ /g, '_');
        return `https://www.reddit.com/r/${sub}/comments/${threadId}/${slug}/`;
    },

    // LinkedIn: Generic Feed
    linkedin_feed: () => `https://www.linkedin.com/feed/`,

    // Discord: Web Client Channel
    discord_channel: () => {
        const guildId = generateSnowflake();
        const channelId = generateSnowflake();
        return `https://discord.com/channels/${guildId}/${channelId}`;
    },

    // --- MESSAGING ---

    // Telegram: Web K/A versions
    telegram_web: () => {
        const version = Math.random() > 0.5 ? 'k' : 'a';
        return `https://web.telegram.org/${version}/`;
    },

    // WhatsApp: Web Interface
    whatsapp_web: () => `https://web.whatsapp.com/`,

    // Desktop App Handshakes (API/Gateway)
    whatsapp_api: () => `https://api.whatsapp.com/`,

    // --- NEWS AGGREGATORS (High Trust) ---
    hacker_news: () => `https://news.ycombinator.com/item?id=${getRandom(8, 'alphanum')}`,
    medium_article: () => `https://medium.com/tag/${pick(DICT.TOPICS).replace(/ /g, '-')}`,
    substack: () =>
        `https://${pick(['technews', 'crypto', 'finance', 'daily'])}.substack.com/p/${getRandom(10, 'alphanum')}`,
};

// --- PRIVACY ENGINE (The "Natural" Filter) ---

class PrivacyEngine {
    /**
     * Applies realistic browser Referrer Policies.
     * Most modern browsers use 'strict-origin-when-cross-origin'.
     * This means deep paths from Reddit/Discord are usually stripped to just the domain.
     */
    static naturalize(url, strategyName) {
        if (!url) return ''; // Direct

        // Whitelist: Sources that famously pass full paths/params or are Redirectors
        const fullPathWhitelist = [
            'google_search',
            'bing_search',
            'duckduckgo',
            'twitter_tco', // t.co is designed to be the full referrer
            'whatsapp_api', // often passes minimal path
        ];

        if (fullPathWhitelist.includes(strategyName)) {
            return url;
        }

        // For everything else (Reddit, Discord, LinkedIn, Telegram Web),
        // a real browser usually sends ONLY the Origin.
        // sending /r/subreddit/comments/... to a third party is a privacy leak
        // that browsers prevent by default in 2025.
        try {
            const u = new URL(url);
            return u.origin + '/';
        } catch (_e) {
            /* c8 ignore next */
            return url;
        }
    }
}

// --- HEADER CONTEXT ENGINE ---

class HeaderEngine {
    static getContextHeaders(refererUrl, targetUrl) {
        // Direct traffic has no Referrer header, and specific Fetch metadata
        if (!refererUrl) {
            return {
                'Sec-Fetch-Site': 'none', // User typed URL / Bookmark
                'Sec-Fetch-Mode': 'navigate',
                'Sec-Fetch-User': '?1',
                'Sec-Fetch-Dest': 'document',
            };
        }

        let refHost, targetHost;
        try {
            refHost = new URL(refererUrl).hostname;
            targetHost = new URL(targetUrl).hostname;
        } catch (_e) {
            return {};
        }

        const isSameOrigin = refHost === targetHost;
        const isSameSite = refHost.endsWith(targetHost.split('.').slice(-2).join('.'));
        let site = isSameOrigin ? 'same-origin' : isSameSite ? 'same-site' : 'cross-site';

        return {
            'Sec-Fetch-Site': site,
            'Sec-Fetch-Mode': 'navigate',
            'Sec-Fetch-User': '?1',
            'Sec-Fetch-Dest': 'document',
        };
    }
}

// --- MAIN CLASS ---

export class ReferrerEngine {
    constructor(config = {}) {
        this.deviceType = DEVICE_TYPES.DESKTOP;
        this.addUTM = config.addUTM || false;
        this.api = config.api;
    }

    _selectStrategy(targetUrl) {
        const r = Math.random();

        // CRITICAL SAFETY:
        // Never use 'twitter_tco' if we are visiting Twitter/X itself.
        // t.co is a redirector. You cannot "click a link" on a t.co page to go to Twitter.
        // It creates an Impossible Navigation paradox.
        const isInternalTwitter = targetUrl.includes('twitter.com') || targetUrl.includes('x.com');

        // 1. Direct Traffic (10%) - Essential for realism
        if (r < 0.1) return 'direct';

        // 2. Search (30%) - High Trust
        if (r < 0.25) return 'google_search';
        if (r < 0.35) return 'bing_search';
        if (r < 0.4) return 'duckduckgo';

        // 3. Social (30%)
        if (!isInternalTwitter && r < 0.55) return 'twitter_tco'; // ONLY if external
        if (r < 0.65) return 'reddit_thread'; // Will be truncated to Origin
        if (r < 0.7) return 'discord_channel'; // Will be truncated to Origin

        // 4. Messaging (25%)
        if (r < 0.8) return 'telegram_web';
        if (r < 0.9) return 'whatsapp_web';
        if (r < 0.95) return 'whatsapp_api';

        // 5. Long Tail
        return Math.random() > 0.5
            ? 'linkedin_feed'
            : pick(['hacker_news', 'medium_article', 'substack']);
    }

    generateContext(targetUrl) {
        const strategy = this._selectStrategy(targetUrl);
        // Pass targetUrl to strategy for context-aware generation (Google/Bing)
        const rawReferrer = Strategies[strategy](targetUrl);

        // APPLY NATURAL PRIVACY FILTER
        const naturalReferrer = PrivacyEngine.naturalize(rawReferrer, strategy);

        const fetchHeaders = HeaderEngine.getContextHeaders(naturalReferrer, targetUrl);

        const finalHeaders = {
            ...fetchHeaders,
        };

        // Only add Referer header if it's not direct
        if (naturalReferrer) {
            finalHeaders['Referer'] = naturalReferrer;
        }

        let finalTarget = targetUrl;
        if (this.addUTM) {
            try {
                const u = new URL(targetUrl);
                // We only inject UTMs if the strategy matches the source
                if (strategy.includes('google')) {
                    u.searchParams.set('utm_source', 'google');
                    u.searchParams.set('utm_medium', 'organic');
                } else if (strategy.includes('twitter')) {
                    u.searchParams.set('utm_source', 'twitter');
                    u.searchParams.set('utm_medium', 'social');
                } else if (strategy.includes('whatsapp')) {
                    u.searchParams.set('utm_source', 'whatsapp');
                    u.searchParams.set('utm_medium', 'messenger');
                }
                finalTarget = u.toString();
            } catch (_e) {
                // Ignore invalid URLs
            }
        }

        return {
            strategy,
            referrer: naturalReferrer,
            headers: finalHeaders,
            targetWithParams: finalTarget,
        };
    }

    /**
     * Executes a "True Navigation" by spoofing the referrer via interception.
     * This creates a perfect history.length and navigation.type profile.
     * @param {object} page - Playwright Page
     * @param {string} targetUrl
     * @param {object} [context] - Optional pre-generated context
     */
    async navigate(page, targetUrl, context = null) {
        // Default to Trampoline (Natural) Navigation for supported types
        return this.trampolineNavigate(page, targetUrl, context);
    }

    /**
     * "Trampoline" Navigation:
     * 1. Intercepts request to the FAKE Referrer URL.
     * 2. Serves a dummy page with a link.
     * 3. Clicks the link to generate genuine navigation headers/history.
     */
    async trampolineNavigate(page, targetUrl, context = null) {
        const ctx = context || this.generateContext(targetUrl);
        const useApi = this.api?.goto;

        console.log(`[ReferrerEngine] Strategy: ${ctx.strategy}`);
        console.log(`[ReferrerEngine] Target: ${ctx.targetWithParams}`);
        console.log(`[ReferrerEngine] Spoofed Origin: ${ctx.referrer || '(Direct)'}`);

        // If Direct, just go there
        if (!ctx.referrer || ctx.strategy === 'direct') {
            if (useApi) {
                await this.api.setExtraHTTPHeaders(ctx.headers);
                return this.api.goto(ctx.targetWithParams, {
                    waitUntil: 'domcontentloaded',
                    warmup: false,
                    warmupMouse: false,
                    warmupFakeRead: false,
                    warmupPause: false,
                });
            }
            await api.setExtraHTTPHeaders(ctx.headers);
            return api.goto(ctx.targetWithParams);
        }

        // For complex referrers (Social, Search), use Trampoline
        try {
            // 1. Set up Interception
            // A. Handle Favicon to prevent 404/Network Error signals on this fake page
            await page.route('**/favicon.ico', (route) =>
                route.fulfill({ status: 200, contentType: 'image/x-icon', body: '' })
            );

            // B. Serve the Trampoline Page
            await page.route(ctx.referrer, async (route) => {
                await route.fulfill({
                    status: 200,
                    contentType: 'text/html',
                    body: `
                        <!DOCTYPE html>
                        <html>
                            <head>
                                <title>Redirecting...</title>
                                <link rel="icon" href="data:,">
                                <style>
                                    body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; background: #f9f9f9; color: #333; }
                                    .container { text-align: center; }
                                    .spinner { border: 4px solid #f3f3f3; border-top: 4px solid #3498db; border-radius: 50%; width: 40px; height: 40px; animation: spin 1s linear infinite; margin: 0 auto 20px; }
                                    @keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }
                                    a { color: #3498db; text-decoration: none; font-size: 14px; }
                                    .h { display: none; }
                                </style>
                            </head>
                            <body>
                                <div class="container">
                                    <div class="spinner"></div>
                                    <p>Redirecting to destination...</p>
                                    <a id="trampoline" href="${ctx.targetWithParams}">Click here if not redirected</a>
                                </div>
                                <script>
                                    // REALISTIC NETWORK DELAY (300ms - 800ms)
                                    // A real redirect involves DNS + TLS + TTFB. 
                                    // <150ms is suspiciously fast for cross-origin.
                                    const delay = Math.floor(Math.random() * 500) + 300;
                                    setTimeout(() => {
                                        const link = document.getElementById('trampoline');
                                        if(link) link.click();
                                    }, delay);
                                </script>
                            </body>
                        </html>
                    `,
                });
            });

            // 2. Go to the "Fake" Referrer (which we just routed)
            // This puts the Referrer URL in the address bar momentarily
            if (useApi) {
                await this.api.goto(ctx.referrer, {
                    waitUntil: 'commit',
                    timeout: 30000,
                    warmup: false,
                    warmupMouse: false,
                    warmupFakeRead: false,
                    warmupPause: false,
                });
            } else {
                await api.goto(ctx.referrer, { waitUntil: 'commit' });
            }

            // 3. Wait for the click to trigger navigation to the REAL target
            // The click happens via script above, or we can force it:
            try {
                if (this.api?.click) {
                    await this.api.click('#trampoline', { timeout: 2000 });
                } else {
                    await api.click('#trampoline', { timeout: 2000 });
                }
            } catch (_e) {
                // Ignore if auto-click worked
            }

            // 4. Wait for real target load
            if (useApi && this.api?.waitForURL) {
                await this.api.waitForURL(
                    (url) => url.toString().includes(new URL(targetUrl).hostname),
                    { timeout: 30000, waitUntil: 'domcontentloaded' }
                );
            } else {
                await api.waitForURL(
                    (url) => url.toString().includes(new URL(targetUrl).hostname),
                    { timeout: 30000, waitUntil: 'domcontentloaded' }
                );
            }

            // Cleanup route (optional, but good practice)
            await page.unroute(ctx.referrer);
        } catch (e) {
            console.warn(
                `[ReferrerEngine] Trampoline failed: ${e.message}. Fallback to direct goto.`
            );
            if (useApi) {
                await this.api.setExtraHTTPHeaders(ctx.headers);
                await this.api.goto(ctx.targetWithParams, {
                    waitUntil: 'domcontentloaded',
                    warmup: false,
                    warmupMouse: false,
                    warmupFakeRead: false,
                    warmupPause: false,
                });
                return;
            }
            await api.setExtraHTTPHeaders(ctx.headers);
            await api.goto(ctx.targetWithParams, { referer: ctx.referrer });
        }
    }
}

// --- SELF-TEST ---
/* c8 ignore start */
if (import.meta.url === `file://${process.argv[1]}`) {
    const engine = new ReferrerEngine({ addUTM: true });
    console.log('--- 10-Sample Naturalized Test (Desktop) ---');
    for (let i = 0; i < 10; i++) {
        const ctx = engine.generateContext('https://target.com');
        console.log(
            `[${i + 1}] ${ctx.strategy.padEnd(16)} | ${ctx.referrer ? ctx.referrer.substring(0, 50) + '...' : '(DIRECT)'}`
        );
    }
}
/* c8 ignore stop */
