/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { GoHomeAction } from '@api/actions/ai-twitter-go-home.js';

// Mock logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        error: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('GoHomeAction', () => {
    let goHomeAction;
    let mockAgent;

    beforeEach(() => {
        vi.clearAllMocks();

        mockAgent = {
            twitterConfig: {
                actions: {
                    goHome: {
                        enabled: true,
                    },
                },
            },
            navigateHome: vi.fn().mockResolvedValue(undefined),
            page: {
                url: vi.fn().mockReturnValue('http://twitter.com/home'),
            },
        };

        goHomeAction = new GoHomeAction(mockAgent);
    });

    describe('constructor', () => {
        it('should load config', () => {
            expect(goHomeAction.enabled).toBe(true);
        });

        it('should default enabled to true', () => {
            const agent = { twitterConfig: {} };
            const action = new GoHomeAction(agent);
            expect(action.enabled).toBe(true);
        });
    });

    describe('canExecute', () => {
        it('should allow if enabled', async () => {
            const result = await goHomeAction.canExecute();
            expect(result.allowed).toBe(true);
        });

        it('should reject if disabled', async () => {
            goHomeAction.enabled = false;
            const result = await goHomeAction.canExecute();
            expect(result.allowed).toBe(false);
            expect(result.reason).toBe('action_disabled');
        });
    });

    describe('execute', () => {
        it('should navigate home successfully', async () => {
            const result = await goHomeAction.execute();

            expect(mockAgent.navigateHome).toHaveBeenCalled();
            expect(result.success).toBe(true);
            expect(goHomeAction.stats.successes).toBe(1);
        });

        it('should handle navigation failure', async () => {
            mockAgent.navigateHome.mockRejectedValue(new Error('Nav failed'));

            const result = await goHomeAction.execute();

            expect(result.success).toBe(false);
            expect(result.reason).toBe('exception');
            expect(goHomeAction.stats.failures).toBe(1);
        });
    });

    describe('tryExecute', () => {
        it('should execute if allowed', async () => {
            const result = await goHomeAction.tryExecute();
            expect(result.executed).toBe(true);
        });

        it('should not execute if not allowed', async () => {
            goHomeAction.enabled = false;
            const result = await goHomeAction.tryExecute();
            expect(result.executed).toBe(false);
        });
    });

    describe('stats', () => {
        it('should track stats correctly', async () => {
            await goHomeAction.execute();

            const stats = goHomeAction.getStats();
            expect(stats.attempts).toBe(1);
            expect(stats.successes).toBe(1);
            expect(stats.successRate).toBe('100.0%');
        });

        it('should reset stats', async () => {
            await goHomeAction.execute();
            goHomeAction.resetStats();

            const stats = goHomeAction.getStats();
            expect(stats.attempts).toBe(0);
            expect(stats.successes).toBe(0);
        });
    });
});
