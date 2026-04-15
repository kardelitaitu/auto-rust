/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { getSettings, clearSettingsCache } from '@api/utils/config.js';
import fs from 'fs/promises';
import path from 'path';

vi.mock('fs/promises', () => ({
    default: {
        readFile: vi.fn(),
    },
}));

describe('api/utils/config.js', () => {
    const expectedSettingsPath = path.resolve(process.cwd(), 'config', 'settings.json');

    beforeEach(() => {
        vi.clearAllMocks();
        clearSettingsCache();
    });

    afterEach(() => {
        clearSettingsCache();
    });

    describe('getSettings', () => {
        it('should return cached settings on subsequent calls', async () => {
            const mockData = { test: 'value' };
            fs.readFile.mockResolvedValue(JSON.stringify(mockData));

            const result1 = await getSettings();
            const result2 = await getSettings();

            expect(result1).toEqual(mockData);
            expect(result2).toEqual(mockData);
            expect(fs.readFile).toHaveBeenCalledTimes(1);
        });

        it('should read settings from config/settings.json', async () => {
            const mockData = { llm: { provider: 'ollama' } };
            fs.readFile.mockResolvedValue(JSON.stringify(mockData));

            const result = await getSettings();

            expect(result).toEqual(mockData);
            expect(fs.readFile).toHaveBeenCalled();
        });

        it("should return empty object if file doesn't exist", async () => {
            const error = new Error('File not found');
            error.code = 'ENOENT';
            fs.readFile.mockRejectedValue(error);

            const result = await getSettings();

            expect(result).toEqual({});
        });

        it('should return empty object if JSON is invalid', async () => {
            fs.readFile.mockResolvedValue('invalid json');

            const result = await getSettings();

            expect(result).toEqual({});
        });

        it('should return empty object on other read errors', async () => {
            fs.readFile.mockRejectedValue(new Error('Permission denied'));

            const result = await getSettings();

            expect(result).toEqual({});
        });

        it('should handle empty file content', async () => {
            fs.readFile.mockResolvedValue('');

            const result = await getSettings();

            expect(result).toEqual({});
        });

        it('should parse nested JSON correctly', async () => {
            const mockData = {
                twitter: {
                    timing: {
                        scrollDelay: 500,
                    },
                },
            };
            fs.readFile.mockResolvedValue(JSON.stringify(mockData));

            const result = await getSettings();

            expect(result.twitter.timing.scrollDelay).toBe(500);
        });

        it('should cache null settings correctly', async () => {
            fs.readFile.mockResolvedValue(JSON.stringify(null));

            const result1 = await getSettings();
            const result2 = await getSettings();

            expect(result1).toEqual(null);
            expect(result2).toEqual(null);
            expect(fs.readFile).toHaveBeenCalledTimes(1);
        });

        it('should use the correct file path', async () => {
            const mockData = { test: 'value' };
            fs.readFile.mockResolvedValue(JSON.stringify(mockData));

            await getSettings();

            expect(fs.readFile).toHaveBeenCalledWith(expectedSettingsPath, 'utf8');
        });

        it('should handle large JSON files correctly', async () => {
            const mockData = { large: 'object', with: { nested: 'structure' } };
            fs.readFile.mockResolvedValue(JSON.stringify(mockData));

            const result = await getSettings();

            expect(result).toEqual(mockData);
        });
    });

    describe('clearSettingsCache', () => {
        it('should clear the cache and re-read file on next call', async () => {
            const mockData = { version: 1 };
            fs.readFile.mockResolvedValue(JSON.stringify(mockData));

            await getSettings();
            clearSettingsCache();

            // After clearing, next call should read from file again
            fs.readFile.mockResolvedValue(JSON.stringify({ version: 2 }));
            const result = await getSettings();

            expect(result.version).toBe(2);
            expect(fs.readFile).toHaveBeenCalledTimes(2);
        });

        it('should allow multiple clears', () => {
            expect(() => clearSettingsCache()).not.toThrow();
            expect(() => clearSettingsCache()).not.toThrow();
        });

        it('should not throw when cache is already null', () => {
            expect(() => clearSettingsCache()).not.toThrow();
        });

        it('should clear cache so next call reads from file', async () => {
            const mockData1 = { version: 1 };
            const mockData2 = { version: 2 };

            fs.readFile.mockResolvedValue(JSON.stringify(mockData1));

            const result1 = await getSettings();
            expect(result1).toEqual(mockData1);

            clearSettingsCache();

            fs.readFile.mockResolvedValue(JSON.stringify(mockData2));
            const result2 = await getSettings();
            expect(result2).toEqual(mockData2);
            expect(fs.readFile).toHaveBeenCalledTimes(2);
        });
    });
});
