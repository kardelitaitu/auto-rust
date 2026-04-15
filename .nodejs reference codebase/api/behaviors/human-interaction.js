/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Human Interaction Utilities
 * Provides human-like behavior patterns for automation
 * @module utils/human-interaction
 */

import { createLogger } from '../core/logger.js';
import { mathUtils } from '../utils/math.js';
import { entropy as _entropy } from '../utils/entropyController.js';
import { GhostCursor } from '../utils/ghostCursor.js';
import { api } from '../index.js';

const logger = createLogger('human-interaction.js');

export class HumanInteraction {
    constructor(page = null) {
        this.debugMode = process.env.HUMAN_DEBUG === 'true';
        this.page = page;
        this.ghost = null;
        if (page) {
            this.ghost = new GhostCursor(page);
        }
    }

    /**
     * Set page reference and initialize GhostCursor
     * Call this before using human-like clicking methods
     */
    setPage(page) {
        this.page = page;
        this.ghost = new GhostCursor(page);
    }

    /**
     * Human-like click using GhostCursor simulated mouse
     * Includes: scroll into view, fixation delay, micro-movements, and ghost click
     */
    async humanClick(element, description = 'Target') {
        if (!this.page || !this.ghost) {
            this.logWarn(`[humanClick] No page/ghost initialized`);
            throw new Error('ghost_not_initialized');
        }

        this.logDebug(`[humanClick] Starting human-like click on ${description}`);

        try {
            await element.evaluate((el) =>
                el.scrollIntoView({ block: 'center', inline: 'center' })
            );
            await api.wait(mathUtils.randomInRange(300, 600));
            const ghostResult = await this.ghost.click(element, {
                label: description,
                hoverBeforeClick: true,
            });
            if (!ghostResult?.success) {
                throw new Error('ghost_click_failed');
            }
            this.logDebug(`[humanClick] Successfully clicked ${description}`);
        } catch (e) {
            this.logDebug(`[humanClick] Failed on ${description}: ${e.message}`);
            throw e;
        }
    }

    /**
     * Safe human-like click with retry logic
     * Wraps humanClick with automatic retry on failure
     * @param {Object} element - Playwright locator or element handle
     * @param {string} description - Description for logging
     * @param {number} retries - Number of retry attempts (default: 3)
     * @returns {Promise<boolean>} - True if successful, false if all retries failed
     */
    async safeHumanClick(element, description = 'Target', retries = 3) {
        const attemptLogs = [];
        for (let attempt = 1; attempt <= retries; attempt++) {
            try {
                await this.humanClick(element, description);
                this.logDebug(
                    `[safeHumanClick] [${description}] Success on attempt ${attempt}/${retries}`
                );
                return true;
            } catch (error) {
                attemptLogs.push(`Attempt ${attempt}: ${error.message}`);
                this.logWarn(
                    `[safeHumanClick] [${description}] Attempt ${attempt}/${retries} failed: ${error.message}`
                );
                if (attempt === retries) {
                    this.logWarn(
                        `[safeHumanClick] [${description}] All retries exhausted. Errors: ${attemptLogs.join('; ')}`
                    );
                    return false;
                }
                // Exponential backoff: 1s, 2s, 3s...
                const delay = 1000 * attempt;
                this.logDebug(
                    `[safeHumanClick] [${description}] Waiting ${delay}ms before retry ${attempt + 1}...`
                );
                await api.wait(delay);
            }
        }
        return false;
    }

    /**
     * Get random method based on weights
     */
    selectMethod(methods) {
        const roll = Math.random() * 100;
        let cumulative = 0;

        for (const method of methods) {
            cumulative += method.weight;
            if (roll <= cumulative) {
                this.logDebug(
                    `[MethodSelect] Selected: ${method.name} (roll: ${roll.toFixed(1)}, threshold: ${cumulative.toFixed(1)})`
                );
                return method;
            }
        }

        return methods[0];
    }

    /**
     * Human-like pre-action delay
     */
    async hesitation(min = null, max = null) {
        const persona = api.getPersona();
        const baseDelay =
            typeof persona.hesitationDelay === 'number' ? persona.hesitationDelay : 400;
        const hesitation = typeof persona.hesitation === 'number' ? persona.hesitation : 0.15;
        const useDefaultRange = typeof min !== 'number' && typeof max !== 'number';
        const minDelay = typeof min === 'number' ? min : Math.max(300, Math.round(baseDelay * 0.5));
        let maxDelay =
            typeof max === 'number'
                ? max
                : Math.round(baseDelay * (1 + Math.max(0.1, hesitation) * 3));
        if (useDefaultRange) {
            maxDelay = Math.min(1500, Math.max(minDelay + 50, maxDelay));
        } else {
            maxDelay = Math.max(minDelay + 50, maxDelay);
        }
        const delay = mathUtils.randomInRange(minDelay, maxDelay);
        this.logDebug(`[Hesitation] Waiting ${delay}ms before action...`);
        await api.wait(delay);
        return delay;
    }

