/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { api } from '../../index.js';
import { mathUtils } from '../../utils/math.js';
import { entropy } from '../../utils/entropyController.js';

export class BaseHandler {
    constructor(agent) {
        this.agent = agent;
        this.page = agent.page;
        this.config = agent.config;
        this.logger = agent.logger;
        this.human = agent.human;
        this.ghost = agent.ghost;
        this.mathUtils = agent.mathUtils || mathUtils;
        this.entropy = entropy;
    }

    // Common properties with getters/setters for proper access
    get state() {
        return this.agent.state;
    }

    set state(value) {
        this.agent.state = value;
    }

    get sessionStart() {
        return this.agent.sessionStart;
    }

    set sessionStart(value) {
        this.agent.sessionStart = value;
    }

    get sessionEndTime() {
        return this.agent.sessionEndTime;
    }

    set sessionEndTime(value) {
        this.agent.sessionEndTime = value;
    }

    get loopIndex() {
        return this.agent.loopIndex;
    }

    set loopIndex(value) {
        this.agent.loopIndex = value;
    }

    get isFatigued() {
        return this.agent.isFatigued;
    }

    set isFatigued(value) {
        this.agent.isFatigued = value;
    }

    get fatigueThreshold() {
        return this.agent.fatigueThreshold;
    }

    set fatigueThreshold(value) {
        this.agent.fatigueThreshold = value;
    }

    get lastNetworkActivity() {
        return this.agent.lastNetworkActivity;
    }

    set lastNetworkActivity(value) {
        this.agent.lastNetworkActivity = value;
    }

    // Common utility methods
    log(msg) {
        if (this.logger) {
            this.logger.info(msg);
        } else {
            console.log(msg);
        }
    }

    clamp(n, min, max) {
        return Math.max(min, Math.min(max, n));
    }

    checkFatigue() {
        const elapsed = Date.now() - this.sessionStart;
        if (elapsed > this.fatigueThreshold) {
            this.isFatigued = true;
            this.log('💤 Fatigue triggered. Session will end naturally.');
        }
        return this.isFatigued;
    }

    triggerHotSwap() {
        const slowerProfile = this.config.getFatiguedVariant(this.config.timings.scrollPause.mean);
        if (slowerProfile) {
            this.config = slowerProfile;
            this.log('🔄 Hot-swapped to fatigued profile.');
        }
    }

    /**
     * Checks for "Something went wrong" soft error and reloads if found.
     * @param {string|null} [reloadUrl=null] - Optional URL to force navigate to on reload
     * @returns {Promise<boolean>} - True if error was found and reload triggered
     */
    async checkAndHandleSoftError(reloadUrl = null) {
        try {
            // Broad text match for the error header
            const softError = this.page.locator('text=/Something went wrong/i').first();

            // Check if error is visible (fast check)
            if (await api.visible(softError).catch(() => false)) {
                this.state.consecutiveSoftErrors = (this.state.consecutiveSoftErrors || 0) + 1;
                this.log(
                    `⚠️ Soft Error detected: 'Something went wrong'. (Attempt ${this.state.consecutiveSoftErrors}/3)`
                );

                if (this.state.consecutiveSoftErrors >= 3) {
                    this.log(`[SoftError] Maximum retries reached. potential twitter logged out`);
                    throw new Error('potential twitter logged out');
                }

                // Strategy 1: Try clicking explicit "Retry" button if available
                // LIMIT: Only try this 1x (on the first error detection)
                if (this.state.consecutiveSoftErrors === 1) {
                    const retryBtn = this.page
                        .locator('[role="button"][name="Retry"], button:has-text("Retry")')
                        .first();
                    if (await api.visible(retryBtn).catch(() => false)) {
                        this.log(`[SoftError] Found Retry button. Clicking...`);
                        await retryBtn.click().catch(() => {});
                        await api.wait(3000);
                        return true;
                    }
                }

                // Strategy 2: Full Page Reload
                this.log(`[SoftError] No retry button found. Initializing Page Reload...`);
                try {
                    // Reduce chance of infinite reloading the same bad state by waiting a bit
                    await api.wait(2000);

                    const targetUrl = reloadUrl || (await api.getCurrentUrl());
                    if (targetUrl.startsWith('http')) {
                        this.log(`[SoftError] Simulating Refresh by re-entering URL: ${targetUrl}`);
                        await api.goto(targetUrl, {
                            waitUntil: 'domcontentloaded',
                            timeout: 45000,
                        });
                    } else {
                        // Fallback if URL is weird (e.g. about:blank)
                        await api.reload({ waitUntil: 'domcontentloaded', timeout: 45000 });
                    }

                    await api.wait(5000); // Post-refresh wait

                    // VERIFICATION: Did the refresh fix it?
                    if (
                        !(await api
                            .visible(this.page.locator('text=/Something went wrong/i'))
                            .catch(() => false))
                    ) {
                        this.log(`[SoftError] Refresh successful. Error cleared. Resuming task...`);
                        this.state.consecutiveSoftErrors = 0;
                    }

                    return true;
                } catch (e) {
                    this.log(`⚠️ Reload failed: ${e.message}`);
                    return true; // Still return true so we don't proceed with broken page
                }
            } else {
                // No soft error visible, reset counter
                this.state.consecutiveSoftErrors = 0;
            }
        } catch (e) {
            // Propagate the logout error
            if (e.message.includes('potential twitter logged out')) throw e;
            // Ignore other errors (like page closed) during check
        }
        return false;
    }

