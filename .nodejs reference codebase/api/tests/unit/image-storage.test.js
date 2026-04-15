/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import ImageStorage from '@api/core/vision/image-storage.js';
import fs from 'fs/promises';

vi.mock('fs/promises');

describe('ImageStorage', () => {
    let storage;
    const testDir = 'test-screenshots';

    beforeEach(() => {
        storage = new ImageStorage(testDir);
        vi.clearAllMocks();
    });

    it('should ensure directory exists on save', async () => {
        fs.mkdir.mockResolvedValue(undefined);
        fs.writeFile.mockResolvedValue(undefined);

        const filepath = await storage.save('test.jpg', Buffer.from('abc'));

        expect(fs.mkdir).toHaveBeenCalledWith(testDir, { recursive: true });
        expect(fs.writeFile).toHaveBeenCalled();
        expect(filepath).toContain(testDir);
    });

    it('should use default directory if not provided', () => {
        const defaultStorage = new ImageStorage();
        expect(defaultStorage.baseDir).toContain('screenshot');
    });

    it('should cleanup old files', async () => {
        const now = Date.now();
        const oldFile = 'old.jpg';
        const newFile = 'new.jpg';

        fs.readdir.mockResolvedValue([oldFile, newFile]);
        fs.stat.mockImplementation(async (p) => {
            if (p.includes(oldFile)) return { mtimeMs: now - 4000000 }; // > 1 hour
            return { mtimeMs: now };
        });
        fs.unlink.mockResolvedValue(undefined);

        const count = await storage.cleanup(3600000);

        expect(count).toBe(1);
        expect(fs.unlink).toHaveBeenCalledTimes(1);
    });

    it('should handle cleanup errors gracefully', async () => {
        fs.readdir.mockRejectedValue(new Error('Access denied'));
        const count = await storage.cleanup();
        expect(count).toBe(0);
    });

    it('should return stats', async () => {
        fs.readdir.mockResolvedValue(['a.jpg', 'b.jpg']);
        fs.stat.mockResolvedValue({ size: 100 });

        const stats = await storage.getStats();

        expect(stats.count).toBe(2);
        expect(stats.size).toBe(200);
    });

    it('should handle stats errors', async () => {
        fs.readdir.mockRejectedValue(new Error('Fail'));
        const stats = await storage.getStats();
        expect(stats.error).toBe('Fail');
    });
});
