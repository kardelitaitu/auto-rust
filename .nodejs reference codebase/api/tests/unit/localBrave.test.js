/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import LocalBraveDiscover from '@api/connectors/discovery/localBrave.js';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('LocalBraveDiscover', () => {
    let discoverer;

    beforeEach(() => {
        vi.clearAllMocks();
        discoverer = new LocalBraveDiscover();
        global.fetch = vi.fn();
    });

    it('should initialize correctly', () => {
        expect(discoverer.browserType).toBe('localBrave');
    });

    it('should discover a running Brave instance', async () => {
        global.fetch.mockImplementation(async (url) => {
            if (url.includes(':9005/json/version')) {
                return {
                    ok: true,
                    json: async () => ({
                        webSocketDebuggerUrl: 'ws://127.0.0.1:9005/devtools/browser/xyz',
                    }),
                };
            }
            if (url.includes(':9005/json')) {
                return {
                    ok: true,
                    json: async () => [
                        {
                            id: 'tab1',
                            webSocketDebuggerUrl: 'ws://127.0.0.1:9005/devtools/page/abc',
                        },
                    ],
                };
            }
            return { ok: false };
        });

        const profiles = await discoverer.discover();
        expect(profiles.length).toBe(1);
        expect(profiles[0].id).toBe('brave-local-9005');
        expect(profiles[0].ws).toBe('ws://127.0.0.1:9005/devtools/browser/xyz');
    });

    it('should fallback to regex if /json/version fails', async () => {
        global.fetch.mockImplementation(async (url) => {
            if (url.includes(':9010/json/version')) {
                throw new Error('version error');
            }
            if (url.includes(':9010/json')) {
                return {
                    ok: true,
                    json: async () => [
                        {
                            id: 'tab1',
                            webSocketDebuggerUrl: 'ws://127.0.0.1:9010/devtools/page/abc',
                            version: '1.2.3',
                            userAgent: 'Mock Brave',
                        },
                    ],
                };
            }
            return { ok: false };
        });

        const profiles = await discoverer.discover();
        expect(profiles.length).toBe(1);
        expect(profiles[0].ws).toContain('ws://127.0.0.1:9010/devtools/browser/');
        expect(profiles[0].browserVersion).toBe('1.2.3');
    });

    it('should return empty array if no instances found', async () => {
        global.fetch.mockResolvedValue({ ok: false });

        const profiles = await discoverer.discover();
        expect(profiles).toEqual([]);
    });

    it('should handle invalid JSON response', async () => {
        global.fetch.mockImplementation(async (url) => {
            if (url.includes(':9015/json')) {
                return {
                    ok: true,
                    json: async () => {
                        throw new Error('invalid json');
                    },
                };
            }
            return { ok: false };
        });

        const profiles = await discoverer.discover();
        expect(profiles).toEqual([]);
    });
});