    getScrollMethod() {
        const methods = ['WHEEL_DOWN', 'PAGE_DOWN', 'ARROW_DOWN'];
        return methods[Math.floor(Math.random() * methods.length)];
    }

    normalizeProbabilities(p) {
        const merged = { ...this.config.probabilities, ...p };

        // Ensure probabilities are valid numbers
        Object.keys(merged).forEach((key) => {
            if (typeof merged[key] === 'number') {
                merged[key] = Math.max(0, Math.min(1, merged[key]));
            }
        });

        // Apply fatigue bias
        if (this.state.fatigueBias > 0) {
            if (merged.tweetDive != null) merged.tweetDive *= 0.7;
            if (merged.profileDive != null) merged.profileDive *= 0.7;
            if (merged.followOnProfile != null) merged.followOnProfile *= 0.5;
        }

        // Apply burst mode adjustments
        if (this.state.activityMode === 'BURST') {
            if (merged.tweetDive != null) merged.tweetDive = 0.2;
            if (merged.profileDive != null) merged.profileDive = 0.2;
        }

        return merged;
    }

    isSessionExpired() {
        const elapsed = Date.now() - this.sessionStart;
        return elapsed > (this.config.maxSessionDuration || 45 * 60 * 1000);
    }

    /**
     * Perform system health check
     * @returns {Promise<Object>} - Health status object
     */
    async performHealthCheck() {
        try {
            // 1. Check Network Vital Signs
            const timeSinceLastActivity = Date.now() - this.lastNetworkActivity;
            if (timeSinceLastActivity > 30000) {
                // 30s Threshold
                return {
                    healthy: false,
                    reason: `network_inactivity_${Math.round(timeSinceLastActivity / 1000)}s`,
                };
            }

            // 2. Check for Critical Error Pages
            // Simple text content scan is usually enough for these standard chrome error pages
            const content = await api.eval(() => document.documentElement.outerHTML);
            if (
                content.includes('ERR_TOO_MANY_REDIRECTS') ||
                content.includes('This page isn’t working') ||
                content.includes('redirected you too many times')
            ) {
                return { healthy: false, reason: 'critical_error_page_redirects' };
            }

            return { healthy: true, reason: '' };
        } catch {
            // If we can't check, assume something is very wrong if network is also silent,
            // but let's be conservative and return healthy if just the check fails but page is alive.
            return { healthy: true, reason: 'check_failed' };
        }
    }

