/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import HistoryCompactor from '@api/core/history-compactor.js';

// Mock Logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn().mockReturnValue({
        info: vi.fn(),
        debug: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
    }),
}));

describe('HistoryCompactor', () => {
    let compactor;

    beforeEach(() => {
        compactor = new HistoryCompactor();
    });

    describe('compactHistory', () => {
        it('should not compact if history is below threshold', () => {
            const actions = [
                { action: 'navigate', target: 'google.com', success: true, timestamp: Date.now() },
                { action: 'click', target: '#search', success: true, timestamp: Date.now() },
            ];

            const result = compactor.compactHistory(actions);

            expect(result.originalCount).toBe(2);
            expect(result.compactedCount).toBe(2);
            expect(result.compressionRatio).toBe(1.0);
            expect(result.summary).toContain('1. ✓ navigate → google.com');
            expect(result.summary).toContain('2. ✓ click → #search');
        });

        it('should compact history if it exceeds threshold', () => {
            // Threshold is 20, let's create 30 actions
            const actions = [];
            for (let i = 0; i < 30; i++) {
                actions.push({
                    action: 'scroll',
                    target: 'page',
                    success: true,
                    timestamp: Date.now() + i,
                });
            }

            const result = compactor.compactHistory(actions);

            expect(result.originalCount).toBe(30);
            expect(result.compactedCount).toBeLessThan(30);
            expect(result.compactedCount).toBeLessThanOrEqual(compactor.targetLength);
            expect(result.summary).toContain('scroll (×30)');
        });

        it('should generate a simple summary with failures', () => {
            const actions = [{ action: 'click', target: 'A', success: false, timestamp: 1 }];
            const result = compactor.compactHistory(actions);
            expect(result.summary).toContain('✗ click → A');
        });

        it('should handle empty history', () => {
            const result = compactor.compactHistory([]);
            expect(result.summary).toBe('No actions recorded.');
            expect(result.originalCount).toBe(0);
        });
    });

    describe('_performCompaction', () => {
        it('should group consecutive similar actions', () => {
            const actions = [
                { action: 'click', target: 'A', success: true, timestamp: 1 },
                { action: 'click', target: 'B', success: true, timestamp: 2 },
                { action: 'click', target: 'C', success: false, timestamp: 3 },
                { action: 'navigate', target: 'D', success: true, timestamp: 4 },
            ];

            const compacted = compactor._performCompaction(actions);

            expect(compacted.length).toBe(2);
            expect(compacted[0].action).toBe('click (×3)');
            expect(compacted[0].target).toBe('3 different targets');
            expect(compacted[0].success).toBe(true); // 2 success vs 1 failure
            expect(compacted[1].action).toBe('navigate');
        });

        it('should handle empty actions array directly', () => {
            const compacted = compactor._performCompaction([]);
            expect(compacted).toEqual([]);
        });

        it('should limit output to targetLength', () => {
            compactor.targetLength = 2;
            const actions = [
                { action: 'type', target: '1', success: true, timestamp: 1 },
                { action: 'click', target: '2', success: true, timestamp: 2 },
                { action: 'scroll', target: '3', success: true, timestamp: 3 },
            ];

            const compacted = compactor._performCompaction(actions);

            expect(compacted.length).toBe(2);
            expect(compacted[0].action).toBe('click');
            expect(compacted[1].action).toBe('scroll');
        });
    });

    describe('generateNarrativeSummary', () => {
        it('should generate a comprehensive narrative', () => {
            const actions = [
                { action: 'navigate', target: 'site.com', success: true },
                { action: 'click', target: 'button', success: true },
                { action: 'type', target: 'input', success: true },
                { action: 'click', target: 'submit', success: false, error: 'Timeout' },
            ];

            const narrative = compactor.generateNarrativeSummary(actions);

            expect(narrative).toContain('Session involved 4 actions.');
            expect(narrative).toContain('Navigated to 1 page(s).');
            expect(narrative).toContain('Performed 2 click(s).');
            expect(narrative).toContain('Typed into 1 field(s).');
            expect(narrative).toContain('Encountered 1 failure(s).');
            expect(narrative).toContain('Errors: Timeout.');
        });

        it('should handle all successes', () => {
            const actions = [{ action: 'click', target: 'A', success: true }];
            const narrative = compactor.generateNarrativeSummary(actions);
            expect(narrative).toContain('All actions succeeded.');
        });

        it('should handle failures without error messages', () => {
            const actions = [{ action: 'click', target: 'A', success: false }];
            const narrative = compactor.generateNarrativeSummary(actions);
            expect(narrative).toContain('Encountered 1 failure(s).');
            expect(narrative).not.toContain('Errors:');
        });

        it('should handle missing action types in narrative', () => {
            const actions = [{ action: 'other', target: 'A', success: true }];
            const narrative = compactor.generateNarrativeSummary(actions);
            expect(narrative).not.toContain('Navigated');
            expect(narrative).not.toContain('Performed click');
            expect(narrative).not.toContain('Typed');
        });

        it('should handle no actions', () => {
            expect(compactor.generateNarrativeSummary([])).toBe('No actions performed.');
        });
    });

    describe('Stats', () => {
        it('should calculate stats correctly', () => {
            const original = new Array(20).fill({});
            const compacted = new Array(5).fill({});

            const stats = compactor.getStats(original, compacted);

            expect(stats.originalCount).toBe(20);
            expect(stats.compactedCount).toBe(5);
            expect(stats.compressionRatio).toBe(0.25);
            expect(stats.tokenSavingsEstimate).toBe(150); // (20-5) * 10
        });
    });
});