    /**
     * Human-like reading delay
     */
    async readingTime(min = null, max = null) {
        const persona = api.getPersona();
        const minDelay =
            typeof min === 'number'
                ? min
                : typeof persona.hoverMin === 'number'
                  ? persona.hoverMin * 2
                  : 5000;
        const maxDelay =
            typeof max === 'number'
                ? max
                : typeof persona.hoverMax === 'number'
                  ? persona.hoverMax * 3
                  : 15000;
        const time = mathUtils.randomInRange(minDelay, Math.max(minDelay + 100, maxDelay));
        this.logDebug(`[Reading] Reading for ${time}ms...`);
        await api.wait(time);
        return time;
    }

    /**
     * Random scroll during reading (30% probability)
     */
    async maybeScroll(page, min = 100, max = 300) {
        if (Math.random() < 0.3) {
            const scrollAmount = mathUtils.randomInRange(min, max);
            const direction = Math.random() < 0.5 ? scrollAmount : -scrollAmount;
            this.logDebug(`[Scroll] Random scroll: ${direction}px`);
            await api.scroll(direction);
            return true;
        }
        return false;
    }

    /**
     * Micro mouse movement
     */
    async microMove(page, range = 20) {
        const x = mathUtils.randomInRange(-range, range);
        const y = mathUtils.randomInRange(-range, range);
        this.logDebug(`[MicroMove] Mouse: ${x}, ${y}`);
        await page.mouse.move(x, y);
    }

    /**
     * Target fixation (pause before clicking)
     */
    async fixation(min = null, max = null) {
        const persona = api.getPersona();
        const minDelay =
            typeof min === 'number'
                ? min
                : typeof persona.hoverMin === 'number'
                  ? persona.hoverMin
                  : 200;
        const maxDelay =
            typeof max === 'number'
                ? max
                : typeof persona.hoverMax === 'number'
                  ? persona.hoverMax
                  : 800;
        const time = mathUtils.randomInRange(minDelay, Math.max(minDelay + 50, maxDelay));
        this.logDebug(`[Fixation] Fixating for ${time}ms...`);
        await api.wait(time);
        return time;
    }

    /**
     * Find element with multiple selectors
     * @returns {Promise<any>}
     */
    async findElement(page, selectors, options = {}) {
        const { visibleOnly = true, timeout = 5000 } = options;
        const startTime = Date.now();

        for (const selector of selectors) {
            const elapsed = Date.now() - startTime;
            if (elapsed > timeout) {
                this.logDebug(`[FindElement] Timeout reached`);
                break;
            }

            try {
                const elements = await page.locator(selector).all();
                this.logDebug(`[FindElement] Selector "${selector}": ${elements.length} elements`);

                for (let i = 0; i < elements.length; i++) {
                    try {
                        const el = elements[i];
                        if (visibleOnly) {
                            if (await api.visible(el)) {
                                this.logDebug(`[FindElement] Found visible element at index ${i}`);
                                return { element: el, selector, index: i };
                            }
                        } else {
                            return { element: el, selector, index: i };
                        }
                    } catch (e) {
                        this.logDebug(`[FindElement] Element ${i} error: ${e.message}`);
                    }
                }
            } catch (e) {
                this.logDebug(`[FindElement] Selector "${selector}" error: ${e.message}`);
            }
        }

        return { element: null, selector: null, index: -1 };
    }

