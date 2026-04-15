/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import path from 'path';

const mockExistsSync = vi.fn();
const mockMkdirSync = vi.fn();

vi.mock('fs', () => ({
    existsSync: mockExistsSync,
    mkdirSync: mockMkdirSync,
    default: {
        existsSync: mockExistsSync,
        mkdirSync: mockMkdirSync,
    },
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        debug: vi.fn(),
        error: vi.fn(),
    })),
}));

describe('api/utils/screenshot.js', () => {
    let mockPage;
    let takeScreenshot;

    beforeEach(async () => {
        vi.clearAllMocks();
        vi.useFakeTimers();

        mockExistsSync.mockReturnValue(true);

        mockPage = {
            screenshot: vi.fn().mockResolvedValue(undefined),
        };

        // Import after mocks are set up
        const module = await import('@api/utils/screenshot.js');
        takeScreenshot = module.takeScreenshot;
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it('should take a screenshot with default parameters', async () => {
        const result = await takeScreenshot(mockPage);

        expect(mockPage.screenshot).toHaveBeenCalledWith({
            path: expect.stringContaining('.jpg'),
            type: 'jpeg',
            quality: 30,
            fullPage: false,
        });
        expect(result).toBeDefined();
    });

    it('should take a screenshot with custom session name', async () => {
        const result = await takeScreenshot(mockPage, 'my-session');

        expect(mockPage.screenshot).toHaveBeenCalled();
        expect(result).toContain('my-session');
    });

    it('should take a screenshot with custom suffix', async () => {
        const result = await takeScreenshot(mockPage, 'session', '_error');

        expect(result).toContain('session_error');
    });

    it('should sanitize session name with special characters', async () => {
        const result = await takeScreenshot(mockPage, 'session:1/test');

        // The sanitized name should be in the result (session-1-test)
        const filename = result.split(path.sep).pop();
        expect(filename).toContain('session-1-test');
    });

    it('should include timestamp in filename', async () => {
        vi.setSystemTime(new Date('2026-03-05T12:00:00.000Z'));

        const result = await takeScreenshot(mockPage, 'test');

        expect(result).toContain('2026-03-05');
        expect(result).toContain('12-00-00');
    });

    it('should create screenshot directory if it does not exist', async () => {
        mockExistsSync.mockReturnValueOnce(false);

        await takeScreenshot(mockPage, 'test');

        expect(mockMkdirSync).toHaveBeenCalled();
    });

    it('should return null when screenshot fails', async () => {
        mockPage.screenshot.mockRejectedValueOnce(new Error('Screenshot failed'));

        const result = await takeScreenshot(mockPage);

        expect(result).toBeNull();
    });

    it('should use jpeg format with quality 30', async () => {
        await takeScreenshot(mockPage);

        expect(mockPage.screenshot).toHaveBeenCalledWith(
            expect.objectContaining({
                type: 'jpeg',
                quality: 30,
            })
        );
    });

    it('should not capture fullPage by default', async () => {
        await takeScreenshot(mockPage);

        expect(mockPage.screenshot).toHaveBeenCalledWith(
            expect.objectContaining({
                fullPage: false,
            })
        );
    });
});