    /**
     * Human-like click with realistic mouse movements and timing
     * @param {Object} target - Playwright locator or element handle
     * @param {string} description - Description for logging (default: 'Target')
     */
    async humanClick(target, description = 'Target') {
        if (!target) return;

        // HUMAN-LIKE: Thinking pause before clicking
        await this.human.think(description);

        try {
            await target.evaluate((el) => el.scrollIntoView({ block: 'center', inline: 'center' }));
            const fixationDelay = this.mathUtils.randomInRange(200, 500);
            await api.wait(fixationDelay);
            await api.wait(this.mathUtils.randomInRange(300, 600));
            const ghostResult = await this.ghost.click(target, {
                label: description,
                hoverBeforeClick: true,
            });
            if (ghostResult?.success && ghostResult?.x != null && ghostResult?.y != null) {
                const x = Math.round(ghostResult.x);
                const y = Math.round(ghostResult.y);
                this.log(`[ai-twitterActivity][api.click] Clicked x=${x} y=${y}`);
            } else if (ghostResult?.success === false) {
                throw new Error('ghost_click_failed');
            }
        } catch (e) {
            this.log(`[Interaction] humanClick failed on ${description}: ${e.message}`);

            // HUMAN-LIKE: Error recovery
            await this.human.recoverFromError('click_failed', { locator: target });
            throw e;
        }
    }

    /**
     * Safe human click with retry logic
     * @param {Object} target - Playwright locator or element handle
     * @param {string} description - Description for logging (default: 'Target')
     * @param {number} retries - Number of retry attempts (default: 3)
     * @returns {Promise<boolean>} - True if click succeeded
     */
    async safeHumanClick(target, description = 'Target', retries = 3) {
        const attemptLogs = [];
        for (let attempt = 1; attempt <= retries; attempt++) {
            try {
                await this.humanClick(target, description);
                this.log(`[Interaction] [${description}] Success on attempt ${attempt}/${retries}`);
                return true;
            } catch (error) {
                attemptLogs.push(`Attempt ${attempt}: ${error.message}`);
                this.log(
                    `[Interaction] [${description}] Attempt ${attempt}/${retries} failed: ${error.message}`
                );
                if (attempt === retries) {
                    this.log(
                        `[Interaction] [${description}] All ${retries} attempts failed: ${attemptLogs.join('; ')}`
                    );
                    return false;
                }
                await api.wait(this.mathUtils.randomInRange(1000, 2000));
            }
        }
        return false;
    }

    /**
     * Check if element is actionable (visible, not disabled, etc.)
     * @param {Object} element - Playwright locator or element handle
     * @returns {Promise<boolean>} - True if element is actionable
     */
    async isElementActionable(element) {
        try {
            const handle = await element.elementHandle();
            if (!handle) return false;

            return await this.page.evaluate(async (el) => {
                const rect = el.getBoundingClientRect();
                if (rect.width === 0 || rect.height === 0) return false;

                const centerX = rect.left + rect.width / 2;
                const centerY = rect.top + rect.height / 2;

                // Check if center is in viewport
                if (
                    centerX < 0 ||
                    centerX > window.innerWidth ||
                    centerY < 0 ||
                    centerY > window.innerHeight
                ) {
                    return false;
                }

                // Check if element is visible and not disabled
                const style = window.getComputedStyle(el);
                if (
                    style.visibility === 'hidden' ||
                    style.display === 'none' ||
                    style.opacity === '0' ||
                    el.disabled ||
                    el.hidden
                ) {
                    return false;
                }

                // Check if element is clickable (has pointer events)
                if (style.pointerEvents === 'none') {
                    return false;
                }

                return true;
            }, handle);
        } catch {
            return false;
        }
    }

    /**
     * Scroll element to golden zone (optimal click position)
     * @param {Object} element - Playwright locator or element handle
     */
    async scrollToGoldenZone(element) {
        try {
            // Use the unified API's focus method which is designed exactly for this
            // It centers the element with entropy and moves the cursor to it
            await api.scroll.focus(element);
        } catch (e) {
            this.log(`[Interaction] scrollToGoldenZone error: ${e.message}`);
        }
    }

