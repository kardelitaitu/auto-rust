/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi } from 'vitest';
import { sessionPhases } from '@api/twitter/session-phases.js';

describe('sessionPhases', () => {
    describe('getSessionPhase', () => {
        it('should return warmup for first 10%', () => {
            expect(sessionPhases.getSessionPhase(5, 100)).toBe('warmup');
            expect(sessionPhases.getSessionPhase(9, 100)).toBe('warmup');
        });

        it('should return active for 10% to 80%', () => {
            expect(sessionPhases.getSessionPhase(11, 100)).toBe('active');
            expect(sessionPhases.getSessionPhase(50, 100)).toBe('active');
            expect(sessionPhases.getSessionPhase(79, 100)).toBe('active');
        });

        it('should return cooldown for last 20%', () => {
            expect(sessionPhases.getSessionPhase(81, 100)).toBe('cooldown');
            expect(sessionPhases.getSessionPhase(100, 100)).toBe('cooldown');
        });
    });

    describe('getPhaseModifier', () => {
        it('should return correct modifiers for warmup', () => {
            expect(sessionPhases.getPhaseModifier('like', 'warmup')).toBe(0.4);
            expect(sessionPhases.getPhaseModifier('reply', 'warmup')).toBe(0.5);
        });

        it('should return correct modifiers for active', () => {
            expect(sessionPhases.getPhaseModifier('like', 'active')).toBe(1.0);
        });

        it('should return correct modifiers for cooldown', () => {
            expect(sessionPhases.getPhaseModifier('like', 'cooldown')).toBe(0.7);
        });

        it('should return 1.0 for unknown actions', () => {
            expect(sessionPhases.getPhaseModifier('unknown', 'active')).toBe(1.0);
        });

        it('should fallback to active modifiers for unknown phase', () => {
            // Mock console.warn to keep output clean
            const spy = vi.spyOn(console, 'warn').mockImplementation(() => {});
            expect(sessionPhases.getPhaseModifier('like', 'invalid')).toBe(1.0); // active.like
            spy.mockRestore();
        });
    });

    describe('getOverallModifier', () => {
        it('should return correct overall modifiers', () => {
            expect(sessionPhases.getOverallModifier('warmup')).toBe(0.6);
            expect(sessionPhases.getOverallModifier('active')).toBe(1.0);
            expect(sessionPhases.getOverallModifier('cooldown')).toBe(0.4);
        });
    });

    describe('getPhaseDescription', () => {
        it('should return description for phases', () => {
            expect(sessionPhases.getPhaseDescription('warmup')).toContain('Warming up');
            expect(sessionPhases.getPhaseDescription('active')).toContain('Peak engagement');
            expect(sessionPhases.getPhaseDescription('cooldown')).toContain('Slowing down');
        });
    });

    describe('getPhaseStats', () => {
        it('should return full stats object', () => {
            const stats = sessionPhases.getPhaseStats('warmup');
            expect(stats.phase).toBe('warmup');
            expect(stats.description).toBeDefined();
            expect(stats.modifiers).toBeDefined();
            expect(stats.modifiers.like).toBe(0.4);
        });
    });

    describe('Time Helpers', () => {
        it('should calculate remaining time', () => {
            expect(sessionPhases.calculateRemainingTime(40, 100)).toBe(60);
            expect(sessionPhases.calculateRemainingTime(110, 100)).toBe(0); // Clamped to 0
        });

        it('should check if near end', () => {
            expect(sessionPhases.isNearEnd(91, 100)).toBe(true);
            expect(sessionPhases.isNearEnd(80, 100)).toBe(false);
        });

        it('should check if near start', () => {
            expect(sessionPhases.isNearStart(5, 100)).toBe(true);
            expect(sessionPhases.isNearStart(15, 100)).toBe(false);
        });
    });
});
