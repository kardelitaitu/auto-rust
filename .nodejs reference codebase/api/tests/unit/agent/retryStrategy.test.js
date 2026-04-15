/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/retryStrategy.js
 * @module tests/unit/agent/retryStrategy.test
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('api/agent/retryStrategy.js', () => {
    let retryStrategy;

    beforeEach(async () => {
        vi.clearAllMocks();
        const module = await import('@api/agent/retryStrategy.js');
        retryStrategy = module.retryStrategy || module.default;
    });

    describe('Constructor', () => {
        it('should initialize with strategies', () => {
            expect(retryStrategy.strategies).toBeDefined();
            expect(retryStrategy.strategies.length).toBeGreaterThan(0);
            expect(retryStrategy.strategies).toContain('same_action_retry');
            expect(retryStrategy.strategies).toContain('alternative_selector');
            expect(retryStrategy.strategies).toContain('scroll_and_retry');
        });

        it('should have strategy descriptions', () => {
            expect(retryStrategy.strategyDescriptions.same_action_retry).toBeDefined();
            expect(retryStrategy.strategyDescriptions.alternative_selector).toBeDefined();
        });
    });

    describe('getNextStrategy()', () => {
        it('should return strategy for selector_not_found error', () => {
            const result = retryStrategy.getNextStrategy(
                'Selector not found',
                { action: 'click', selector: '#btn' },
                0
            );
            expect(result.name).toBeDefined();
            expect(result.description).toBeDefined();
            expect(result.action).toBeDefined();
        });

        it('should return scroll_and_retry for element_not_visible on first attempt', () => {
            const result = retryStrategy.getNextStrategy(
                'Element not visible',
                { action: 'click', selector: '#btn' },
                0
            );
            expect(result.name).toBe('scroll_and_retry');
        });

        it('should return wait_and_retry for element_not_visible on second attempt', () => {
            const result = retryStrategy.getNextStrategy(
                'Element not visible',
                { action: 'click', selector: '#btn' },
                1
            );
            expect(result.name).toBe('wait_and_retry');
        });

        it('should return wait_and_retry for timeout on first attempt', () => {
            const result = retryStrategy.getNextStrategy(
                'Timeout waiting for element',
                { action: 'click', selector: '#btn' },
                0
            );
            expect(result.name).toBe('wait_and_retry');
        });

        it('should return same_action_retry for timeout on second attempt', () => {
            const result = retryStrategy.getNextStrategy(
                'Timeout waiting for element',
                { action: 'click', selector: '#btn' },
                1
            );
            expect(result.name).toBe('same_action_retry');
        });

        it('should cycle through strategies for unknown errors', () => {
            const result = retryStrategy.getNextStrategy(
                'Some unknown error',
                { action: 'click', selector: '#btn' },
                0
            );
            expect(retryStrategy.strategies).toContain(result.name);
        });

        it('should detect verification_failed error', () => {
            const result = retryStrategy.getNextStrategy(
                'Verification failed',
                { action: 'verify' },
                0
            );
            expect(result.name).toBe('wait_and_retry');
        });
    });

    describe('_classifyError()', () => {
        it('should classify selector errors', () => {
            expect(retryStrategy._classifyError('Selector not found')).toBe('selector_not_found');
            expect(retryStrategy._classifyError('Cannot find element')).toBe('selector_not_found');
        });

        it('should classify visibility errors', () => {
            expect(retryStrategy._classifyError('Element not visible')).toBe('element_not_visible');
            expect(retryStrategy._classifyError('Element is hidden')).toBe('element_not_visible');
            expect(retryStrategy._classifyError('Element obscured')).toBe('element_not_visible');
        });

        it('should classify timeout errors', () => {
            expect(retryStrategy._classifyError('Timeout waiting')).toBe('timeout');
            expect(retryStrategy._classifyError('Operation timed out')).toBe('timeout');
        });

        it('should classify verification errors', () => {
            expect(retryStrategy._classifyError('Verification failed')).toBe('verification_failed');
        });

        it('should classify unknown as action_failed', () => {
            expect(retryStrategy._classifyError('Some random error')).toBe('action_failed');
        });
    });

    describe('_getSelectorStrategy()', () => {
        it('should return selector strategies in order', () => {
            const attempt0 = retryStrategy._getSelectorStrategy(0);
            const attempt1 = retryStrategy._getSelectorStrategy(1);
            const attempt2 = retryStrategy._getSelectorStrategy(2);
            const attempt3 = retryStrategy._getSelectorStrategy(3);

            expect(attempt0).toBe('alternative_selector');
            expect(attempt1).toBe('simplify_selector');
            expect(attempt2).toBe('use_text_selector');
            expect(attempt3).toBe('coordinate_click');
        });

        it('should cycle strategies after exhausting list', () => {
            const attempt4 = retryStrategy._getSelectorStrategy(4);
            expect(attempt4).toBe('alternative_selector');
        });
    });

    describe('_getActionStrategy()', () => {
        it('should return action strategies in order', () => {
            const attempt0 = retryStrategy._getActionStrategy(0);
            const attempt1 = retryStrategy._getActionStrategy(1);

            expect(attempt0).toBe('same_action_retry');
            expect(attempt1).toBe('wait_and_retry');
        });
    });

    describe('_generateRetryAction()', () => {
        it('should generate wait action for same_action_retry', () => {
            const result = retryStrategy._generateRetryAction('same_action_retry', {}, {});
            expect(result.action).toBe('wait');
            expect(result.value).toBe('500');
        });

        it('should generate modified action for alternative_selector', () => {
            const result = retryStrategy._generateRetryAction(
                'alternative_selector',
                { action: 'click', selector: '#btn' },
                {}
            );
            expect(result.selector).toBeDefined();
            expect(result.selector).not.toBe('#btn');
        });

        it('should generate clickAt for coordinate_click with coords', () => {
            const result = retryStrategy._generateRetryAction(
                'coordinate_click',
                { action: 'click', selector: '#btn' },
                { lastClickCoords: { x: 100, y: 200 } }
            );
            expect(result.action).toBe('clickAt');
            expect(result.x).toBe(100);
            expect(result.y).toBe(200);
        });

        it('should generate wait for coordinate_click without coords', () => {
            const result = retryStrategy._generateRetryAction(
                'coordinate_click',
                { action: 'click', selector: '#btn' },
                {}
            );
            expect(result.action).toBe('wait');
        });

        it('should generate scroll action for scroll_and_retry', () => {
            const result = retryStrategy._generateRetryAction('scroll_and_retry', {}, {});
            expect(result.action).toBe('scroll');
            expect(result.value).toBe('down');
        });

        it('should generate navigate action for navigate_back', () => {
            const result = retryStrategy._generateRetryAction(
                'navigate_back',
                {},
                { previousUrl: 'https://example.com' }
            );
            expect(result.action).toBe('navigate');
            expect(result.value).toBe('https://example.com');
        });

        it('should simplify selector for simplify_selector', () => {
            const result = retryStrategy._generateRetryAction(
                'simplify_selector',
                { action: 'click', selector: 'div.class[id="test"]' },
                {}
            );
            expect(result.selector).toBeDefined();
        });

        it('should use text selector for use_text_selector', () => {
            const result = retryStrategy._generateRetryAction(
                'use_text_selector',
                { action: 'click', selector: '#submit-btn' },
                {}
            );
            expect(result.selector).toContain('text=');
        });

        it('should return original action for unknown strategy', () => {
            const original = { action: 'click', selector: '#btn' };
            const result = retryStrategy._generateRetryAction('unknown_strategy', original, {});
            expect(result.action).toBe('click');
        });
    });

    describe('_generateAlternativeSelector()', () => {
        it('should convert ID to class selector', () => {
            expect(retryStrategy._generateAlternativeSelector('#myId')).toBe('.myId');
        });

        it('should convert class to ID selector', () => {
            expect(retryStrategy._generateAlternativeSelector('.myClass')).toBe('#myClass');
        });

        it('should simplify complex selector', () => {
            const result = retryStrategy._generateAlternativeSelector('div > span > button');
            expect(result).toBe('button');
        });

        it('should return selector unchanged for simple selector', () => {
            expect(retryStrategy._generateAlternativeSelector('button')).toBe('button');
        });
    });

    describe('_simplifySelector()', () => {
        it('should remove attribute selectors', () => {
            const result = retryStrategy._simplifySelector('button[type="submit"]');
            expect(result).toBe('button');
        });

        it('should remove pseudo-selectors', () => {
            const result = retryStrategy._simplifySelector('li:nth-child(2)');
            expect(result).toBe('li');
        });

        it('should clean up extra spaces', () => {
            const result = retryStrategy._simplifySelector('div  >  span');
            expect(result).toBe('div > span');
        });

        it('should return original if result would be empty', () => {
            const result = retryStrategy._simplifySelector('[required]');
            expect(result).toBeDefined();
        });
    });

    describe('_extractTextFromSelector()', () => {
        it('should extract meaningful text from selector', () => {
            const result = retryStrategy._extractTextFromSelector('#submit-button');
            expect(result).toBe('submit-button');
        });

        it('should handle complex selectors', () => {
            const result = retryStrategy._extractTextFromSelector('div.class#id > span');
            expect(result.length).toBeGreaterThan(0);
        });

        it('should return empty string for no meaningful parts', () => {
            const result = retryStrategy._extractTextFromSelector('123');
            expect(result).toBe('');
        });
    });

    describe('shouldRetry()', () => {
        it('should return true when under max attempts', () => {
            expect(retryStrategy.shouldRetry(0, 3, 'error')).toBe(true);
            expect(retryStrategy.shouldRetry(1, 3, 'error')).toBe(true);
            expect(retryStrategy.shouldRetry(2, 3, 'error')).toBe(true);
        });

        it('should return false when at max attempts', () => {
            expect(retryStrategy.shouldRetry(3, 3, 'error')).toBe(false);
        });

        it('should return false when over max attempts', () => {
            expect(retryStrategy.shouldRetry(4, 3, 'error')).toBe(false);
        });

        it('should not retry fatal errors', () => {
            expect(retryStrategy.shouldRetry(0, 3, 'out of memory')).toBe(false);
            expect(retryStrategy.shouldRetry(0, 3, 'crash detected')).toBe(false);
            expect(retryStrategy.shouldRetry(0, 3, 'fatal error')).toBe(false);
        });

        it('should be case insensitive for fatal errors', () => {
            expect(retryStrategy.shouldRetry(0, 3, 'OUT OF MEMORY')).toBe(false);
            expect(retryStrategy.shouldRetry(0, 3, 'Fatal Error')).toBe(false);
        });
    });

    describe('getRetryDelay()', () => {
        it('should increase delay with exponential backoff', () => {
            const delay0 = retryStrategy.getRetryDelay(0, 'default');
            const delay1 = retryStrategy.getRetryDelay(1, 'default');
            const delay2 = retryStrategy.getRetryDelay(2, 'default');

            expect(delay1).toBeGreaterThan(delay0);
            expect(delay2).toBeGreaterThan(delay1);
        });

        it('should double delay for wait_and_retry', () => {
            const delay = retryStrategy.getRetryDelay(0, 'wait_and_retry');
            expect(delay).toBe(2000);
        });

        it('should use short delay for scroll_and_retry', () => {
            const delay = retryStrategy.getRetryDelay(0, 'scroll_and_retry');
            expect(delay).toBe(500);
        });

        it('should use short delay for same_action_retry', () => {
            const delay = retryStrategy.getRetryDelay(0, 'same_action_retry');
            expect(delay).toBe(500);
        });

        it('should cap delay at 10 seconds', () => {
            const delay = retryStrategy.getRetryDelay(10, 'default');
            expect(delay).toBeLessThanOrEqual(10000);
        });
    });

    describe('getStats()', () => {
        it('should return strategy statistics', () => {
            const stats = retryStrategy.getStats();
            expect(stats.totalStrategies).toBeGreaterThan(0);
            expect(stats.strategies).toBeDefined();
            expect(Array.isArray(stats.strategies)).toBe(true);
        });

        it('should include all strategies', () => {
            const stats = retryStrategy.getStats();
            expect(stats.strategies).toContain('same_action_retry');
            expect(stats.strategies).toContain('scroll_and_retry');
        });
    });

    describe('Edge Cases', () => {
        it('should handle empty error message', () => {
            const result = retryStrategy.getNextStrategy('', { action: 'click' }, 0);
            expect(result.name).toBeDefined();
        });

        it('should handle failed action without selector', () => {
            const result = retryStrategy._generateRetryAction(
                'alternative_selector',
                { action: 'wait', value: '1000' },
                {}
            );
            expect(result.action).toBe('wait');
        });

        it('should handle context without previousUrl', () => {
            const result = retryStrategy._generateRetryAction('navigate_back', {}, {});
            expect(result.value).toBe('/');
        });
    });
});