    /**
     * Verify composer is open and get the best textarea locator
     */
    async verifyComposerOpen(page) {
        const composerSelectors = [
            '[data-testid="tweetTextarea_0"]',
            '[contenteditable="true"][role="textbox"]',
            '[data-testid="tweetTextarea"]',
            '[class*="composer"] textarea',
            'textarea[placeholder*="Post your reply"]',
            'textarea[placeholder*="What\'s happening"]',
            '[role="textbox"][contenteditable="true"]',
        ];

        this.logDebug(`[Verify] Checking if composer is open...`);

        // First, try to find the most reliable selector
        for (const selector of composerSelectors) {
            try {
                const el = page.locator(selector).first();
                if (await api.exists(el)) {
                    const isVisible = await api.visible(el);
                    const box = await el.boundingBox();

                    this.logDebug(
                        `[Verify] Selector "${selector}": visible=${isVisible}, box=${box ? 'found' : 'none'}`
                    );

                    if (isVisible && box && box.width > 50 && box.height > 20) {
                        this.logDebug(`[Verify] Composer open with: ${selector}`);

                        // Double check it's not a stale element
                        const textContent = await el.inputValue().catch(() => '');
                        this.logDebug(
                            `[Verify] Element has value: ${textContent.length > 0 ? 'yes' : 'empty'}`
                        );

                        return { open: true, selector, locator: el };
                    }
                }
            } catch (e) {
                this.logDebug(`[Verify] Selector "${selector}" error: ${e.message}`);
            }
        }

        // Try one more time with longer wait
        await api.wait(500);

        for (const selector of composerSelectors) {
            try {
                const el = page.locator(selector).first();
                if ((await api.exists(el)) && (await api.visible(el))) {
                    this.logDebug(`[Verify] Late detection: ${selector}`);
                    return { open: true, selector, locator: el };
                }
            } catch {
                // Ignore error, continue to typing
            }
        }

        this.logDebug(`[Verify] Composer not open`);
        return { open: false, selector: null, locator: null };
    }

    /**
     * Verify post was sent (composer closed o
    /**
     * Verify post was sent (composer closed or confirmation shown)
     */
    async verifyPostSent(page) {
        // Wait a moment for UI update
        await api.wait(500);

        const checks = [
            // Positive indicators (Success)
            { selector: '[data-testid="toast"]', label: 'toast notification', type: 'positive' },
            {
                selector: 'span:has-text("Your post was sent")',
                label: 'sent text',
                type: 'positive',
            },
            {
                selector: 'span:has-text("Your Tweet was sent")',
                label: 'sent text (old)',
                type: 'positive',
            },

            // Negative indicators (Success if GONE)
            { selector: '[data-testid="tweetTextarea_0"]', label: 'composer', type: 'negative' },
            { selector: '[data-testid="tweetButton"]', label: 'post button', type: 'negative' },
            {
                selector: '[data-testid="tweetButtonInline"]',
                label: 'inline post button',
                type: 'negative',
            },
        ];

        this.logDebug(`[Verify] Checking if post was sent...`);

        // Check positive indicators first
        for (const check of checks.filter((c) => c.type === 'positive')) {
            try {
                const el = page.locator(check.selector).first();
                if (await api.visible(el).catch(() => false)) {
                    const text = await el.innerText().catch(() => '');
                    this.logDebug(`[Verify] Found ${check.label}: "${text.substring(0, 30)}"`);

                    // Verify it's not an error toast
                    if (check.label === 'toast notification') {
                        const lowerText = text.toLowerCase();
                        if (
                            lowerText.includes('fail') ||
                            lowerText.includes('error') ||
                            lowerText.includes('wrong') ||
                            lowerText.includes('retry')
                        ) {
                            this.logWarn(`[Verify] Toast indicates failure: "${text}"`);
                            continue;
                        }
                    }

                    return { sent: true, method: check.label };
                }
            } catch {
                /* ignore */
            }
        }

        // Check negative indicators (must be GONE)
        const composer = page.locator('[data-testid="tweetTextarea_0"]');
        const isComposerVisible = await api.visible(composer).catch(() => false);
        this.logDebug(`[Verify] Composer visible: ${isComposerVisible}`);

        if (!isComposerVisible) {
            // Double check it's not just a loading glitch
            await api.wait(500);
            if (!(await api.visible(composer).catch(() => false))) {
                this.logDebug(`[Verify] Composer is no longer visible (confirmed)`);
                return { sent: true, method: 'composer_closed' };
            }
        }

        // Check URL changed back
        const url = api.getCurrentUrl();
        this.logDebug(`[Verify] Current URL: ${url}`);

        if (!url.includes('/compose/') && !url.includes('/status/')) {
            this.logDebug(`[Verify] URL check passed (not in compose mode)`);
        }

        // Additional verification: wait a bit and check again if not confirmed
        this.logDebug(`[Verify] Post not immediately confirmed, waiting 1.5s...`);
        await api.wait(1500);

        // Final check: composer should be closed
        const composerVisibleFinal = await api
            .visible(page.locator('[data-testid="tweetTextarea_0"]'))
            .catch(() => false);
        if (!composerVisibleFinal) {
            this.logDebug(`[Verify] Composer closed after wait: confirmed`);
            return { sent: true, method: 'composer_closed_delayed' };
        }

        // If composer is still there, check if text was cleared
        const inputValue = await composer.inputValue().catch(() => '');
        if (inputValue.length === 0) {
            this.logDebug(`[Verify] Composer visible but empty. Treating as success.`);
            return { sent: true, method: 'composer_cleared' };
        }

        this.logDebug(
            `[Verify] Composer still visible with content (${inputValue.length} chars). Post failed.`
        );
        return { sent: false, method: null };
    }

