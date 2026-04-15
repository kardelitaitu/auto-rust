/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import RoIDetector from '@api/core/vision/roi-detector.js';

describe('RoIDetector', () => {
    let detector;
    let mockPage;
    let mockElement;

    beforeEach(() => {
        detector = new RoIDetector();
        mockElement = {
            boundingBox: vi.fn(),
        };
        mockPage = {
            $: vi.fn(),
        };
    });

    it('should return ROI when modal is found and has bounding box', async () => {
        mockPage.$.mockResolvedValue(mockElement);
        mockElement.boundingBox.mockResolvedValue({ x: 100, y: 100, width: 200, height: 200 });

        const roi = await detector.detect(mockPage);

        expect(roi).toEqual({
            x: 80, // 100 - 20
            y: 80, // 100 - 20
            width: 240, // 200 + 40
            height: 240, // 200 + 40
        });
    });

    it('should clamp negative coordinates to 0', async () => {
        mockPage.$.mockResolvedValue(mockElement);
        mockElement.boundingBox.mockResolvedValue({ x: 10, y: 10, width: 200, height: 200 });

        const roi = await detector.detect(mockPage);

        expect(roi.x).toBe(0);
        expect(roi.y).toBe(0);
    });

    it('should return null if no modal found', async () => {
        mockPage.$.mockResolvedValue(null);
        const roi = await detector.detect(mockPage);
        expect(roi).toBeNull();
    });

    it('should return null if modal has no bounding box', async () => {
        mockPage.$.mockResolvedValue(mockElement);
        mockElement.boundingBox.mockResolvedValue(null);
        const roi = await detector.detect(mockPage);
        expect(roi).toBeNull();
    });

    it('should return null on error', async () => {
        mockPage.$.mockRejectedValue(new Error('Page crashed'));
        const roi = await detector.detect(mockPage);
        expect(roi).toBeNull();
    });
});
