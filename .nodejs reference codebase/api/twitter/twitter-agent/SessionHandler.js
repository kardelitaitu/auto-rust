/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { api } from '../../index.js';
import { BaseHandler } from './BaseHandler.js';
import { mathUtils } from '../../utils/math.js';
import { scrollDown, scrollRandom } from '../../behaviors/scroll-helper.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ACCOUNT_ISSUES_FILE = path.join(__dirname, '../../account-issues.txt');

export class SessionHandler extends BaseHandler {
    constructor(agent) {
        super(agent);
    }

    /**
     * Check if session has expired based on various criteria
     * @returns {boolean} true if session should be considered expired
     */
    isSessionExpired() {
        // Check if we've exceeded fatigue threshold
        if (this.isFatigued) {
            this.log('💤 Session expired due to fatigue.');
            return true;
        }

        // Check if session end time was explicitly set
        if (this.sessionEndTime && Date.now() >= this.sessionEndTime) {
            this.log('⏰ Session expired due to time limit.');
            return true;
        }

        // Check for consecutive login failures
        if (this.state.consecutiveLoginFailures >= 3) {
            this.log('🔒 Session expired due to login failures.');
            return true;
        }

        return false;
    }

    /**
     * Main session runner - executes engagement cycles
     * @param {number} cycles - Number of engagement cycles to run
     * @param {number} minDurationSec - Minimum session duration in seconds
     * @param {number} maxDurationSec - Maximum session duration in seconds
     */
    async runSession(cycles = 10, minDurationSec = 0, maxDurationSec = 0) {
        // Set session duration if specified
        if (minDurationSec > 0 && maxDurationSec > 0) {
            const durationMs = mathUtils.randomInRange(
                minDurationSec * 1000,
                maxDurationSec * 1000
            );
            this.sessionEndTime = Date.now() + durationMs;
            this.log(`⏰ Session time limit set: ${Math.round(durationMs / 1000)}s`);
        }

        // Apply theme if configured
        if (this.config.theme) {
            try {
                await api.emulateMedia({ colorScheme: this.config.theme });
                this.log(`🎨 Applied theme: ${this.config.theme}`);
            } catch (e) {
                this.log(`⚠️ Failed to apply theme: ${e.message}`);
            }
        }

        // Initial navigation to home
        for (let i = 0; i < 3; i++) {
            try {
                await this.agent.navigation.navigateHome();
                break;
            } catch (e) {
                this.log(`❌ Navigation attempt ${i + 1}/3 failed: ${e.message}`);
                if (i < 2) {
                    await api.wait(2000);
                }
            }
        }

        // Check for critical login failures
        if (this.state.consecutiveLoginFailures >= 3) {
            this.log('🔒 Critical: 3+ consecutive login failures. Session aborted.');
            return;
        }

        // Main engagement loop
        this.loopIndex = 0;
        while (true) {
            // Check session expiration conditions
            if (this.isSessionExpired()) {
                break;
            }

            // Check loop completion
            if (!this.sessionEndTime && this.loopIndex >= cycles) {
                this.log(`✅ Completed ${cycles} engagement cycles.`);
                break;
            }

            // Handle burst mode transitions
            if (this.state.activityMode === 'BURST') {
                const now = Date.now();
                if (now > this.state.burstEndTime) {
                    this.state.activityMode = 'NORMAL';
                    this.log('🌀 Burst mode ended, returning to normal activity.');
                }
            }

            // Execute engagement cycle
            await this.executeEngagementCycle();
            this.loopIndex++;

            // Add variable delay between cycles
            const cycleDelay = mathUtils.randomInRange(3000, 8000);
            await api.wait(cycleDelay);
        }

        this.log('🏁 Session completed successfully.');
    }

