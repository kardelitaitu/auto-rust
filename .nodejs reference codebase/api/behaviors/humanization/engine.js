/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { api } from '../../index.js';
/**
 * Humanization Engine - Main Orchestrator
 * Central hub for all human-like behavior patterns
 *
 * Usage:
 *   const human = new HumanizationEngine(page, agent);
 *   await human.scroll();           // Human-like scroll
 *   await human.think('like');      // Thinking pause
 *   await human.contentSkim();      // Skim content
 */

import { createLogger } from '../../core/logger.js';
import { mathUtils } from '../../utils/math.js';
import { entropy as _entropy } from '../../utils/entropyController.js';
import { HumanScroll } from './scroll.js';
import { HumanTiming } from './timing.js';
import { ContentSkimmer } from './content.js';
import { ErrorRecovery } from './error.js';
import { SessionManager } from './session.js';
import { MultitaskEngine } from './multitask.js';
import { scrollRandom } from '../scroll-helper.js';
import { ActionPredictor } from './action.js';

export class HumanizationEngine {
    constructor(page, agent = null) {
        this.page = page;
        this.agent = agent;
        this.logger = createLogger('humanization.js');

        // Initialize sub-engines
        this._scrollEngine = new HumanScroll(page, this.logger);
        this._timingEngine = new HumanTiming(page, this.logger);
        this._contentEngine = new ContentSkimmer(page, this.logger);
        this._errorEngine = new ErrorRecovery(page, this.logger, this);
        this._sessionEngine = new SessionManager(page, this.logger, agent);
        this._multitaskEngine = new MultitaskEngine(page, this.logger);
        this._actionEngine = new ActionPredictor(this.logger);

        // Activity tracking
        this.lastActivity = Date.now();
        this.cycleCount = 0;
        this.isMultitasking = false;

        this.logger.info('[Humanization] Engine initialized');
    }

    /**
     * Set the agent reference (for logging)
     */
    setAgent(agent) {
        this.agent = agent;
        this._scrollEngine.setAgent(agent);
        this._contentEngine.setAgent(agent);
    }

    /**
     * Session management accessor
     */
    get session() {
        return this._sessionEngine;
    }

    /**
     * Log with agent context if available
     */
    log(message) {
        if (this.agent) {
            this.agent.log(`[Human] ${message}`);
        } else {
            this.logger.info(`[Human] ${message}`);
        }
    }

    // ==========================================
    // MAIN ENTRY POINTS
    // ==========================================

    /**
     * Human-like scroll pattern
     * Use this instead of direct page.mouse.wheel()
     */
    async scroll(direction = 'down', intensity = 'normal') {
        this._updateActivity();
        await this._scrollEngine.execute(direction, intensity);
    }

    /**
     * Thinking pause before actions
     * Use before: like, retweet, reply, follow, etc.
     */
    async think(actionType = 'general', context = {}) {
        const thinkTime = this._timingEngine.getThinkTime(actionType, context);
        this.log(`Thinking about ${actionType} for ${Math.round(thinkTime)}ms...`);
        await api.wait(1000);
        this._updateActivity();
    }

    /**
     * Content consumption pattern
     * Use when "reading" tweets
     */
    async consumeContent(type = 'tweet', duration = 'normal') {
        this._updateActivity();
        await this._contentEngine.skipping(type, duration);
    }

    /**
     * Human-like error recovery
     * Use when actions fail
     */
    async recoverFromError(errorType, lastAction) {
        this.log(`Error detected: ${errorType}, recovering...`);
        await this._errorEngine.handle(errorType, lastAction);
        this._updateActivity();
    }

    /**
     * Multitasking simulation
     * Use during "idle" periods
     */
    async multitask() {
        if (this.isMultitasking) return;

        this.isMultitasking = true;
        //this.log('Multitasking (background activity)...');

        const activities = [
            () => this._multitaskEngine.checkNotifications(),
            () => this._multitaskEngine.glanceTrending(),
            () => this._multitaskEngine.shiftPosition(),
            () => this._multitaskEngine.glanceMentions(),
        ];

        // Weighted random activity
        const activity = mathUtils.sample(activities);
        await activity();

        this.isMultitasking = false;
        this._updateActivity();
    }

    /**
     * Predict next action based on patterns
     * Returns: { type: 'scroll'|'click'|'back'|..., confidence: 0-1 }
     */
    predictNextAction() {
        return this._actionEngine.predict(this.cycleCount);
    }

    /**
     * Session start - human-like warmup
     */
    async sessionStart() {
        this.log('Session starting...');

        // Human-like warmup: slowly ramp up activity
        await this._timingEngine.sessionRampUp();

        // Initial scroll to "wake up" the feed
        await api.wait(500);
        await this.scroll('down', 'light');

        this._updateActivity();
        //this.log('Session started');
    }

    /**
     * Session end - human-like wrap-up
     */
    async sessionEnd() {
        this.log('Session ending...');

        // Wrap-up activities before leaving
        await this._sessionEngine.wrapUp(this.page);

        // Final position adjustment
        await scrollRandom(-50, 100);

        this.log('Session ended');
    }

    /**
     * Get human-like session length
     */
    getSessionDuration() {
        return this._sessionEngine.getOptimalLength();
    }

    /**
     * Cycle complete - update patterns
     */
    async cycleComplete() {
        this.cycleCount++;

        // Every 3-4 cycles, add boredom pause
        if (this.cycleCount % 3 === 0) {
            await this._sessionEngine.boredomPause(this.page);
        }

        // Multitasking every 2-3 cycles
        if (this.cycleCount % 2 === 0 && mathUtils.roll(0.15)) {
            await this.multitask();
        }

        this._updateActivity();
    }

    /**
     * Get activity tracking info
     */
    getActivityInfo() {
        return {
            cycleCount: this.cycleCount,
            lastActivity: this.lastActivity,
            isIdle: Date.now() - this.lastActivity > 60000,
            multitasking: this.isMultitasking,
        };
    }

    // ==========================================
    // INTERNAL METHODS
    // ==========================================

    /**
     * Update last activity timestamp
     */
    _updateActivity() {
        this.lastActivity = Date.now();
    }

    /**
     * Helper: Scroll to make element visible
     */
    async scrollToElement(locator, context = 'view') {
        await this._scrollEngine.toElement(locator, context);
    }

    /**
     * Helper: Natural pause between actions
     */
    async naturalPause(context = 'transition') {
        const duration = this._timingEngine.getNaturalPause(context);
        await api.wait(duration);
    }

    /**
     * Helper: Random "reading" behavior
     */
    async readBehavior(duration = 'normal') {
        await this._contentEngine.reading(duration);
    }
}

export default HumanizationEngine;