    /**
     * Verify reply was sent specifically
     */
    async twitterVerifyReply(page) {
        // PRIORITY 1: Explicitly wait for the small popup notification at the bottom
        this.logDebug(`[Verify] Prioritizing popup notification for reply verification...`);
        try {
            // Wait up to 3.5 seconds for the toast/popup to appear
            const toastSelector =
                '[data-testid="toast"], span:has-text("Your post was sent"), span:has-text("Your reply was sent")';
            const toast = await page.waitForSelector(toastSelector, {
                state: 'visible',
                timeout: 3500,
            });
            if (toast) {
                const text = await toast.innerText().catch(() => '');
                this.logDebug(`[Verify] Popup found: "${text.substring(0, 30)}"`);
                const lowerText = text.toLowerCase();
                if (
                    lowerText.includes('fail') ||
                    lowerText.includes('error') ||
                    lowerText.includes('wrong') ||
                    lowerText.includes('retry')
                ) {
                    this.logWarn(`[Verify] Popup indicates failure: "${text}"`);
                } else {
                    return { sent: true, method: 'popup_toast' };
                }
            }
        } catch (_e) {
            this.logDebug(
                `[Verify] Popup notification not found within timeout. Falling back to composer state check...`
            );
        }

        // Check negative indicators (Success if GONE)
        const composer = page.locator('[data-testid="tweetTextarea_0"]');
        const isComposerVisible = await api.visible(composer).catch(() => false);
        this.logDebug(`[Verify] Reply Composer visible: ${isComposerVisible}`);

        if (!isComposerVisible) {
            // Double check it's not just a loading glitch
            await api.wait(500);
            if (!(await api.visible(composer).catch(() => false))) {
                this.logDebug(`[Verify] Reply Composer is no longer visible (confirmed)`);
                return { sent: true, method: 'composer_closed' };
            }
        }

        // Additional verification: wait a bit and check again if not confirmed
        this.logDebug(`[Verify] Reply not immediately confirmed, waiting 1.5s...`);
        await api.wait(1500);

        // Final check: composer should be closed
        const composerVisibleFinal = await api
            .visible(page.locator('[data-testid="tweetTextarea_0"]'))
            .catch(() => false);
        if (!composerVisibleFinal) {
            this.logDebug(`[Verify] Reply Composer closed after wait: confirmed`);
            return { sent: true, method: 'composer_closed_delayed' };
        }

        const inputValue = await composer.inputValue().catch(() => '');
        if (inputValue.length === 0) {
            this.logDebug(`[Verify] Reply Composer visible but empty. Treating as success.`);
            return { sent: true, method: 'composer_cleared' };
        }

        this.logDebug(`[Verify] Reply Composer still visible with content. Reply failed.`);
        return { sent: false, method: null };
    }

