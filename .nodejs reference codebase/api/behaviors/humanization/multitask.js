/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { api } from '../../index.js';
/**
 * Multitask Engine
 * Background activity simulation during "reading" or "idle" periods
 *
 * Background Activities:
 * 1. Check notifications (30%)
 * 2. Glance trending sidebar (25%)
 * 3. Shift position (20%)
 * 4. Check mentions (15%)
 * 5. Pure idle (10%)
 */

import { mathUtils } from '../../utils/math.js';
import { scrollRandom } from '../scroll-helper.js';

export class MultitaskEngine {
    constructor(page, logger) {
        this.page = page;
        this.logger = logger;
        this.activities = [
            { name: 'notifications', weight: 0.3 },
            { name: 'trending', weight: 0.25 },
            { name: 'position', weight: 0.2 },
            { name: 'mentions', weight: 0.15 },
            { name: 'idle', weight: 0.1 },
        ];
    }

    /**
     * Execute a random background activity
     */
    async execute() {
        const activity = this._weightedRandom();

        switch (activity.name) {
            case 'notifications':
                return await this.checkNotifications();
            case 'trending':
                return await this.glanceTrending();
            case 'position':
                return await this.shiftPosition();
            case 'mentions':
                return await this.glanceMentions();
            case 'idle':
                return await this.pureIdle();
            default:
                return await this.shiftPosition();
        }
    }

    /**
     * Check notifications (move to notification area briefly)
     */
    async checkNotifications() {
        // Move to notification bell area (top-right)
        const notifyX = mathUtils.randomInRange(700, 850);
        const notifyY = mathUtils.randomInRange(50, 120);

        await this._moveToArea(notifyX, notifyY, 'notifications');

        // Brief glance (300-800ms)
        await api.wait(1000);

        // Move back to original position area
        await this._moveToArea(400, 400, 'returning');

        return { success: true, activity: 'notifications' };
    }

    /**
     * Glance at trending sidebar
     */
    async glanceTrending() {
        // Move to trending area (right sidebar)
        const trendX = mathUtils.randomInRange(850, 1000);
        const trendY = mathUtils.randomInRange(150, 400);

        await this._moveToArea(trendX, trendY, 'trending');

        // Brief glance at trending topics (400-1200ms)
        await api.wait(1000);

        // Maybe scroll slightly in trending
        if (mathUtils.roll(0.3)) {
            await scrollRandom(30, 80);
        }

        // Move back
        await this._moveToArea(400, 400, 'returning');

        return { success: true, activity: 'trending' };
    }

    /**
     * Shift position (random mouse movement)
     */
    async shiftPosition() {
        // Random position shift
        const shiftX = mathUtils.randomInRange(-100, 100);
        const shiftY = mathUtils.randomInRange(-80, 80);

        await this.page.mouse.move(shiftX, shiftY);

        // Brief pause
        await api.wait(1000);

        return { success: true, activity: 'position_shift' };
    }

    /**
     * Glance at mentions/notifications
     */
    async glanceMentions() {
        // Move to mentions area (top-left/center)
        const mentionX = mathUtils.randomInRange(50, 150);
        const mentionY = mathUtils.randomInRange(50, 150);

        await this._moveToArea(mentionX, mentionY, 'mentions');

        // Brief glance (400-1000ms)
        await api.wait(1000);

        // Move back
        await this._moveToArea(400, 400, 'returning');

        return { success: true, activity: 'mentions' };
    }

    /**
     * Pure idle (literally do nothing)
     */
    async pureIdle() {
        // No mouse movement, just wait
        await api.wait(1000);

        return { success: true, activity: 'idle' };
    }

    /**
     * During reading - quick background check
     */
    async quickCheck() {
        // Quick position shift
        await this.page.mouse.move(
            mathUtils.randomInRange(-30, 30),
            mathUtils.randomInRange(-20, 20)
        );

        await api.wait(1000);

        return { success: true, activity: 'quick_check' };
    }

    /**
     * Notification simulation (for logging)
     */
    async simulateNotification() {
        // Move to check notifications
        await this.checkNotifications();

        // Log as if we saw a notification
        return {
            success: true,
            activity: 'notification_check',
            sawNotification: mathUtils.roll(0.4),
        };
    }

    // ==========================================
    // INTERNAL METHODS
    // ==========================================

    /**
     * Move to a specific area (human-like trajectory)
     */
    async _moveToArea(targetX, targetY, _context) {
        // Get current position (approximate)
        const currentX = 400 + mathUtils.randomInRange(-50, 50);
        const currentY = 400 + mathUtils.randomInRange(-50, 50);

        // Move in 2-3 steps (not direct)
        const steps = mathUtils.randomInRange(2, 3);

        for (let i = 0; i < steps; i++) {
            const progress = (i + 1) / steps;
            const x = currentX + (targetX - currentX) * progress + mathUtils.randomInRange(-20, 20);
            const y = currentY + (targetY - currentY) * progress + mathUtils.randomInRange(-15, 15);

            await this.page.mouse.move(x, y);
            await api.wait(1000);
        }
    }

    /**
     * Weighted random selection
     */
    _weightedRandom() {
        const total = this.activities.reduce((sum, a) => sum + a.weight, 0);
        let random = Math.random() * total;

        for (const activity of this.activities) {
            random -= activity.weight;
            if (random <= 0) {
                return activity;
            }
        }

        return this.activities[0];
    }
}

export default MultitaskEngine;
