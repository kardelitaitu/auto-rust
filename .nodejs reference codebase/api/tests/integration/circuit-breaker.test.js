/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for CircuitBreaker
 * @module tests/integration/circuit-breaker.test
 */

import { describe, it, expect, beforeEach } from 'vitest';
import CircuitBreaker from '@api/core/circuit-breaker.js';

describe('CircuitBreaker', () => {
    let breaker;

    beforeEach(() => {
        breaker = new CircuitBreaker({
            failureThreshold: 1, // 1% - extremely sensitive for testing
            successThreshold: 2,
            halfOpenTime: 500, // Increased to prevent flaky HALF_OPEN transition
            monitoringWindow: 1000,
            minSamples: 1, // Require 1 sample to react immediately
        });
    });

    describe('Basic Operations', () => {
        it('should execute successful calls', async () => {
            const result = await breaker.execute('test-model', async () => 'success');
            expect(result).toBe('success');
        });

        it('should execute and track successful calls', async () => {
            await breaker.execute('test-model', async () => 'success');

            const health = breaker.getHealth('test-model');
            expect(health.status).toBe('CLOSED');
            expect(health.recentOperations).toBeGreaterThanOrEqual(1);
        });
    });

    describe('Circuit States', () => {
        it('should open after failure threshold', async () => {
            for (let i = 0; i < 3; i++) {
                try {
                    await breaker.execute('test-model', async () => {
                        throw new Error('Failed');
                    });
                } catch (_e) {
                    // Expected
                }
            }

            const health = breaker.getHealth('test-model');
            expect(health.status).toBe('OPEN');
        });

        it('should reject calls when open', async () => {
            for (let i = 0; i < 3; i++) {
                try {
                    await breaker.execute('test-model', async () => {
                        throw new Error('Failed');
                    });
                } catch (_e) {
                    // Expected
                }
            }

            // Verify it is open first
            expect(breaker.getHealth('test-model').status).toBe('OPEN');

            try {
                await breaker.execute('test-model', async () => 'success');
                throw new Error('Should have thrown');
            } catch (e) {
                expect(e.code).toBe('CIRCUIT_OPEN');
            }
        });
    });

    describe('Manual Control', () => {
        it('should force open breaker', () => {
            breaker.forceOpen('test-model');

            const health = breaker.getHealth('test-model');
            expect(health.status).toBe('OPEN');
        });

        it('should force close breaker', () => {
            breaker.forceClose('test-model');

            const health = breaker.getHealth('test-model');
            expect(health.status).toBe('CLOSED');
        });
    });
});
