/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { api } from '../../index.js';
import { BaseHandler } from './BaseHandler.js';
import { scrollRandom } from '../../behaviors/scroll-helper.js';

export class NavigationHandler extends BaseHandler {
    constructor(agent) {
        super(agent);
    }

    /**
     * Navigate to home feed with multiple fallback strategies
     * - 10% chance to use direct URL navigation
     * - Primary: Home icon click with retry logic
     * - Secondary: X logo click
     * - Fallback: Direct URL navigation
     */
    async navigateHome() {
        this.log('Returning to Home Feed...');

        // 10% chance to use direct URL navigation (User Request)
        if (this.mathUtils.roll(0.1)) {
            this.log('[Navigation] 🎲 Random 10%: Using direct URL navigation.');
            try {
                await api.goto('https://x.com/home');
                await this.ensureForYouTab();
                return;
            } catch (e) {
                this.log(`[Navigation] Direct URL failed: ${e.message}. Falling back to click.`);
            }
        }

        try {
            const useHomeIcon = this.mathUtils.random() < 0.8;
            let targetSelector = useHomeIcon
                ? '[data-testid="AppTabBar_Home_Link"]'
                : '[aria-label="X"]';
            let targetName = useHomeIcon ? 'Home Icon' : 'X Logo';
            let target = this.page.locator(targetSelector).first();

            if (!(await api.visible(target).catch(() => false))) {
                this.log(`Preferred nav target (${targetName}) not visible. Switching...`);
                targetSelector = useHomeIcon
                    ? '[aria-label="X"]'
                    : '[data-testid="AppTabBar_Home_Link"]';
                targetName = useHomeIcon ? 'X Logo' : 'Home Icon';
                target = this.page.locator(targetSelector).first();
            }

            if (await api.visible(target)) {
                this.log(`[Navigation] Clicking '${targetName}' to navigate home...`);
                const clicked = await this.safeHumanClick(target, `${targetName} Click`, 2);

                if (clicked) {
                    await api.wait(this.mathUtils.randomInRange(800, 1500));

                    try {
                        await api.waitForURL('**/home**', { timeout: 5000 });
                        this.log('[Navigation] Navigate to /home Success.');
                        await this.ensureForYouTab();
                        return;
                    } catch {
                        this.log('[Navigation] safeHumanClick did not trigger navigation.');
                    }
                } else {
                    this.log(`[Navigation] safeHumanClick failed for '${targetName}'.`);
                }
            }
        } catch (e) {
            this.log(`[Navigation] Interaction failed: ${e.message}`);
        }

        this.log('[Navigation] Fallback to direct URL goto.');
        await api.goto('https://x.com/home');
        await this.ensureForYouTab();
    }

    /**
     * Ensure the "For you" tab is selected to avoid empty "Following" feeds
     * Uses multiple strategies: text matching, index fallback, and validation
     */
    async ensureForYouTab() {
        try {
            // Strictly enforce 'For you' tab to avoid empty 'Following' feeds
            const targetText = 'For you';

            // Wait for tablist to load (ensure page is ready)
            try {
                await this.page.waitForSelector('div[role="tablist"]', {
                    state: 'visible',
                    timeout: 5000,
                });
            } catch {
                this.log('[Tab] Tablist not found within timeout.');
                return;
            }

            const tablist = this.page.locator('div[role="tablist"]').first();

            // Get all tabs (generalized selector)
            const tabs = tablist.locator('[role="tab"]');
            const count = await tabs.count();
            let target = null;

            // 1. Try to find by text content
            for (let i = 0; i < count; i++) {
                const tab = tabs.nth(i);
                const text = await tab.textContent().catch(() => '');
                if (text && text.trim() === targetText) {
                    target = tab;
                    break;
                }
            }

            // 2. Fallback to index if text not found
            if (!target) {
                this.log(`[Tab] "${targetText}" text not found. Fallback to index.`);
                // Index 0 is "For you"
                const fallbackIndex = 0;
                if (count > fallbackIndex) {
                    target = tabs.nth(fallbackIndex);
                }
            }

            if (!target) {
                this.log(`[Tab] "${targetText}" tab could not be found via text or index.`);
                return;
            }

            // Check if already selected
            const isSelected = await target.getAttribute('aria-selected').catch(() => null);
            if (isSelected === 'true') {
                this.log(`[Tab] "${targetText}" is already selected.`);
                return;
            }

            if (await api.visible(target)) {
                this.log(`[Tab] Switching to "${targetText}" tab...`);
                try {
                    await this.safeHumanClick(target, 'For You Tab', 3);
                    await api.wait(this.mathUtils.randomInRange(800, 1500));
                } catch {
                    this.log('[Tab] Ghost click failed, trying native...');
                    await target.click();
                }
            }

            // After ensuring "For you" tab, check for "Show X posts" button
            // This button appears 2-3 seconds after returning home (per user feedback)
            await this.checkAndClickShowPostsButton();
        } catch (e) {
            this.log(`[Tab] Failed to ensure timeline tab: ${e.message}`);
        }
    }