    /**
     * Execute a single engagement cycle with varied activities
     */
    async executeEngagementCycle() {
        this.log(`🔄 Engagement cycle ${this.loopIndex + 1}...`);

        // Health check
        const health = await this.performHealthCheck();
        if (!health.healthy) {
            this.log(`💀 Health check failed: ${health.reason}`);
            return;
        }

        // Dismiss any overlays
        await this.dismissOverlays();

        // Random engagement selection with weighted probabilities
        const activityRoll = Math.random();

        if (activityRoll < 0.6) {
            // 60% chance: Tweet engagement
            await this.agent.engagement.diveTweet();
        } else if (activityRoll < 0.85) {
            // 25% chance: Profile exploration
            await this.agent.engagement.diveProfile();
        } else {
            // 15% chance: Simulated reading/fidgeting
            await this.simulateReading();
        }

        // Occasional fidgeting behavior
        if (mathUtils.roll(0.2)) {
            await this.simulateFidget();
        }

        this.state.engagements++;
        this.log(`📊 Total engagements: ${this.state.engagements}`);
    }

    /**
     * Simulate human reading behavior with varied patterns
     */
    async simulateReading() {
        const duration = mathUtils.randomInRange(8000, 20000);
        const endTime = Date.now() + duration;

        if (this.state.activityMode === 'BURST') {
            this.log('[Reading] 🌀 Burst mode: Short reading simulation.');
            await api.wait(mathUtils.randomInRange(2000, 5000));
            return;
        }

        this.log(`[Reading] Simulating human reading for ${Math.round(duration / 1000)}s...`);

        let now = Date.now();
        while (now < endTime) {
            // Check for soft errors
            if (await this.checkAndHandleSoftError()) {
                break;
            }

            // Health check
            const health = await this.performHealthCheck();
            if (!health.healthy) {
                break;
            }

            // Random scroll behavior during reading
            if (mathUtils.roll(0.15)) {
                const method = this.getScrollMethod();
                if (method === 'WHEEL_DOWN') {
                    await scrollDown(mathUtils.randomInRange(1, 3));
                } else {
                    await scrollRandom(mathUtils.randomInRange(100, 300));
                }
            }

            // Occasional tab switching
            if (mathUtils.roll(0.25)) {
                await this.agent.navigation.ensureForYouTab();
            }

            // Micro-pauses
            if (mathUtils.roll(0.15)) {
                await api.wait(mathUtils.randomInRange(1000, 3000));
            }

            // Random mouse movements
            if (mathUtils.roll(0.15)) {
                await this.human.randomMouseMovement();
            }

            // Check for tweet dive opportunities
            if (mathUtils.roll(this.config.probabilities?.tweetDive || 0.3)) {
                await this.agent.engagement.diveTweet();
            }

            // Short pause between actions
            await api.wait(mathUtils.randomInRange(500, 1500));
            now = Date.now();
        }

        this.log('[Reading] Reading simulation completed.');
    }

    /**
     * Simulate human fidgeting behavior
     */
    async simulateFidget() {
        const fidgetTypes = ['TEXT_SELECT', 'RANDOM_CLICK', 'SCROLL_JITTER'];
        const fidgetType = fidgetTypes[Math.floor(Math.random() * fidgetTypes.length)];

        this.log(`[Fidget] Simulating ${fidgetType.replace('_', ' ').toLowerCase()}...`);

        try {
            if (fidgetType === 'TEXT_SELECT') {
                // Select random text on page
                const textElements = this.page.locator('p, span, div, h1, h2, h3, h4, h5, h6');
                const count = await textElements.count();

                if (count > 0) {
                    const visibleIndices = [];
                    for (let i = 0; i < Math.min(count, 20); i++) {
                        const element = textElements.nth(i);
                        if (await api.visible(element)) {
                            const box = await element.boundingBox().catch(() => null);
                            if (box && box.height > 0) {
                                visibleIndices.push(i);
                            }
                        }
                    }

                    if (visibleIndices.length > 0) {
                        const randomIndex =
                            visibleIndices[Math.floor(Math.random() * visibleIndices.length)];
                        const targetElement = textElements.nth(randomIndex);

                        // Simulate text selection
                        const box = await targetElement.boundingBox();
                        if (box) {
                            await this.page.mouse.move(box.x + 10, box.y + 5);
                            await api.wait(200);
                            await this.page.mouse.down();
                            await api.wait(100);
                            await this.page.mouse.move(box.x + box.width - 10, box.y + 5);
                            await api.wait(100);
                            await this.page.mouse.up();
                        }
                    }
                }
            } else if (fidgetType === 'RANDOM_CLICK') {
                // Click on random empty space
                const viewport = this.page.viewportSize();
                const x = mathUtils.randomInRange(100, viewport.width - 100);
                const y = mathUtils.randomInRange(100, viewport.height - 100);

                await this.page.mouse.move(x, y);
                await api.wait(300);
                await this.page.mouse.click(x, y);
            } else if (fidgetType === 'SCROLL_JITTER') {
                // Random scroll jitter
                for (let i = 0; i < 3; i++) {
                    await scrollRandom(mathUtils.randomInRange(-100, 100));
                    await api.wait(mathUtils.randomInRange(100, 300));
                }
            }

            await api.wait(mathUtils.randomInRange(500, 1500));
        } catch (error) {
            this.log(`[Fidget] Error: ${error.message}`);
        }
    }

