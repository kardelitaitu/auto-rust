/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview ROI Detector - Identifies regions of interest for vision tasks.
 * @module api/utils/roi-detector
 */

/**
 * Identify Region of Interest (ROI) in a page.
 * Detects modals, active form elements, and main content areas.
 * @param {import('playwright').Page} page - The Playwright page.
 * @returns {Promise<object|null>} ROI coordinates {x, y, width, height} or null.
 */
export async function identifyROI(page) {
    try {
        if (!page || page.isClosed()) return null;

        // ROI Selectors prioritized by "actionable" weight
        const selectors = [
            '[role="dialog"]',
            '[aria-modal="true"]',
            '.modal',
            'form',
            '[role="main"]',
            'main',
            '#main',
            '.main-content',
        ];

        for (const selector of selectors) {
            const element = await page.$(selector);
            if (element) {
                const box = await element.boundingBox();
                if (box && box.width > 50 && box.height > 50) {
                    // Add padding to ensure context around the element
                    const padding = 20;
                    const viewport = page.viewportSize() || { width: 1280, height: 720 };

                    return {
                        x: Math.max(0, box.x - padding),
                        y: Math.max(0, box.y - padding),
                        width: Math.min(viewport.width - box.x + padding, box.width + padding * 2),
                        height: Math.min(
                            viewport.height - box.y + padding,
                            box.height + padding * 2
                        ),
                    };
                }
            }
        }

        return null;
    } catch (_error) {
        // Fallback to null on error (caller will take full screenshot)
        return null;
    }
}

export default identifyROI;
