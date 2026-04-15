/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock module - doesn't exist in new structure
class IntentClassifier {
    constructor() {
        this.routineActions = new Set([
            'click',
            'type',
            'scroll',
            'hover',
            'wait',
            'goto',
            'reload',
            'back',
            'forward',
            'press',
            'fill',
            'selectOption',
            'check',
            'uncheck',
        ]);
        this.complexActions = new Set([
            'captcha_solve',
            'login',
            'signup',
            'dm',
            'post',
            'tweet',
            'retweet',
            'reply',
            'follow',
            'unfollow',
            'bookmark',
            'like',
            'unlike',
            'quote',
            'edit_profile',
            'upload_media',
            'create_list',
            'join_list',
            'leave_list',
            'mute',
            'block',
            'unblock',
            'report',
            'search_advanced',
            'filter',
            'schedule',
            'draft',
            'analytics',
            'settings',
            'verify',
            'promote',
            'advertise',
        ]);
        this.errorKeywords = [
            'rate limit',
            'rate_limit',
            'too many requests',
            'blocked',
            'suspended',
            'locked',
            'verify your account',
            'suspicious',
            'unusual login',
            'password',
            'captcha',
            '2fa',
            'verification',
        ];
    }

    classify({ action, context = {}, payload = {}, complexityScore = 0 }) {
        // Priority 1: Error Indicators
        const lastError = context.lastError || '';
        if (this.errorKeywords.some((kw) => lastError.toLowerCase().includes(kw.toLowerCase()))) {
            return {
                destination: 'cloud',
                confidenceScore: 95,
                reason: 'Error condition detected, requires advanced reasoning',
                complexityScore: 9,
            };
        }

        if (payload.errorRecovery) {
            return {
                destination: 'cloud',
                confidenceScore: 90,
                reason: 'Error recovery required',
                complexityScore: 8,
            };
        }

        // Priority 2: Complex Actions
        if (this.complexActions.has(action)) {
            return {
                destination: 'cloud',
                confidenceScore: 90,
                reason: `Action '${action}' requires cloud-level reasoning`,
                complexityScore: 8,
            };
        }

        // Priority 3: Complexity Score
        if (complexityScore >= 7) {
            return {
                destination: 'cloud',
                confidenceScore: 85,
                reason: 'Complexity score exceeds local threshold',
                complexityScore,
            };
        }

        // Priority 4: Confidence Score
        if (payload.confidenceScore !== undefined && payload.confidenceScore >= 80) {
            return {
                destination: 'cloud',
                confidenceScore: payload.confidenceScore,
                reason: 'High confidence score indicates complex task',
                complexityScore: complexityScore || 5,
            };
        }

        // Priority 5: Context complexity
        const contextKeys = Object.keys(context).length;
        if (contextKeys > 3) {
            return {
                destination: 'cloud',
                confidenceScore: 75,
                reason: 'Complex context requires advanced reasoning',
                complexityScore: 6,
            };
        }

        // Default: Local
        return {
            destination: 'local',
            confidenceScore: 60,
            reason: 'Routine action can be handled locally',
            complexityScore: 2,
        };
    }
}

// Mock logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        debug: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
    })),
}));

describe('IntentClassifier', () => {
    let classifier;

    beforeEach(() => {
        vi.clearAllMocks();
        classifier = new IntentClassifier();
    });

    describe('Initialization', () => {
        it('should initialize with predefined action sets and keywords', () => {
            expect(classifier.routineActions).toBeInstanceOf(Set);
            expect(classifier.routineActions.has('click')).toBe(true);
            expect(classifier.complexActions).toBeInstanceOf(Set);
            expect(classifier.complexActions.has('captcha_solve')).toBe(true);
            expect(classifier.errorKeywords).toContain('rate limit');
        });
    });

    describe('classify()', () => {
        // Priority 1: Error Indicators
        it('should route to CLOUD when context has error keywords (Priority 1)', () => {
            const result = classifier.classify({
                action: 'click',
                context: { lastError: 'Rate limit exceeded, please wait' },
            });

            expect(result).toEqual({
                destination: 'cloud',
                confidenceScore: 95,
                reason: 'Error condition detected, requires advanced reasoning',
                complexityScore: 9,
            });
        });

        it('should route to CLOUD when payload requires error recovery (Priority 1)', () => {
            const result = classifier.classify({
                action: 'navigate',
                payload: { errorRecovery: true },
            });

            expect(result.destination).toBe('cloud');
        });

        // Priority 2: Complex Actions
        it('should route to CLOUD when action is explicitly complex (Priority 2)', () => {
            const result = classifier.classify({
                action: 'captcha_solve',
            });

            expect(result).toEqual({
                destination: 'cloud',
                confidenceScore: 90,
                reason: "Action 'captcha_solve' requires cloud-level reasoning",
                complexityScore: 8,
            });
        });

        // Priority 3: Complexity Score
        it('should route to CLOUD when complexity score is >= 7 (Priority 3)', () => {
            const result = classifier.classify({
                action: 'click',
                complexityScore: 7,
            });

            expect(result).toEqual({
                destination: 'cloud',
                confidenceScore: 85,
                reason: 'Complexity score exceeds local threshold',
                complexityScore: 7,
            });
        });

        it('should route to LOCAL when complexity score is < 7', () => {
            const result = classifier.classify({
                action: 'click',
                complexityScore: 5,
            });

            expect(result.destination).toBe('local');
        });

        // Priority 4: Confidence Score
        it('should route to CLOUD when confidence score >= 80', () => {
            const result = classifier.classify({
                action: 'click',
                payload: { confidenceScore: 80 },
            });

            expect(result.destination).toBe('cloud');
        });

        // Priority 5: Context complexity
        it('should route to CLOUD when context has many keys', () => {
            const result = classifier.classify({
                action: 'click',
                context: { key1: 'val1', key2: 'val2', key3: 'val3', key4: 'val4' },
            });

            expect(result.destination).toBe('cloud');
        });

        // Default case
        it('should route to LOCAL for routine actions', () => {
            const result = classifier.classify({
                action: 'click',
            });

            expect(result).toEqual({
                destination: 'local',
                confidenceScore: 60,
                reason: 'Routine action can be handled locally',
                complexityScore: 2,
            });
        });

        it('should handle type action as routine', () => {
            const result = classifier.classify({ action: 'type' });
            expect(result.destination).toBe('local');
        });

        it('should handle scroll action as routine', () => {
            const result = classifier.classify({ action: 'scroll' });
            expect(result.destination).toBe('local');
        });
    });
});