    /**
     * Human-like typing with realistic timing and errors
     * @param {Object} element - Playwright locator or element handle
     * @param {string} text - Text to type
     */
    async humanType(element, text) {
        try {
            await element.click();
            await api.wait(this.mathUtils.randomInRange(200, 500));

            for (const char of text) {
                await element.press(char);
                if (this.mathUtils.roll(0.05)) {
                    // 5% chance to make a typing error and correct it
                    await api.wait(this.mathUtils.randomInRange(100, 200));
                    await element.press('Backspace');
                    await api.wait(this.mathUtils.randomInRange(50, 150));
                    await element.press(char);
                }
                await api.wait(this.mathUtils.randomInRange(50, 150));
            }
        } catch (e) {
            this.log(`[Interaction] humanType error: ${e.message}`);
            throw e;
        }
    }

    /**
     * Dismiss overlays, toasts, and modals
     */
    async dismissOverlays() {
        try {
            // Check for toast notifications
            const toasts = this.page.locator('[data-testid="toast"], [role="alert"]');
            if (await api.exists(toasts)) {
                await this.page.keyboard.press('Escape');
                await api.wait(300);
            }

            // Check for modals/dialogs
            const modals = this.page.locator('[role="dialog"], [aria-modal="true"]');
            if (await api.exists(modals)) {
                await this.page.keyboard.press('Escape');
                await api.wait(300);
            }
        } catch {
            // Ignore overlay dismissal errors
        }
    }

    /**
     * Simulate reading behavior with human-like patterns
     * @returns {Promise<void>}
     */
    async simulateReading() {
        const { mean, deviation } = this.config.timings.readingPhase;
        // HUMAN-LIKE: Use content skimming pattern
        await this.human.consumeContent('tweet', 'skim');

        // Calculate duration using Gaussian distribution
        const duration = this.mathUtils.gaussian(mean, deviation || mean * 0.3);
        this.log(`[Reading] Simulating reading for ${Math.round(duration / 1000)}s`);

        // Break reading into chunks with human-like micro-interactions
        const endTime = Date.now() + duration;
        let now = Date.now();
        while (now < endTime) {
            const remaining = endTime - now;
            const chunkTime = Math.min(remaining, this.mathUtils.randomInRange(500, 2000));

            // 15% chance to scroll during reading
            if (this.mathUtils.roll(0.15)) {
                await this.human.scroll('down', 'small');
                await api.wait(this.mathUtils.randomInRange(300, 800));
            }

            // 25% chance to move mouse during reading
            if (this.mathUtils.roll(0.25)) {
                const viewport = this.page.viewportSize();
                const x = this.mathUtils.gaussian(viewport.width / 2, viewport.width / 4);
                const y = this.mathUtils.gaussian(viewport.height / 2, viewport.height / 4);
                const safeX = this.clamp(x, 50, viewport.width - 50);
                const safeY = this.clamp(y, 50, viewport.height - 50);
                await this.ghost.move(safeX, safeY, this.mathUtils.randomInRange(8, 20));
            }

            // 15% chance to type something
            if (this.mathUtils.roll(0.15)) {
                await this.page.keyboard.type(' ', {
                    delay: this.mathUtils.randomInRange(50, 150),
                });
                await api.wait(this.mathUtils.randomInRange(50, 140));
            }

            // 15% chance to look around
            if (this.mathUtils.roll(0.15)) {
                if (this.mathUtils.roll(0.2)) {
                    const viewport = this.page.viewportSize();
                    const x = this.mathUtils.gaussian(viewport.width / 2, viewport.width / 3);
                    const y = this.mathUtils.gaussian(viewport.height / 2, viewport.height / 3);
                    const safeX = this.clamp(x, 50, viewport.width - 50);
                    const safeY = this.clamp(y, 50, viewport.height - 50);
                    await this.ghost.move(safeX, safeY, this.mathUtils.randomInRange(15, 30));
                } else {
                    const distance = this.mathUtils.gaussian(300, 100);
                    await this.ghost.move(distance, 0, this.mathUtils.randomInRange(15, 25));
                }
            }

            await api.wait(chunkTime);
            now = Date.now();
        }
    }
}
