/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Browser Patch — Anti-detection and humanization scripts.
 * Internal copy for api/ independence from utils/browserPatch.js.
 *
 * @module api/utils/browserPatch
 */

/**
 * Applies anti-detect and humanization patches to the page context.
 * @param {import('playwright').Page} page
 * @param {{ info?: Function }} [logger]
 */
export async function applyHumanizationPatch(page, logger) {
    if (logger) logger.info('[HumanizationPatch] Injecting consistency and entropy scripts...');

    await page.addInitScript(() => {
        // 1. Consistency Check: Override Navigator Platform to match likely UA
        try {
            const ua = navigator.userAgent;
            let platform = 'Win32';
            if (ua.includes('Mac')) platform = 'MacIntel';
            else if (ua.includes('Linux')) platform = 'Linux x86_64';

            Object.defineProperty(navigator, 'platform', { get: () => platform });
        } catch (_e) {
            void _e;
        }

        try {
            Object.defineProperty(navigator, 'webdriver', { get: () => false, configurable: true });
        } catch (_e) {
            void _e;
        }

        // Removed media capability spoofing as it interferes with X.com player negotiations
        try {
            const originalToDataURL = HTMLCanvasElement.prototype.toDataURL;
            HTMLCanvasElement.prototype.toDataURL = function (...args) {
                // Skip poisoning on X.com/Twitter to avoid "privacy extension" detection
                if (
                    typeof window !== 'undefined' &&
                    window.location &&
                    (window.location.hostname.includes('x.com') ||
                        window.location.hostname.includes('twitter.com'))
                ) {
                    return originalToDataURL.apply(this, args);
                }

                if (this.width > 0 && this.height > 0) {
                    const ctx = this.getContext('2d');
                    if (ctx) {
                        const salt = Math.floor(Math.random() * 255);
                        const oldStyle = ctx.fillStyle;
                        ctx.fillStyle = `rgba(${salt}, ${salt}, ${salt}, 0.01)`;
                        ctx.fillRect(0, 0, 1, 1);
                        ctx.fillStyle = oldStyle;
                    }
                }
                return originalToDataURL.apply(this, args);
            };
        } catch (_e) {
            void _e;
        }

        // 5. Visibility Spoofing
        try {
            Object.defineProperty(document, 'hidden', { get: () => false, configurable: true });
            Object.defineProperty(document, 'visibilityState', {
                get: () => 'visible',
                configurable: true,
            });

            const originalAddEventListener = document.addEventListener;
            document.addEventListener = function (type, listener, options) {
                if (type === 'visibilitychange') {
                    void listener;
                }
                return originalAddEventListener.call(this, type, listener, options);
            };
        } catch (_e) {
            void _e;
        }
    });

    if (logger) logger.info('[HumanizationPatch] Scripts injected.');
}