    /**
     * Verify quote was sent specifically
     */
    async twitterVerifyQuote(page) {
        // PRIORITY 1: Explicitly wait for the small popup notification at the bottom
        this.logDebug(`[Verify] Prioritizing popup notification for quote verification...`);
        try {
            // Wait up to 5 seconds for the toast/popup to appear
            const toastSelector = '[data-testid="toast"], span:has-text("Your post was sent")';
            const toast = await page.waitForSelector(toastSelector, {
                state: 'visible',
                timeout: 5000,
            });
            if (toast) {
                const text = await toast.innerText().catch(() => '');
                this.logDebug(`[Verify] Popup found: "${text.substring(0, 30)}"`);
                const lowerText = text.toLowerCase();
                if (
                    lowerText.includes('fail') ||
                    lowerText.includes('error') ||
                    lowerText.includes('wrong') ||
                    lowerText.includes('retry')
                ) {
                    this.logWarn(`[Verify] Popup indicates failure: "${text}"`);
                } else {
                    return { sent: true, method: 'popup_toast' };
                }
            }
        } catch (_e) {
            this.logDebug(
                `[Verify] Popup notification not found within timeout. Falling back to composer state check...`
            );
        }

        // Check negative indicators (Success if GONE)
        const composer = page.locator('[data-testid="tweetTextarea_0"]');
        const isComposerVisible = await api.visible(composer).catch(() => false);
        this.logDebug(`[Verify] Quote Composer visible: ${isComposerVisible}`);

        if (!isComposerVisible) {
            // Double check it's not just a loading glitch
            await api.wait(500);
            if (!(await api.visible(composer).catch(() => false))) {
                this.logDebug(`[Verify] Quote Composer is no longer visible (confirmed)`);
                return { sent: true, method: 'composer_closed' };
            }
        }

        // Additional verification: wait a bit and check again if not confirmed
        this.logDebug(`[Verify] Quote not immediately confirmed, waiting 1.5s...`);
        await api.wait(1500);

        // Final check: composer should be closed
        const composerVisibleFinal = await api
            .visible(page.locator('[data-testid="tweetTextarea_0"]'))
            .catch(() => false);
        if (!composerVisibleFinal) {
            this.logDebug(`[Verify] Quote Composer closed after wait: confirmed`);
            return { sent: true, method: 'composer_closed_delayed' };
        }

        const inputValue = await composer.inputValue().catch(() => '');
        if (inputValue.length === 0) {
            this.logDebug(`[Verify] Quote Composer visible but empty. Treating as success.`);
            return { sent: true, method: 'composer_cleared' };
        }

        this.logDebug(`[Verify] Quote Composer still visible with content. Quote failed.`);
        return { sent: false, method: null };
    }

    /**
     * Post with Ctrl+Enter or fallback
     */
    async typeText(page, text, inputEl, options = {}) {
        const { skipClear = false, skipFocusClick = false } = options;
        // Ensure we have page reference for ghost cursor
        if (!this.page || this.page !== page) {
            this.setPage(page);
        }

        // Step 1: Clear any existing text first using human-like click (if not skipped)
        if (!skipClear) {
            try {
                await this.humanClick(inputEl, 'Text Input - Clear');
                await page.keyboard.press('Control+a');
                await api.wait(200);
            } catch (e) {
                this.logDebug(`[Type] Clear text failed: ${e.message}`);
            }
        }

        // Step 2: Multiple focus strategies
        const focused = await this.ensureFocus(page, inputEl, { skipClick: skipFocusClick });
        if (!focused) {
            this.logWarn(`[Type] Could not focus element, trying alternative approach...`);
            // Fallback: use keyboard to focus
            await page.keyboard.press('Tab');
            await api.wait(300);
        }

        // Step 3: Verify we're focused by checking active element
        const activeCheck = await page.evaluate(() => {
            const el = document.activeElement;
            return {
                tagName: el?.tagName,
                isContentEditable: el?.getAttribute('contenteditable') === 'true',
                hasFocus:
                    el === document.querySelector('[data-testid="tweetTextarea_0"]') ||
                    el === document.querySelector('[contenteditable="true"]'),
            };
        });

        this.logDebug(
            `[Type] Active element: ${activeCheck.tagName}, contentEditable: ${activeCheck.isContentEditable}`
        );

        // If still not focused, try force clicking as last resort
        if (
            !activeCheck.isContentEditable &&
            !activeCheck.tagName?.toLowerCase().includes('textarea')
        ) {
            this.logDebug(`[Type] Not focused correctly, trying force click fallback...`);
            try {
                await inputEl.click({ force: true });
                await api.wait(200);
            } catch (_e) {
                // Ignore error
            }
        }

        // Human-like typing with variations
        const baseDelay = mathUtils.randomInRange(80, 150);
        const punctuationPause = mathUtils.randomInRange(200, 400);
        const spacePause = mathUtils.randomInRange(100, 200);

        // Pre-create Set for O(1) lookup instead of O(n) Array.includes()
        const punctuationSet = new Set(['.', '!', '?', ',', ';', ':']);

        this.logDebug(`[Type] Starting to type ${text.length} chars...`);

        for (let i = 0; i < text.length; i++) {
            const char = text[i];

            if (char === ' ') {
                await api.wait(spacePause);
            } else if (punctuationSet.has(char)) {
                await api.wait(punctuationPause);
            } else {
                await api.wait(baseDelay);
            }

            // Occasional longer pause (thinking)
            if (Math.random() < 0.05 && i < text.length - 1) {
                const pause = mathUtils.randomInRange(300, 800);
                //this.logDebug(`[Type] Thinking pause: ${pause}ms`); // disabled to reduce log spam
                await api.wait(pause);
            }

            await page.keyboard.type(char);
        }

        const duration = mathUtils.randomInRange(500, 1000);
        this.logDebug(`[Type] Finished typing`);
        await api.wait(duration);
    }

