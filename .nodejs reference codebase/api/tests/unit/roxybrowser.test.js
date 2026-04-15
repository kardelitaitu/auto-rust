/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for connectors/discovery/roxybrowser.js
 * @module tests/unit/roxybrowser.test
 */

import { describe, it, expect, vi, beforeAll, beforeEach, afterEach } from 'vitest';

const mockApiHandler = {
    get: vi.fn(),
};

vi.mock('@api/utils/apiHandler.js', () => ({
    default: mockApiHandler,
}));

vi.mock('@api/utils/envLoader.js', () => ({
    getEnv: vi.fn((key, def) => {
        if (key === 'ROXYBROWSER_API_KEY') return 'test-key';
        if (key === 'ROXYBROWSER_API_URL') return 'http://127.0.0.1:50000/';
        return def;
    }),
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn().mockReturnValue({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    }),
}));

vi.mock('@api/connectors/baseDiscover.js', () => ({
    default: class BaseDiscover {
        constructor() {
            this.browserType = 'base';
            this.logger = {
                info: vi.fn(),
                warn: vi.fn(),
                error: vi.fn(),
                debug: vi.fn(),
            };
        }
        async discover() {
            throw new Error('Method "discover()" must be implemented by subclass');
        }
    },
}));

describe('connectors/discovery/roxybrowser', () => {
    let RoxybrowserDiscover;

    beforeAll(async () => {
        const module = await import('../../connectors/discovery/roxybrowser.js');
        RoxybrowserDiscover = module.default;
    });

    beforeEach(() => {
        vi.clearAllMocks();
    });

    afterEach(() => {
        vi.restoreAllMocks();
    });

    it('should return empty array if API key is missing', async () => {
        const { getEnv } = await import('@api/utils/envLoader.js');
        getEnv.mockImplementationOnce((key, def) => {
            if (key === 'ROXYBROWSER_API_KEY') return undefined;
            return def;
        });

        const discoverer = new RoxybrowserDiscover();
        const results = await discoverer.discover();

        expect(results).toEqual([]);
    });

    it('should handle no open profiles from API', async () => {
        const mockResponse = {
            code: 0,
            data: [],
        };
        mockApiHandler.get.mockResolvedValue(mockResponse);

        const discoverer = new RoxybrowserDiscover();
        const results = await discoverer.discover();

        expect(results).toEqual([]);
    });

    it('should discover running profiles successfully', async () => {
        const mockResponse = {
            code: 0,
            data: [
                {
                    id: 'p1',
                    name: 'Profile 1',
                    ws: 'ws://localhost:1234',
                    http: 'http://localhost:1234',
                    sortNum: 1,
                },
            ],
        };
        mockApiHandler.get.mockResolvedValue(mockResponse);

        const discoverer = new RoxybrowserDiscover();
        const results = await discoverer.discover();

        expect(results).toHaveLength(1);
        expect(results[0].id).toBe('p1');
        expect(results[0].ws).toBe('ws://localhost:1234');
    });

    it('should handle API errors gracefully', async () => {
        mockApiHandler.get.mockRejectedValue(new Error('Network error'));

        const discoverer = new RoxybrowserDiscover();
        const results = await discoverer.discover();

        expect(results).toEqual([]);
    });

    it('should filter profiles missing connection info', async () => {
        const mockResponse = {
            code: 0,
            data: [
                { id: 'p1', name: 'Valid', ws: 'ws://...' },
                { id: 'p2', name: 'Invalid' }, // missing ws and http
            ],
        };
        mockApiHandler.get.mockResolvedValue(mockResponse);

        const discoverer = new RoxybrowserDiscover();
        const results = await discoverer.discover();

        expect(results).toHaveLength(1);
        expect(results[0].id).toBe('p1');
    });
});
