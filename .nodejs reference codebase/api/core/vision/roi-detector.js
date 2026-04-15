/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview ROI Detector - Identifies regions of interest in Playwright pages.
 * @module core/vision/roi-detector
 */

class RoIDetector {
    /**
     * Identify Region of Interest (ROI) in a page.
     * @param {object} page - The Playwright page.
     * @returns {Promise<object|null>} ROI coordinates or null if full viewport.
     */
    async detect(page) {
        try {
            // Check for visible modals or dialogs
            const modalSelector = '[role="dialog"], .modal, [aria-modal="true"]';
            const modal = await page.$(modalSelector);

            if (modal) {
                const box = await modal.boundingBox();
                if (box) {
                    const padding = 20;
                    return {
                        x: Math.max(0, box.x - padding),
                        y: Math.max(0, box.y - padding),
                        width: box.width + padding * 2,
                        height: box.height + padding * 2,
                    };
                }
            }
            return null;
        } catch (_error) {
            return null;
        }
    }
}

export default RoIDetector;
