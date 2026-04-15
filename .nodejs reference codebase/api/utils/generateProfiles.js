/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const OUTPUT_FILE = path.join(__dirname, '../../data/twitterActivityProfiles.json');

// Tweaked Bounds
const BOUNDS = {
    Skimmer: {
        readingPhase: { mean: 10000, deviation: 3000 },
        scrollPause: { mean: 800, deviation: 300 },
        refresh: 0.2,
        dive: 0.2,
        tweetDive: 0.05,
        like: 0.01,
        bookmark: 0.002,
        follow: 0.001,
        tweet: 0.01, // 1%
    },
    Balanced: {
        readingPhase: { mean: 15000, deviation: 5000 },
        scrollPause: { mean: 2000, deviation: 800 },
        refresh: 0.1,
        dive: 0.3,
        tweetDive: 0.1,
        like: 0.015,
        bookmark: 0.005,
        follow: 0.003,
        tweet: 0.05, // 5%
    },
    DeepDiver: {
        readingPhase: { mean: 20000, deviation: 5000 },
        scrollPause: { mean: 4000, deviation: 1500 },
        refresh: 0.05,
        dive: 0.6,
        tweetDive: 0.2,
        like: 0.018,
        bookmark: 0.02,
        follow: 0.005,
        tweet: 0.02, // 2%
    },
    Lurker: {
        readingPhase: { mean: 20000, deviation: 8000 },
        scrollPause: { mean: 3000, deviation: 1000 },
        refresh: 0.05,
        dive: 0.1,
        tweetDive: 0.15,
        like: 0.005,
        bookmark: 0.03,
        follow: 0.001,
        tweet: 0.001, // 0.1%
    },
    DoomScroller: {
        readingPhase: { mean: 8000, deviation: 2000 },
        scrollPause: { mean: 600, deviation: 200 },
        refresh: 0.1,
        dive: 0.05,
        tweetDive: 0.02,
        like: 0.005,
        bookmark: 0.001,
        follow: 0.0005,
        tweet: 0.001, // 0.1%
    },
    NewsJunkie: {
        readingPhase: { mean: 12000, deviation: 4000 },
        scrollPause: { mean: 1200, deviation: 400 },
        refresh: 0.6,
        dive: 0.1,
        tweetDive: 0.3,
        like: 0.01,
        bookmark: 0.025,
        follow: 0.01,
        tweet: 0.15, // 15%
    },
    Stalker: {
        readingPhase: { mean: 15000, deviation: 4000 },
        scrollPause: { mean: 2000, deviation: 800 },
        refresh: 0.1,
        dive: 0.9,
        tweetDive: 0.05,
        like: 0.01,
        bookmark: 0.005,
        follow: 0.02,
        tweet: 0.01, // 1%
    },
};

class ProfileFactory {
    static gaussian(mean, stdev) {
        let u = 0,
            v = 0;
        while (u === 0) u = Math.random();
        while (v === 0) v = Math.random();
        return mean + stdev * Math.sqrt(-2.0 * Math.log(u)) * Math.cos(2.0 * Math.PI * v);
    }

    static round(num) {
        return Math.round(num * 10000) / 10000; // 4 decimals
    }