    /**
     * Human-like typing simulation
     */
    async humanType(element, text) {
        this.log(
            `[Type] Simulating human typing: "${text.substring(0, 20)}${text.length > 20 ? '...' : ''}"`
        );

        try {
            await element.click();
            await api.wait(mathUtils.randomInRange(300, 800));

            // Type with human-like timing and errors
            for (const char of text) {
                await element.press(char);

                // Variable typing speed (50-150ms per character)
                await api.wait(mathUtils.randomInRange(50, 150));

                // Occasional typos and corrections (5% chance)
                if (mathUtils.roll(0.05)) {
                    await element.press('Backspace');
                    await api.wait(mathUtils.randomInRange(100, 300));
                    await element.press(char);
                    await api.wait(mathUtils.randomInRange(50, 150));
                }
            }

            await api.wait(mathUtils.randomInRange(500, 1000));
        } catch (error) {
            this.log(`[Type] Error: ${error.message}`);
            throw error;
        }
    }

    /**
     * Post a tweet with human-like behavior
     */
    async postTweet(text) {
        this.log(
            `[Tweet] Composing tweet: "${text.substring(0, 30)}${text.length > 30 ? '...' : ''}"`
        );

        try {
            // Open composer
            const composerButton = this.page.locator(
                '[data-testid="SideNav_NewTweet_Button"], a[href="/compose/tweet"]'
            );
            let composerOpened = false;

            if ((await api.exists(composerButton)) && (await api.visible(composerButton))) {
                await this.safeHumanClick(composerButton, 'Composer Button');
                composerOpened = true;
            }

            if (!composerOpened) {
                // Fallback: Direct URL
                await api.goto('https://x.com/compose/tweet');
                await api.wait(2000);
            }

            // Wait for composer to appear
            const composer = this.page.locator(
                '[data-testid="tweetTextarea_0"], [data-testid="tweetTextView"]'
            );
            await composer.waitFor({ state: 'visible', timeout: 5000 });

            // Type the tweet
            await this.humanType(composer, text);

            // Post the tweet
            const postButton = this.page.locator(
                '[data-testid="tweetButton"], [data-testid="tweetButtonInline"]'
            );
            if ((await api.exists(postButton)) && (await api.visible(postButton))) {
                await this.safeHumanClick(postButton, 'Post Button');

                // Wait for post confirmation
                await api.wait(3000);

                this.state.tweets++;
                this.log(`✅ Tweet posted successfully! Total tweets: ${this.state.tweets}`);
                return true;
            }
        } catch (error) {
            this.log(`❌ Failed to post tweet: ${error.message}`);
        }

        return false;
    }

