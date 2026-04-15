/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Image Storage - Manages filesystem I/O for screenshots.
 * @module core/vision/image-storage
 */

import fs from 'fs/promises';
import path from 'path';

class ImageStorage {
    constructor(baseDir) {
        this.baseDir = baseDir || path.join(process.cwd(), 'screenshot');
    }

    async ensureDir() {
        await fs.mkdir(this.baseDir, { recursive: true });
    }

    async save(filename, buffer) {
        const filepath = path.join(this.baseDir, filename);
        await this.ensureDir();
        await fs.writeFile(filepath, buffer);
        return filepath;
    }

    async cleanup(maxAgeMs = 3600000) {
        try {
            const files = await fs.readdir(this.baseDir);
            const now = Date.now();
            let deletedCount = 0;

            for (const file of files) {
                const filepath = path.join(this.baseDir, file);
                const stats = await fs.stat(filepath);
                if (now - stats.mtimeMs > maxAgeMs) {
                    await fs.unlink(filepath);
                    deletedCount++;
                }
            }
            return deletedCount;
        } catch (_error) {
            return 0; // Directory might not exist or other error
        }
    }

    async getStats() {
        try {
            const files = await fs.readdir(this.baseDir);
            let totalSize = 0;
            for (const file of files) {
                const filepath = path.join(this.baseDir, file);
                const stats = await fs.stat(filepath);
                totalSize += stats.size;
            }
            return {
                count: files.length,
                size: totalSize,
            };
        } catch (error) {
            return { count: 0, size: 0, error: error.message };
        }
    }
}

export default ImageStorage;
