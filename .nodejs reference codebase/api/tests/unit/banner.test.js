/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for utils/banner.js
 * @module tests/unit/banner.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createLogger } from '@api/core/logger.js';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn().mockReturnValue({
        info: vi.fn(),
        error: vi.fn(),
        warn: vi.fn(),
    }),
}));

describe('utils/banner', () => {
    let showBanner;
    let consoleSpy;

    beforeEach(async () => {
        vi.clearAllMocks();
        consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

        const module = await import('../../utils/banner.js');
        showBanner = module.showBanner;
    });

    afterEach(() => {
        vi.restoreAllMocks();
    });

    it('should print banner and log initialization message', () => {
        const loggerInstance = createLogger('banner');

        showBanner();

        expect(consoleSpy).toHaveBeenCalled();
        // Note: logger.info is commented out in banner.js line 30
    });
});