    /**
     * Logs account issues to a separate file
     * @param {string} status - The detected status (Locked, Verify, LoggedOut)
     */
    _logAccountIssue(status) {
        try {
            const timestamp = new Date().toLocaleString();
            const sessionId = this.agent.logger?.currentSessionId || 'unknown-session';
            const profileId = this.config.id || 'unknown-profile';
            const logEntry = `[${timestamp}] [Session:${sessionId}] [Account:${profileId}] STATUS: ${status}\n`;

            fs.appendFileSync(ACCOUNT_ISSUES_FILE, logEntry, 'utf8');
            this.log(`🚨 Account issue logged to account-issues.txt: ${status}`);
        } catch (e) {
            this.log(`⚠️ Failed to log account issue: ${e.message}`);
        }
    }

    /**
     * Check if user is logged in
     * @returns {Promise<boolean>} - True if logged in, false otherwise
     */
    async checkLoginState() {
        try {
            // 1. Check for Account Locked / Restricted States
            const lockedText = [
                'Account locked',
                'Help us keep your account safe',
                'Your account is temporarily limited',
                'Automated behavior',
            ];

            for (const text of lockedText) {
                const element = this.page.getByText(text).first();
                if (await api.visible(element).catch(() => false)) {
                    this.log(`[CRITICAL] Account LOCKED detected: "${text}"`);
                    this._logAccountIssue('Locked');
                    this.state.consecutiveLoginFailures++;
                    return false;
                }
            }

            // 2. Check for Verification Challenges
            const verifyText = [
                'Verify your identity',
                'Confirm your email',
                'suspicious activity',
                'Enter the verification code',
                'Authenticate your account',
            ];

            for (const text of verifyText) {
                const element = this.page.getByText(text).first();
                if (await api.visible(element).catch(() => false)) {
                    this.log(`[IMPORTANT] Verification challenge detected: "${text}"`);
                    this._logAccountIssue('Verify');
                    this.state.consecutiveLoginFailures++;
                    return false;
                }
            }

            // 3. Check for specific text content indicating logged out state (Relaxed Matching)
            const signedOutText = [
                'Sign in',
                'Sign up with Google',
                'Create account',
                'Join X today',
                'Oops, something went wrong',
            ];

            for (const text of signedOutText) {
                // Removed { exact: true } to be more robust against whitespace/styling
                const element = this.page.getByText(text).first();
                if (await api.visible(element).catch(() => false)) {
                    this.log(`[WARN] Not logged in. Found text: "${text}"`);
                    this._logAccountIssue('LoggedOut');
                    this.state.consecutiveLoginFailures++;
                    return false;
                }
            }

            // Check for specific login selectors
            const loginSelectors = [
                'a[href*="/login"]',
                'a[href*="/i/flow/login"]',
                '[data-testid="loginButton"]',
                '[data-testid="signupButton"]',
                '[data-testid="google_sign_in_container"]',
            ];

            for (const selector of loginSelectors) {
                const element = this.page.locator(selector).first();
                if (await api.visible(element).catch(() => false)) {
                    this.log(`[WARN] Not logged in. Found selector: "${selector}"`);
                    this._logAccountIssue('LoggedOut');
                    this.state.consecutiveLoginFailures++;
                    return false;
                }
            }

            // Heuristic: If we are supposedly on Home but can't see the primary column or interacting elements
            if ((await api.getCurrentUrl()).includes('home')) {
                const mainColumn = this.page.locator('[data-testid="primaryColumn"]');
                if ((await mainColumn.count()) === 0 || !(await api.visible(mainColumn))) {
                    const timeline = this.page.locator('[aria-label="Home timeline"]');
                    if ((await timeline.count()) === 0) {
                        this.log(
                            `[WARN] Suspected not logged in: Primary Timeline not visible on /home.`
                        );
                        this.state.consecutiveLoginFailures++;
                        return false;
                    }
                }
            }

            // Reset if successful
            this.state.consecutiveLoginFailures = 0;
            return true;
        } catch (e) {
            this.log(`[WARN] checkLoginState failed: ${e.message}`);
            this.state.consecutiveLoginFailures++;
            return false;
        }
    }
}
