/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import LocalChromeDiscover from '@api/connectors/discovery/localChrome.js';
import LocalBraveDiscover from '@api/connectors/discovery/localBrave.js';
import LocalEdgeDiscover from '@api/connectors/discovery/localEdge.js';
import LocalVivaldiDiscover from '@api/connectors/discovery/localVivaldi.js';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

// Mock global fetch to prevent actual network requests during discovery tests

describe('LocalChromeDiscover', () => {
    let discover;

    beforeEach(() => {
        global.fetch = vi.fn().mockRejectedValue(new Error('Fetch failed'));

        vi.clearAllMocks();
        discover = new LocalChromeDiscover();
    });

    it('should initialize with correct browser type', () => {
        expect(discover.browserType).toBe('localChrome');
    });

    it('should return empty array as placeholder', async () => {
        const result = await discover.discover();
        expect(result).toEqual([]);
    });
});

describe('LocalBraveDiscover', () => {
    let discover;

    beforeEach(() => {
        global.fetch = vi.fn().mockRejectedValue(new Error('Fetch failed'));

        vi.clearAllMocks();
        discover = new LocalBraveDiscover();
    });

    it('should initialize with correct browser type', () => {
        expect(discover.browserType).toBe('localBrave');
    });

    it('should return empty array as placeholder', async () => {
        const result = await discover.discover();
        expect(result).toEqual([]);
    });
});

describe('LocalEdgeDiscover', () => {
    let discover;

    beforeEach(() => {
        global.fetch = vi.fn().mockRejectedValue(new Error('Fetch failed'));

        vi.clearAllMocks();
        discover = new LocalEdgeDiscover();
    });

    it('should initialize with correct browser type', () => {
        expect(discover.browserType).toBe('localEdge');
    });

    it('should return empty array as placeholder', async () => {
        const result = await discover.discover();
        expect(result).toEqual([]);
    });
});

describe('LocalVivaldiDiscover', () => {
    let discover;

    beforeEach(() => {
        global.fetch = vi.fn().mockRejectedValue(new Error('Fetch failed'));

        vi.clearAllMocks();
        discover = new LocalVivaldiDiscover();
    });

    it('should initialize with correct browser type', () => {
        expect(discover.browserType).toBe('localVivaldi');
    });

    it('should return empty array as placeholder', async () => {
        const result = await discover.discover();
        expect(result).toEqual([]);
    });
});
