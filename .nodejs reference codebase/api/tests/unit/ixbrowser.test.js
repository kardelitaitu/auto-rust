/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for connectors/discovery/ixbrowser.js
 * @module tests/unit/ixbrowser.test
 */

import { describe, it, expect, vi, beforeAll, beforeEach, afterEach } from 'vitest';

const mockApiHandler = {
    post: vi.fn(),
};

vi.mock('@api/utils/apiHandler.js', () => ({
    default: mockApiHandler,
}));

vi.mock('@api/utils/envLoader.js', () => ({
    getEnv: vi.fn((key, def) => def),
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

describe('connectors/discovery/ixbrowser', () => {
    let IxbrowserDiscover;

    beforeAll(async () => {
        const module = await import('../../connectors/discovery/ixbrowser.js');
        IxbrowserDiscover = module.default;
    });

    beforeEach(() => {
        vi.clearAllMocks();
    });

    afterEach(() => {
        vi.restoreAllMocks();
    });

    it('should discover running profiles from array response', async () => {
        const mockResponse = {
            error: { code: 0 },
            data: [
                {
                    profile_id: '123',
                    debugging_port: 53201,
                    debugging_address: '127.0.0.1:53201',
                    pid: 1001,
                },
            ],
        };
        mockApiHandler.post.mockResolvedValue(mockResponse);

        const discoverer = new IxbrowserDiscover();
        const results = await discoverer.discover();

        expect(results).toHaveLength(1);
        expect(results[0].id).toBe('ixbrowser-123');
        expect(results[0].port).toBe(53201);
    });

    it('should discover running profiles from object response', async () => {
        const mockResponse = {
            error: { code: 0 },
            data: {
                'profile-1': {
                    profile_id: '456',
                    debugging_port: 53202,
                    debugging_address: '127.0.0.1:53202',
                },
            },
        };
        mockApiHandler.post.mockResolvedValue(mockResponse);

        const discoverer = new IxbrowserDiscover();
        const results = await discoverer.discover();

        expect(results).toHaveLength(1);
        expect(results[0].id).toBe('ixbrowser-456');
    });

    it('should handle no open profiles found', async () => {
        const mockResponse = {
            error: { code: 0 },
            data: [],
        };
        mockApiHandler.post.mockResolvedValue(mockResponse);

        const discoverer = new IxbrowserDiscover();
        const results = await discoverer.discover();

        expect(results).toEqual([]);
    });

    it('should filter profiles missing connection info', async () => {
        const mockResponse = {
            error: { code: 0 },
            data: [
                { profile_id: '123' }, // Missing debugging_port and ws
            ],
        };
        mockApiHandler.post.mockResolvedValue(mockResponse);

        const discoverer = new IxbrowserDiscover();
        const results = await discoverer.discover();

        expect(results).toEqual([]);
    });

    it('should handle missing API URL', async () => {
        const { getEnv } = await import('@api/utils/envLoader.js');
        getEnv.mockImplementation((key, def) => {
            if (key === 'IXBROWSER_API_URL') return undefined;
            return def;
        });

        const discoverer = new IxbrowserDiscover();
        const results = await discoverer.discover();

        expect(results).toEqual([]);
    });

    it('should handle exceptions', async () => {
        mockApiHandler.post.mockRejectedValue(new Error('Connection failed'));

        const discoverer = new IxbrowserDiscover();
        const results = await discoverer.discover();

        expect(results).toEqual([]);
    });
});