    /**
     * Ensure element is focused with multiple strategies
     * Uses human-like clicking with GhostCursor
     */
    async ensureFocus(page, element, options = {}) {
        const { skipClick = false } = options;
        // Ensure we have page reference
        if (!this.page || this.page !== page) {
            this.setPage(page);
        }

        const focusStrategies = [];

        // Strategy 1: Human-like click with GhostCursor (if not skipped)
        if (!skipClick) {
            focusStrategies.push(async () => {
                try {
                    await this.humanClick(element, 'Focus Target');
                    await api.wait(200);
                    return true;
                } catch {
                    return false;
                }
            });
        }

        // Strategy 2: Focus method (no click needed)
        focusStrategies.push(async () => {
            try {
                await element.focus();
                await api.wait(200);
                return true;
            } catch {
                return false;
            }
        });

        for (let i = 0; i < focusStrategies.length; i++) {
            try {
                const result = await focusStrategies[i]();
                if (result) {
                    // Verify focus worked
                    const isFocused = await page.evaluate(() => {
                        const el = document.activeElement;
                        return (
                            el?.getAttribute('contenteditable') === 'true' ||
                            el?.tagName === 'TEXTAREA' ||
                            el?.tagName === 'INPUT'
                        );
                    });

                    if (isFocused) {
                        this.logDebug(`[EnsureFocus] Strategy ${i + 1} succeeded`);
                        return true;
                    }
                }
            } catch (e) {
                this.logDebug(`[EnsureFocus] Strategy ${i + 1} failed: ${e.message}`);
            }
        }

        this.logDebug(`[EnsureFocus] All strategies failed`);
        return false;
    }

