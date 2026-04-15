/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for twitter/session-phases.js
 * @module tests/unit/session-phases.test
 */

import { describe, it, expect } from 'vitest';
import sessionPhases from '@api/twitter/session-phases.js';

const {
    getSessionPhase,
    getPhaseModifier,
    getOverallModifier,
    getPhaseDescription,
    getPhaseStats,
    calculateRemainingTime,
    isNearEnd,
    isNearStart,
} = sessionPhases;

describe('twitter/session-phases.js', () => {
    describe('getSessionPhase', () => {
        it('should return warmup for progress < 10%', () => {
            expect(getSessionPhase(0, 60000)).toBe('warmup');
            expect(getSessionPhase(3000, 60000)).toBe('warmup');
            expect(getSessionPhase(5999, 60000)).toBe('warmup');
        });

        it('should return active for progress between 10% and 80%', () => {
            expect(getSessionPhase(6000, 60000)).toBe('active');
            expect(getSessionPhase(30000, 60000)).toBe('active');
            expect(getSessionPhase(47999, 60000)).toBe('active');
        });

        it('should return cooldown for progress >= 80%', () => {
            expect(getSessionPhase(48000, 60000)).toBe('cooldown');
            expect(getSessionPhase(55000, 60000)).toBe('cooldown');
            expect(getSessionPhase(60000, 60000)).toBe('cooldown');
        });

        it('should handle zero totalMs', () => {
            expect(getSessionPhase(0, 0)).toBe('cooldown');
        });
    });

    describe('getPhaseModifier', () => {
        it('should return correct modifiers for warmup phase', () => {
            expect(getPhaseModifier('reply', 'warmup')).toBe(0.5);
            expect(getPhaseModifier('like', 'warmup')).toBe(0.4);
            expect(getPhaseModifier('retweet', 'warmup')).toBe(0.3);
            expect(getPhaseModifier('follow', 'warmup')).toBe(0.2);
            expect(getPhaseModifier('dive', 'warmup')).toBe(0.7);
        });

        it('should return correct modifiers for active phase', () => {
            expect(getPhaseModifier('reply', 'active')).toBe(1.0);
            expect(getPhaseModifier('like', 'active')).toBe(1.0);
            expect(getPhaseModifier('retweet', 'active')).toBe(1.0);
            expect(getPhaseModifier('follow', 'active')).toBe(1.0);
        });

        it('should return correct modifiers for cooldown phase', () => {
            expect(getPhaseModifier('reply', 'cooldown')).toBe(0.6);
            expect(getPhaseModifier('like', 'cooldown')).toBe(0.7);
            expect(getPhaseModifier('retweet', 'cooldown')).toBe(0.5);
            expect(getPhaseModifier('follow', 'cooldown')).toBe(0.3);
        });

        it('should return default for unknown action', () => {
            expect(getPhaseModifier('unknown', 'active')).toBe(1.0);
        });

        it('should return default for unknown phase', () => {
            expect(getPhaseModifier('reply', 'unknown')).toBe(1.0);
        });
    });

    describe('getOverallModifier', () => {
        it('should return correct overall modifier for each phase', () => {
            expect(getOverallModifier('warmup')).toBe(0.6);
            expect(getOverallModifier('active')).toBe(1.0);
            expect(getOverallModifier('cooldown')).toBe(0.4);
        });

        it('should return 1.0 for unknown phase', () => {
            expect(getOverallModifier('unknown')).toBe(1.0);
        });
    });

    describe('getPhaseDescription', () => {
        it('should return correct description for each phase', () => {
            expect(getPhaseDescription('warmup')).toContain('Warming up');
            expect(getPhaseDescription('active')).toContain('Peak engagement');
            expect(getPhaseDescription('cooldown')).toContain('Slowing down');
        });

        it('should return Unknown for invalid phase', () => {
            expect(getPhaseDescription('invalid')).toBe('Unknown phase');
        });
    });

    describe('getPhaseStats', () => {
        it('should return stats for valid phases', () => {
            const stats = getPhaseStats('warmup');
            expect(stats.phase).toBe('warmup');
            expect(stats.description).toContain('Warming up');
            expect(stats.modifiers).toBeDefined();
            expect(stats.modifiers.reply).toBe(0.5);
        });

        it('should return null for unknown phase', () => {
            expect(getPhaseStats('unknown')).toBeNull();
        });
    });

    describe('calculateRemainingTime', () => {
        it('should calculate remaining time correctly', () => {
            expect(calculateRemainingTime(10000, 60000)).toBe(50000);
            expect(calculateRemainingTime(0, 60000)).toBe(60000);
            expect(calculateRemainingTime(60000, 60000)).toBe(0);
        });

        it('should return 0 when elapsed exceeds total', () => {
            expect(calculateRemainingTime(70000, 60000)).toBe(0);
        });
    });

    describe('isNearEnd', () => {
        it('should return true when near end threshold', () => {
            expect(isNearEnd(54000, 60000, 0.9)).toBe(true);
            expect(isNearEnd(60000, 60000, 0.9)).toBe(true);
        });

        it('should return false when not near end', () => {
            expect(isNearEnd(30000, 60000, 0.9)).toBe(false);
            expect(isNearEnd(50000, 60000, 0.9)).toBe(false);
        });

        it('should use default threshold of 0.9', () => {
            expect(isNearEnd(54000, 60000)).toBe(true);
            expect(isNearEnd(50000, 60000)).toBe(false);
        });
    });

    describe('isNearStart', () => {
        it('should return true when near start threshold', () => {
            expect(isNearStart(3000, 60000, 0.1)).toBe(true);
            expect(isNearStart(0, 60000, 0.1)).toBe(true);
        });

        it('should return false when not near start', () => {
            expect(isNearStart(10000, 60000, 0.1)).toBe(false);
            expect(isNearStart(30000, 60000, 0.1)).toBe(false);
        });

        it('should use default threshold of 0.1', () => {
            expect(isNearStart(3000, 60000)).toBe(true);
            expect(isNearStart(10000, 60000)).toBe(false);
        });
    });

    describe('sessionPhases exports', () => {
        it('should export all phases', () => {
            expect(sessionPhases.phases.warmup).toBeDefined();
            expect(sessionPhases.phases.active).toBeDefined();
            expect(sessionPhases.phases.cooldown).toBeDefined();
        });

        it('should export all modifiers', () => {
            expect(sessionPhases.modifiers.warmup).toBeDefined();
            expect(sessionPhases.modifiers.active).toBeDefined();
            expect(sessionPhases.modifiers.cooldown).toBeDefined();
        });
    });
});