    static create(index, type = 'Balanced') {
        const bounds = BOUNDS[type];

        // 1. Timings
        const pReading = {
            mean: Math.floor(Math.max(5000, this.gaussian(bounds.readingPhase.mean, 5000))),
            deviation: Math.floor(
                Math.max(2000, this.gaussian(bounds.readingPhase.deviation, 1000))
            ),
        };

        const pScroll = {
            mean: Math.floor(Math.max(200, this.gaussian(bounds.scrollPause.mean, 200))),
            deviation: Math.floor(Math.max(50, this.gaussian(bounds.scrollPause.deviation, 50))),
        };

        // 2. Initial Probabilities
        let pRefresh = Math.max(0, Math.min(0.8, this.gaussian(bounds.refresh, 0.05)));
        let pDive = Math.max(0, Math.min(0.8, this.gaussian(bounds.dive, 0.1)));
        let pIdle = Math.max(0, Math.min(0.3, this.gaussian(0.1, 0.05)));

        // New Detailed Probabilities
        let pTweetDive = Math.max(0, Math.min(0.9, this.gaussian(bounds.tweetDive, 0.05)));
        // Enforce 0.5% min, 2% max for Likes
        let pLikeAfter = Math.max(0.005, Math.min(0.02, this.gaussian(bounds.like, 0.005)));
        let pBookmark = Math.max(0, Math.min(0.1, this.gaussian(bounds.bookmark, 0.005)));
        let pFollow = Math.max(0, Math.min(0.01, this.gaussian(bounds.follow, 0.001)));

        // Tweet (Post) Probability
        let pTweet = Math.max(0, Math.min(0.3, this.gaussian(bounds.tweet, 0.02)));

        // 3. New Input Logic (Mouse vs Keyboard Personas)
        const isMouseUser = type === 'DoomScroller' ? true : Math.random() < 0.9;
        let inputP;

        if (isMouseUser) {
            const wheelBase = type === 'DoomScroller' ? 0.95 : 0.85;
            const wheelDown = this.gaussian(wheelBase, 0.05);
            const wheelUp = Math.random() * 0.04 + 0.01;
            // const remaining = 1.0 - (wheelDown + wheelUp);
            let wDown = Math.max(0.7, wheelDown);
            let wUp = Math.max(0.01, wheelUp);
            let sp = Math.max(0, 1 - wDown - wUp);

            inputP = {
                wheelDown: this.round(wDown),
                wheelUp: this.round(wUp),
                space: this.round(sp),
                keysDown: 0,
                keysUp: 0,
            };
        } else {
            const keysDown = this.gaussian(0.85, 0.05);
            const keysUp = Math.random() * 0.04 + 0.01;
            let kDown = Math.max(0.7, keysDown);
            let kUp = Math.max(0.01, keysUp);
            let sp = Math.max(0, 1 - kDown - kUp);

            inputP = {
                wheelDown: 0,
                wheelUp: 0,
                space: this.round(sp),
                keysDown: this.round(kDown),
                keysUp: this.round(kUp),
            };
        }

        pRefresh = this.round(pRefresh);
        pDive = this.round(pDive);
        pIdle = this.round(pIdle);
        pTweetDive = this.round(pTweetDive);
        pLikeAfter = this.round(pLikeAfter);
        pBookmark = this.round(pBookmark);
        pFollow = this.round(pFollow);
        pTweet = this.round(pTweet);

        // Generate ID and Description
        const id = `${String(index).padStart(2, '0')}-${type}`;
        const inputDesc = isMouseUser
            ? `Mouse (${(inputP.wheelDown * 100).toFixed(0)}%)`
            : `Keys (${(inputP.keysDown * 100).toFixed(0)}%)`;

        const desc = `Type: ${type} | Input: ${inputDesc} | Dive: ${(pDive * 100).toFixed(1)}% | Tweet: ${(pTweet * 100).toFixed(1)}% | T-Dive: ${(pTweetDive * 100).toFixed(1)}% Like: ${(pLikeAfter * 100).toFixed(2)}%`;

        return {
            id,
            description: desc,
            timings: {
                readingPhase: pReading,
                scrollPause: pScroll,
                actionSpecific: {
                    space: {
                        mean: Math.floor(Math.random() * (1200 - 800) + 800),
                        deviation: Math.floor(Math.random() * (300 - 100) + 100),
                    },
                    keys: {
                        mean: Math.floor(Math.random() * (150 - 80) + 80),
                        deviation: Math.floor(Math.random() * (50 - 20) + 20),
                    },
                    // Idle duration (staring at screen) - Increased as requested
                    idle: {
                        mean: 15000,
                        deviation: 5000,
                    },
                },
            },
            probabilities: {
                refresh: pRefresh,
                profileDive: pDive,
                tweetDive: pTweetDive,
                likeTweetafterDive: pLikeAfter,
                bookmarkAfterDive: pBookmark,
                followOnProfile: pFollow,
                tweet: pTweet,
                // Reverted to original range logic (approx 10-30%)
                idle: pIdle,
            },
            inputMethods: inputP,
            maxLike: 2,
            maxFollow: 1,
            theme: 'dark',
        };
    }
}

// Generate Profiles
const profileCounts = {
    Skimmer: 4,
    Balanced: 26,
    DeepDiver: 4,
    Lurker: 4,
    DoomScroller: 4,
    NewsJunkie: 4,
    Stalker: 4,
};

let profiles = [];
let idx = 1;

for (const [type, count] of Object.entries(profileCounts)) {
    for (let i = 0; i < count; i++) {
        profiles.push(ProfileFactory.create(idx++, type));
    }
}

try {
    fs.writeFileSync(OUTPUT_FILE, JSON.stringify(profiles, null, 2));
    console.log(`[SUCCESS] Generated ${profiles.length} profiles to ${OUTPUT_FILE}`);
    // Preview a few
    console.log('Preview:');
    profiles.slice(0, 3).forEach((p) => console.log(p.description));
} catch (e) {
    console.error('Failed to save profiles:', e);
}