    /**
     * Post by ghost-clicking the post/reply button
     */
    async postTweet(page, type = 'tweet') {
        this.logDebug(`[Post] Attempting to post (${type})...`);

        const verifyPost = async () => {
            if (type === 'reply') return await this.twitterVerifyReply(page);
            if (type === 'quote') return await this.twitterVerifyQuote(page);
            return await this.verifyPostSent(page);
        };

        // STEP 1: Find "Post" or "Reply" button
        // Priority 1: Search within the active composer or near the focused element
        let targetBtn = null;
        let targetSelector = null;

        const postSelectors = [
            '[data-testid="tweetButton"]',
            '[data-testid="tweetButtonInline"]',
            '[data-testid="sendTweets"]',
            '[aria-label="Post"]',
            '[aria-label="Reply"]',
            '[role="button"][data-testid*="tweetButton"]',
            'button[type="submit"]',
        ];

        try {
            const focused = page.locator(':focus');
            if (await api.exists(focused)) {
                // Try to find button in the same container (modal, inline box, etc)
                // We search for common composer containers that house both the textarea and the button
                const containerSelectors = [
                    '[role="dialog"]',
                    '[data-testid="inlineComposer"]',
                    '.DraftEditor-root',
                    '[data-testid="tweetTextarea_0"]',
                ];

                for (const contSelector of containerSelectors) {
                    const container = page.locator(contSelector).first();
                    if (await api.exists(container)) {
                        // Check if the button is within this container or its parent
                        const parent =
                            contSelector === '[data-testid="tweetTextarea_0"]'
                                ? page
                                      .locator('[role="dialog"], [data-testid="inlineComposer"]')
                                      .first()
                                : container;

                        if (await api.exists(parent)) {
                            for (const selector of postSelectors) {
                                const btn = parent.locator(selector).first();
                                if ((await api.exists(btn)) && (await api.visible(btn))) {
                                    targetBtn = btn;
                                    targetSelector = `container:${selector}`;
                                    this.logDebug(
                                        `[Post] Found button in container ${contSelector}: ${selector}`
                                    );
                                    break;
                                }
                            }
                        }
                    }
                    if (targetBtn) break;
                }
            }
        } catch (e) {
            this.logDebug(`[Post] Context-aware search failed: ${e.message}`);
        }

        // Fallback: Global search (original behavior)
        if (!targetBtn) {
            for (const selector of postSelectors) {
                try {
                    const btn = page.locator(selector).first();
                    if ((await api.exists(btn)) && (await api.visible(btn))) {
                        targetBtn = btn;
                        targetSelector = selector;
                        this.logDebug(`[Post] Found button with global selector: ${selector}`);
                        break;
                    }
                } catch (e) {
                    this.logDebug(`[Post] Error checking selector "${selector}": ${e.message}`);
                }
            }
        }

        if (!targetBtn) {
            this.logWarn(`[Post] No post button found!`);
            return { success: false, reason: 'button_not_found' };
        }

        // Handle disabled button (wait for it to enable)
        let isDisabled = await targetBtn.evaluate(
            (e) => e.disabled || e.getAttribute('aria-disabled') === 'true'
        );
        if (isDisabled) {
            this.logDebug(`[Post] Button is disabled, waiting for it to enable...`);

            // Try to trigger input event on focused element to wake up React
            try {
                const activeEl = page.locator(':focus');
                if (await api.exists(activeEl)) {
                    this.logDebug(`[Post] Triggering input event on focused element...`);
                    await page.keyboard.type(' ');
                    await page.keyboard.press('Backspace');
                    await api.wait(500);
                }
            } catch (e) {
                this.logDebug(`[Post] Failed to trigger input: ${e.message}`);
            }

            // Wait up to 3 seconds for button to enable
            const startTime = Date.now();
            while (isDisabled && Date.now() - startTime < 3000) {
                await api.wait(500);
                isDisabled = await targetBtn.evaluate(
                    (e) => e.disabled || e.getAttribute('aria-disabled') === 'true'
                );
            }

            if (isDisabled) {
                this.logWarn(`[Post] Button still disabled after wait. Attempting click anyway...`);
            } else {
                this.logDebug(`[Post] Button became enabled!`);
            }
        }

        // STEP 2: Attempt submit — JS click first (keeps textarea focused, avoids React blur)
        this.logDebug(`[Post] Submitting via JS click (no focus loss): ${targetSelector}`);
        try {
            await targetBtn.evaluate((el) => el.click());
        } catch (e) {
            this.logDebug(`[Post] JS click failed: ${e.message}`);
        }

        // Wait for result
        await api.wait(2000);
        const result2 = await verifyPost();

        if (result2.sent) {
            this.logDebug(`[Post] ✅ Success via JS click`);
            return { success: true, method: 'js_click' };
        }

        // Fallback: ghost cursor click (original behavior)
        this.logDebug(
            `[Post] JS click not confirmed, falling back to ghost cursor: ${targetSelector}`
        );
        try {
            await this.humanClick(targetBtn, 'Post Button', { precision: 'high' });
        } catch (e) {
            this.logWarn(`[Post] humanClick failed: ${e.message}`);
        }

        await api.wait(2000);
        const result3 = await verifyPost();

        if (result3.sent) {
            this.logDebug(`[Post] ✅ Success via ghost click`);
            return { success: true, method: 'ghost_click' };
        }

        // Final fallback: force click
        this.logWarn(`[Post] Ghost click not confirmed. Trying force click...`);
        try {
            await targetBtn.click({ force: true });
            await api.wait(2000);
            const result4 = await verifyPost();
            if (result4.sent) {
                this.logDebug(`[Post] ✅ Success via force click`);
                return { success: true, method: 'force_click' };
            }
        } catch (e) {
            this.logDebug(`[Post] Force click failed: ${e.message}`);
        }

        this.logWarn(`[Post] Failed - no method worked`);
        return { success: false, reason: 'post_failed' };
    }

    // =========================================================================
    // SELECTOR FALLBACK METHODS - Reduces element not found errors
    // =========================================================================

    /**
     * Find element with fallback selectors chain
     * Reduces element not found errors by trying multiple selectors
     * @param {string[]} selectors - Array of selectors to try (ordered by priority)
     * @param {object} options - Options for visibility check
     * @returns {Promise<object|null>} - Found element info or null
     */
    async findWithFallback(selectors, options = {}) {
        const { visible = true, timeout = 5000, logLevel: _logLevel = 'debug' } = options;

        const startTime = Date.now();

        for (let i = 0; i < selectors.length; i++) {
            const selector = selectors[i];
            const elapsed = Date.now() - startTime;

            if (elapsed >= timeout) {
                this.logWarn(`[Fallback] Timeout reached after ${elapsed}ms, stopping search`);
                break;
            }

            try {
                const element = this.page.locator(selector).first();

                // Check if element exists
                const count = await element.count();
                if (count === 0) {
                    this.logDebug(
                        `[Fallback] Selector ${i + 1}/${selectors.length} not found: ${selector}`
                    );
                    continue;
                }

                // Check visibility if required
                if (visible) {
                    const isVisible = await api.visible(element).catch(() => false);
                    if (!isVisible) {
                        this.logDebug(
                            `[Fallback] Selector ${i + 1}/${selectors.length} not visible: ${selector}`
                        );
                        continue;
                    }
                }

                this.logDebug(
                    `[Fallback] Found element with selector ${i + 1}/${selectors.length}: ${selector}`
                );
                return { element, selector, index: i };
            } catch (error) {
                this.logDebug(
                    `[Fallback] Error with selector ${i + 1}/${selectors.length} (${selector}): ${error.message}`
                );
                continue;
            }
        }

        this.logWarn(`[Fallback] All ${selectors.length} selectors failed`);
        return null;
    }

