/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Fingerprint Manager for api/ module.
 * Internal copy for api/ independence from utils/fingerprintManager.js.
 * Loads fingerprint profiles from data/fingerprints.json.
 *
 * @module api/utils/fingerprint
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { mathUtils } from './math.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const DATA_FILE = path.join(__dirname, '..', '..', 'data', 'fingerprints.json');

let fingerprints = [];

try {
    if (fs.existsSync(DATA_FILE)) {
        fingerprints = JSON.parse(fs.readFileSync(DATA_FILE, 'utf8'));
    }
} catch (e) {
    console.error('Failed to load fingerprints:', e);
}

export const fingerprintManager = {
    getAll: () => fingerprints,

    getByPlatform: (platform) => {
        return fingerprints.filter((fp) =>
            fp.platform.toLowerCase().includes(platform.toLowerCase())
        );
    },

    getRandom: () => {
        return mathUtils.sample(fingerprints);
    },

    /**
     * Get a fingerprint that matches the given user agent (heuristic).
     * @param {string} userAgent
     */
    matchUserAgent: (userAgent) => {
        if (!userAgent) return mathUtils.sample(fingerprints);

        const ua = userAgent.toLowerCase();
        const platform = ua.includes('windows')
            ? 'Win'
            : ua.includes('macintosh')
              ? 'Mac'
              : ua.includes('linux')
                ? 'Linux'
                : null;

        if (platform) {
            const matches = fingerprints.filter((fp) => fp.platform.includes(platform));
            if (matches.length > 0) return mathUtils.sample(matches);
        }

        return mathUtils.sample(fingerprints);
    },
};
