/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@api/core/logger.js', () => ({
    createLogger: () => ({
        info: vi.fn(),
        debug: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
    }),
}));

describe('api/agent/confidenceScorer.js', () => {
    let confidenceScorer;

    beforeEach(async () => {
        vi.clearAllMocks();
        const module = await import('@api/agent/confidenceScorer.js');
        confidenceScorer = module.confidenceScorer || module.default;
    });

    describe('confidenceScorer', () => {
        it('should be defined', () => {
            expect(confidenceScorer).toBeDefined();
        });

        it('should have score method', () => {
            expect(typeof confidenceScorer.score).toBe('function');
        });

        it('should have shouldReprompt method', () => {
            expect(typeof confidenceScorer.shouldReprompt).toBe('function');
        });

        it('should have getConfidenceLevel method', () => {
            expect(typeof confidenceScorer.getConfidenceLevel).toBe('function');
        });

        it('should have getSummary method', () => {
            expect(typeof confidenceScorer.getSummary).toBe('function');
        });

        it('should return confidence level for high score', () => {
            const level = confidenceScorer.getConfidenceLevel(0.9);
            expect(level).toBe('high');
        });

        it('should return confidence level for medium score', () => {
            const level = confidenceScorer.getConfidenceLevel(0.7);
            expect(level).toBe('medium');
        });

        it('should return confidence level for low score', () => {
            const level = confidenceScorer.getConfidenceLevel(0.4);
            expect(level).toBe('low');
        });

        it('should determine when to reprompt', () => {
            expect(confidenceScorer.shouldReprompt(0.3)).toBe(true);
            expect(confidenceScorer.shouldReprompt(0.8)).toBe(false);
        });
    });
});