    /**
     * Find multiple elements with fallback selectors
     * @param {string[]} selectors - Array of selectors to try
     * @param {object} options - Options
     * @returns {Promise<object[]>} - Array of element locators
     */
    async findAllWithFallback(selectors, options = {}) {
        const { visible = true, limit = 20 } = options;

        for (const selector of selectors) {
            try {
                const elements = this.page.locator(selector);
                const count = await elements.count();

                if (count > 0) {
                    const results = [];
                    const actualLimit = Math.min(count, limit);

                    for (let i = 0; i < actualLimit; i++) {
                        const el = elements.nth(i);

                        if (visible) {
                            if (await api.visible(el).catch(() => false)) {
                                results.push(el);
                            }
                        } else {
                            results.push(el);
                        }
                    }

                    if (results.length > 0) {
                        this.logDebug(
                            `[Fallback] Found ${results.length} visible elements with: ${selector}`
                        );
                        return results;
                    }
                }
            } catch (error) {
                this.logDebug(`[Fallback] Error with selector (${selector}): ${error.message}`);
                continue;
            }
        }

        return [];
    }

    /**
     * Click element with automatic fallback selector chain
     * Tries multiple selectors if primary fails
     * @param {string[]} selectors - Array of selectors to try
     * @param {string} description - Description for logging
     * @param {object} options - Click options
     * @returns {Promise<boolean>} - True if successful
     */
    async clickWithFallback(selectors, description = 'Element', _options = {}) {
        for (let i = 0; i < selectors.length; i++) {
            const selector = selectors[i];

            try {
                const element = this.page.locator(selector).first();
                const count = await element.count();

                if (count === 0) {
                    this.logDebug(
                        `[ClickFallback] Selector ${i + 1}/${selectors.length} not found: ${selector}`
                    );
                    continue;
                }

                if (!(await api.visible(element).catch(() => false))) {
                    this.logDebug(
                        `[ClickFallback] Selector ${i + 1}/${selectors.length} not visible: ${selector}`
                    );
                    continue;
                }

                this.logDebug(`[ClickFallback] Clicking ${description} with: ${selector}`);
                await this.humanClick(element, description);
                return true;
            } catch (error) {
                this.logDebug(
                    `[ClickFallback] Error with selector ${i + 1}/${selectors.length} (${selector}): ${error.message}`
                );
                continue;
            }
        }

        this.logWarn(`[ClickFallback] All selectors failed for: ${description}`);
        return false;
    }

    /**
     * Wait for element with multiple selector fallbacks
     * @param {string[]} selectors - Array of selectors
     * @param {object} options - Wait options
     * @returns {Promise<object|null>} - Element info or null
     */
    async waitForWithFallback(selectors, options = {}) {
        const { visible = true, timeout = 5000, state = 'visible' } = options;

        for (const selector of selectors) {
            try {
                const element = this.page.locator(selector);

                await element.first().waitFor({ state, timeout: Math.min(timeout, 10000) });

                if (visible) {
                    if (await api.visible(element.first()).catch(() => false)) {
                        this.logDebug(`[WaitFallback] Found: ${selector}`);
                        return { element: element.first(), selector };
                    }
                } else {
                    this.logDebug(`[WaitFallback] Found: ${selector}`);
                    return { element: element.first(), selector };
                }
            } catch (_error) {
                this.logDebug(`[WaitFallback] Not found within timeout: ${selector}`);
                continue;
            }
        }

        return null;
    }

    // =========================================================================
    // LOGGING HELPERS
    // =========================================================================

    /**
     * Debug logging helper
     */
    logDebug(message) {
        if (this.debugMode) {
            logger.debug(message);
        }
    }

    /**
     * Warning logging helper
     */
    logWarn(message) {
        logger.warn(message);
    }

    /**
     * Log step in a sequence
     */
    logStep(stepName, details = '') {
        logger.debug(`[STEP] ${stepName}${details ? ': ' + details : ''}`);
    }
}

export default HumanInteraction;