    /**
     * Check for and click "Show X posts" button that appears when new posts are available
     * This button typically appears 2-3 seconds after returning home and ensuring "For you" tab
     * @returns {Promise<boolean>} true if button was found and clicked, false otherwise
     */
    async checkAndClickShowPostsButton() {
        try {
            // Wait 2-3 seconds for button to appear (as per user feedback)
            this.log('[Posts] Waiting for "Show X posts" button to appear...');
            await api.wait(this.mathUtils.randomInRange(2000, 3000));

            // Multiple selector patterns to catch variations:
            // - "Show 34 posts"
            // - "Show 5 posts"
            // - "Show X new posts"
            const buttonSelectors = [
                // Primary: button with role containing span with "Show" and "posts"
                '[role="button"]:has(span:has-text(/Show\\s+\\d+\\s+posts/i))',
                // Alternative: button containing text "Show" and "posts"
                'button:has-text(/Show\\s+\\d+\\s+posts/i)',
                // Broader match: any element with role button containing "Show" and number
                '[role="button"]:has-text("Show"):has-text(/\\d+/)]',
            ];

            let showPostsBtn = null;
            let btnText = '';

            // Try each selector
            for (const selector of buttonSelectors) {
                try {
                    const btn = this.page.locator(selector).first();
                    if ((await api.exists(btn)) && (await api.visible(btn))) {
                        const text = await btn.textContent().catch(() => '');
                        // Validate it matches the pattern
                        if (/show\s+\d+\s+post/i.test(text)) {
                            showPostsBtn = btn;
                            btnText = text.trim();
                            break;
                        }
                    }
                } catch {
                    // Continue to next selector
                }
            }

            if (showPostsBtn) {
                this.log(`[Posts] Found "${btnText}" button, clicking to load new posts...`);

                // HUMAN-LIKE: Additional pre-click behavior for this specific button
                // 1. Ensure button is in viewport (scroll if needed)
                await showPostsBtn.evaluate((el) =>
                    el.scrollIntoView({ block: 'center', inline: 'center' })
                );
                await api.wait(this.mathUtils.randomInRange(300, 600));

                // 2. Move cursor to vicinity first (not directly on button)
                const box = await showPostsBtn.boundingBox();
                if (box) {
                    const offsetX = this.mathUtils.randomInRange(-30, 30);
                    const offsetY = this.mathUtils.randomInRange(-20, 20);
                    await this.ghost.move(
                        box.x + box.width / 2 + offsetX,
                        box.y + box.height / 2 + offsetY,
                        this.mathUtils.randomInRange(15, 25)
                    );
                    await api.wait(this.mathUtils.randomInRange(400, 800));
                }

                // 3. Use safeHumanClick for full simulated interaction with retry
                await this.safeHumanClick(showPostsBtn, 'Show Posts Button', 3);

                // 3. HUMAN-LIKE: "Reading" the new posts after loading
                this.log('[Posts] Posts loading, simulating reading behavior...');
                const waitTime = this.mathUtils.randomInRange(1200, 2500);
                await api.wait(waitTime);

                // 4. Scroll down slightly to show the new posts (human-like discovery)
                const scrollVariance = 0.8 + Math.random() * 0.4;
                await scrollRandom(
                    Math.floor(150 * scrollVariance),
                    Math.floor(250 * scrollVariance)
                );
                await api.wait(this.mathUtils.randomInRange(600, 1000));

                this.log('[Posts] New posts loaded successfully.');
                return true;
            } else {
                this.log('[Posts] No "Show X posts" button found.');
                return false;
            }
        } catch (error) {
            this.log(`[Posts] Error checking for show posts button: ${error.message}`);
            return false;
        }
    }
}
