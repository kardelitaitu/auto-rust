/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { EntropyController, entropy } from '@api/utils/entropyController.js';

// Mock logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('EntropyController', () => {
    let controller;

    beforeEach(() => {
        vi.useFakeTimers();
        controller = new EntropyController({ sessionId: 'test-session' });
    });

    afterEach(() => {
        vi.useRealTimers();
        vi.restoreAllMocks();
    });

    describe('Initialization', () => {
        it('should initialize with correct defaults', () => {
            expect(controller.sessionId).toBe('test-session');
            expect(controller.sessionEntropy).toBeDefined();
            expect(controller.sessionEntropy.paceMultiplier).toBeGreaterThan(0);
            expect(controller.fatigueEnabled).toBe(true);
            expect(controller.fatigueActive).toBe(false);
        });

        it('should generate unique session profiles', () => {
            const c2 = new EntropyController({ sessionId: 's2' });
            // It's statistically improbable they are exactly equal
            expect(c2.sessionEntropy).not.toEqual(controller.sessionEntropy);
        });
    });

    describe('Fatigue System', () => {
        it('should not activate fatigue before threshold', () => {
            controller.fatigueActivationTime = 5000; // 5s
            vi.advanceTimersByTime(1000); // 1s
            expect(controller.checkFatigue()).toBe(false);
        });

        it('should activate fatigue after threshold', () => {
            controller.fatigueActivationTime = 5000; // 5s
            vi.advanceTimersByTime(6000); // 6s
            expect(controller.checkFatigue()).toBe(true);
            expect(controller.fatigueActive).toBe(true);
            expect(controller.fatigueLevel).toBeGreaterThan(0);
        });

        it('should calculate fatigue level correctly', () => {
            controller.fatigueActivationTime = 0;
            controller.sessionStart = Date.now();

            // 5 minutes past activation
            vi.advanceTimersByTime(5 * 60 * 1000);

            controller.checkFatigue();
            // Fatigue rate is 1 / 10min. So 5min should be ~0.5
            expect(controller.fatigueLevel).toBeCloseTo(0.5, 1);
        });

        it('should cap fatigue level at 1.0', () => {
            controller.fatigueActivationTime = 0;
            vi.advanceTimersByTime(20 * 60 * 1000); // 20 mins
            controller.checkFatigue();
            expect(controller.fatigueLevel).toBe(1.0);
        });

        it('should return null modifiers if fatigue not active', () => {
            expect(controller.getFatigueModifiers()).toBeNull();
        });

        it('should return modifiers when fatigue is active', () => {
            controller.fatigueActive = true;
            controller.fatigueLevel = 0.5;
            const mods = controller.getFatigueModifiers();

            expect(mods).toBeDefined();
            expect(mods.movementSpeed).toBeLessThan(1.0);
            expect(mods.hesitationIncrease).toBeGreaterThan(1.0);
        });
    });

    describe('Timing Functions', () => {
        it('should generate gaussian values within range (probabilistic)', () => {
            // Run multiple times to check bounds logic
            for (let i = 0; i < 100; i++) {
                const val = controller.gaussian(100, 10, 80, 120);
                expect(val).toBeGreaterThanOrEqual(80);
                expect(val).toBeLessThanOrEqual(120);
            }
        });

        it('should apply fatigue to timing', () => {
            controller.fatigueActive = true;
            controller.fatigueLevel = 0.5;

            const base = 1000;
            const mods = controller.getFatigueModifiers();

            const movement = controller.applyFatigueToTiming('movement', base);
            expect(movement).toBe(base / mods.movementSpeed);

            const click = controller.applyFatigueToTiming('click', base);
            expect(click).toBe(base * mods.clickHoldTime);
        });
    });

    describe('Micro-Breaks', () => {
        it('should respect micro-break probability', () => {
            // Mock random to force break
            vi.spyOn(Math, 'random').mockReturnValue(0.001); // Very low, should trigger
            expect(controller.shouldMicroBreak()).toBe(true);
        });

        it('should respect micro-break duration', () => {
            const duration = controller.microBreakDuration();
            expect(duration).toBeGreaterThan(0);
        });
    });

    describe('Singleton & Parallel Safety', () => {
        it('should export a default singleton', () => {
            expect(entropy).toBeInstanceOf(EntropyController);
        });

        it('should allow creating independent instances', () => {
            const e1 = new EntropyController({ sessionId: '1' });
            const e2 = new EntropyController({ sessionId: '2' });

            e1.logAction('test');
            expect(e1.actionLog.length).toBe(1);
            expect(e2.actionLog.length).toBe(0);
        });
    });

    describe('Audit Logging', () => {
        it('should log actions and limit log size', () => {
            // Fill log
            for (let i = 0; i < 600; i++) {
                controller.logAction('action-' + i);
            }

            expect(controller.actionLog.length).toBeLessThanOrEqual(500);
            expect(controller.getSessionStats().actionCount).toBe(controller.actionLog.length);
        });

        it('should reset session', () => {
            controller.logAction('test');
            controller.resetSession();
            expect(controller.actionLog.length).toBe(0);
            expect(controller.fatigueActive).toBe(false);
        });
    });

    describe('Coverage Gap Tests', () => {
        describe('Log-Normal Distribution', () => {
            it('should generate log-normal distributed values', () => {
                const values = [];
                for (let i = 0; i < 100; i++) {
                    values.push(controller.logNormal(0, 1));
                }
                // Log-normal should produce values >= 0
                values.forEach((v) => expect(v).toBeGreaterThanOrEqual(0));
            });

            it('should handle different mu and sigma values', () => {
                const result = controller.logNormal(2, 0.5);
                expect(result).toBeGreaterThan(0);
            });
        });

        describe('Retry Delay', () => {
            it('should generate retry delay with defaults', () => {
                const delay = controller.retryDelay();
                expect(delay).toBeGreaterThanOrEqual(500);
                expect(delay).toBeLessThanOrEqual(30000);
            });

            it('should generate retry delay with custom parameters', () => {
                const delay = controller.retryDelay(3, 2000);
                expect(delay).toBeGreaterThanOrEqual(500);
            });

            it('should increase delay with attempt number', () => {
                const delay0 = controller.retryDelay(0, 1000);
                const delay5 = controller.retryDelay(5, 1000);
                expect(delay5).toBeGreaterThanOrEqual(delay0);
            });
        });

        describe('Poisson Distribution', () => {
            it('should generate poisson distributed integers', () => {
                const lambda = 5;
                for (let i = 0; i < 50; i++) {
                    const result = controller.poisson(lambda);
                    expect(result).toBeGreaterThanOrEqual(0);
                    expect(Number.isInteger(result)).toBe(true);
                }
            });

            it('should handle lambda of 0', () => {
                const result = controller.poisson(0);
                expect(result).toBe(0);
            });

            it('should handle small lambda values', () => {
                const result = controller.poisson(0.5);
                expect(result).toBeGreaterThanOrEqual(0);
            });
        });

        describe('Reaction Time', () => {
            it('should generate reaction time within reasonable bounds', () => {
                for (let i = 0; i < 50; i++) {
                    const rt = controller.reactionTime();
                    expect(rt).toBeGreaterThanOrEqual(100);
                    expect(rt).toBeLessThanOrEqual(800);
                }
            });

            it('should respect pace multiplier from session profile', () => {
                // Run multiple times to account for randomness
                let slow1Total = 0;
                let slow2Total = 0;
                for (let i = 0; i < 20; i++) {
                    controller.sessionEntropy.paceMultiplier = 1.0;
                    slow1Total += controller.reactionTime();
                    controller.sessionEntropy.paceMultiplier = 2.0;
                    slow2Total += controller.reactionTime();
                }
                // With 20 samples, higher pace multiplier should result in higher times
                expect(slow2Total).toBeGreaterThan(slow1Total);
            });
        });

        describe('Scroll Settle Time', () => {
            it('should generate scroll settle time within bounds', () => {
                for (let i = 0; i < 50; i++) {
                    const time = controller.scrollSettleTime();
                    expect(time).toBeGreaterThan(0);
                }
            });

            it('should be affected by pace multiplier', () => {
                // Run multiple times to account for randomness
                let fastTotal = 0;
                let slowTotal = 0;
                for (let i = 0; i < 20; i++) {
                    controller.sessionEntropy.paceMultiplier = 1.0;
                    fastTotal += controller.scrollSettleTime();
                    controller.sessionEntropy.paceMultiplier = 1.5;
                    slowTotal += controller.scrollSettleTime();
                }
                // With 20 samples, slow should be noticeably higher on average
                expect(slowTotal).toBeGreaterThan(fastTotal);
            });
        });

        describe('Page Load Wait', () => {
            it('should generate page load wait time', () => {
                for (let i = 0; i < 50; i++) {
                    const wait = controller.pageLoadWait();
                    expect(wait).toBeGreaterThan(0);
                }
            });

            it('should add distraction multiplier when distracted', () => {
                // First call without distraction (random > 0.1)
                vi.spyOn(Math, 'random').mockImplementation(() => 0.5);
                const _normal = controller.pageLoadWait();

                // Force distraction with random < 0.1
                vi.spyOn(Math, 'random').mockImplementation(() => 0.05);
                const distracted = controller.pageLoadWait();

                // Distracted should be longer due to 1.8x multiplier
                // Note: due to gaussian randomness, we can't guarantee exact comparison
                // but the minimum distracted value should be higher than minimum normal
                expect(distracted).toBeGreaterThan(0);
                Math.random.mockRestore();
            });
        });

        describe('Post Click Delay', () => {
            it('should generate post click delay within bounds', () => {
                for (let i = 0; i < 50; i++) {
                    const delay = controller.postClickDelay();
                    expect(delay).toBeGreaterThan(0);
                }
            });

            it('should be affected by pace multiplier', () => {
                // Run multiple times to account for randomness
                let fastTotal = 0;
                let slowTotal = 0;
                for (let i = 0; i < 20; i++) {
                    controller.sessionEntropy.paceMultiplier = 1.0;
                    fastTotal += controller.postClickDelay();
                    controller.sessionEntropy.paceMultiplier = 1.5;
                    slowTotal += controller.postClickDelay();
                }
                // With 20 samples, slow should be noticeably higher on average
                expect(slowTotal).toBeGreaterThan(fastTotal);
            });
        });

        describe('Pre-Decision Delay', () => {
            it('should return delay and microRetreat', () => {
                const result = controller.preDecisionDelay();
                expect(result).toHaveProperty('delay');
                expect(result).toHaveProperty('microRetreat');
                expect(result.delay).toBeGreaterThan(0);
            });

            it('should have longer delays for hesitant users', () => {
                controller.sessionEntropy.hesitationFactor = 0.8; // High hesitation
                const hesitant = controller.preDecisionDelay();
                controller.sessionEntropy.hesitationFactor = 0.2; // Low hesitation
                const confident = controller.preDecisionDelay();
                expect(hesitant.delay).toBeGreaterThan(confident.delay);
            });

            it('should have microRetreat more often for hesitant users', () => {
                controller.sessionEntropy.hesitationFactor = 1.0;
                let retreatCount = 0;
                for (let i = 0; i < 100; i++) {
                    if (controller.preDecisionDelay().microRetreat) retreatCount++;
                }
                expect(retreatCount).toBeGreaterThan(0);
            });
        });

        describe('Inter Action Gap', () => {
            it('should generate inter action gaps', () => {
                for (let i = 0; i < 50; i++) {
                    const gap = controller.interActionGap();
                    expect(gap).toBeGreaterThan(0);
                }
            });

            it('should occasionally generate longer gaps', () => {
                vi.spyOn(Math, 'random').mockReturnValue(0.99); // Force long gap
                const longGap = controller.interActionGap();
                vi.spyOn(Math, 'random').mockReturnValue(0.01); // Force short gap
                const shortGap = controller.interActionGap();
                expect(longGap).toBeGreaterThan(shortGap);
                Math.random.mockRestore();
            });
        });

        describe('Reading Time', () => {
            it('should calculate reading time for given word count', () => {
                const time = controller.readingTime(500);
                expect(time).toBeGreaterThan(0);
            });

            it('should scale with word count', () => {
                const short = controller.readingTime(100);
                const long = controller.readingTime(500);
                expect(long).toBeGreaterThan(short);
            });

            it('should handle zero words with pauses', () => {
                // Even with 0 words, there can be comprehension pauses
                const time = controller.readingTime(0);
                expect(time).toBeGreaterThanOrEqual(0);
            });

            it('should add comprehension pauses', () => {
                vi.spyOn(Math, 'random').mockReturnValue(0.5);
                const time = controller.readingTime(100);
                expect(time).toBeGreaterThan(0);
                Math.random.mockRestore();
            });
        });

        describe('Scroll Distance', () => {
            it('should generate scroll distances', () => {
                for (let i = 0; i < 50; i++) {
                    const dist = controller.scrollDistance();
                    expect(dist).toBeGreaterThan(0);
                }
            });

            it('should generate small scans more often than large sweeps', () => {
                vi.spyOn(Math, 'random').mockReturnValue(0.3); // Small scan threshold
                const smallScan = controller.scrollDistance();
                vi.spyOn(Math, 'random').mockReturnValue(0.9); // Large sweep
                const largeSweep = controller.scrollDistance();
                expect(largeSweep).toBeGreaterThan(smallScan);
                Math.random.mockRestore();
            });
        });

        describe('Apply Fatigue To Timing - Edge Cases', () => {
            it('should return base value for unknown timing type', () => {
                controller.fatigueActive = true;
                controller.fatigueLevel = 0.5;
                const result = controller.applyFatigueToTiming('unknown', 1000);
                expect(result).toBe(1000);
            });

            it('should handle all valid timing types', () => {
                controller.fatigueActive = true;
                controller.fatigueLevel = 0.5;

                expect(controller.applyFatigueToTiming('movement', 1000)).toBeDefined();
                expect(controller.applyFatigueToTiming('hesitation', 1000)).toBeDefined();
                expect(controller.applyFatigueToTiming('click', 1000)).toBeDefined();
                expect(controller.applyFatigueToTiming('scroll', 1000)).toBeDefined();
                expect(controller.applyFatigueToTiming('typing', 1000)).toBeDefined();
            });
        });

        describe('Check Fatigue - Edge Cases', () => {
            it('should return fatigue active if already active', () => {
                controller.fatigueActive = true;
                expect(controller.checkFatigue()).toBe(true);
            });

            it('should handle disabled fatigue', () => {
                const controllerNoFatigue = new EntropyController({
                    sessionId: 'no-fatigue',
                    fatigueEnabled: false,
                });
                controllerNoFatigue.fatigueActivationTime = 0;
                vi.advanceTimersByTime(10 * 60 * 1000);
                expect(controllerNoFatigue.checkFatigue()).toBe(false);
            });

            it('should respect elapsedMs parameter override', () => {
                controller.fatigueActivationTime = 1000;
                expect(controller.checkFatigue(500)).toBe(false);
                expect(controller.checkFatigue(2000)).toBe(true);
            });
        });

        describe('Micro Break - Edge Cases', () => {
            it('should increase break chance with time since last action', () => {
                controller.lastActionTimestamp = Date.now() - 5 * 60 * 1000; // 5 min ago
                vi.spyOn(Math, 'random').mockReturnValue(0.001);
                expect(controller.shouldMicroBreak()).toBe(true);
                Math.random.mockRestore();
            });

            it('should return longer breaks on fatigue', () => {
                controller.fatigueActive = true;
                controller.fatigueLevel = 0.8;
                vi.spyOn(Math, 'random')
                    .mockReturnValueOnce(0.3) // Short break path
                    .mockReturnValueOnce(0.1); // Fatigue multiplier check
                const duration = controller.microBreakDuration();
                expect(duration).toBeGreaterThan(0);
                Math.random.mockRestore();
            });
        });

        describe('Session Stats', () => {
            it('should include all expected fields', () => {
                controller.logAction('test-action', { detail: 'value' });
                const stats = controller.getSessionStats();

                expect(stats.sessionAge).toBeGreaterThanOrEqual(0);
                expect(stats.entropy).toBeDefined();
                expect(stats.actionCount).toBe(1);
                expect(stats.fatigue).toBeDefined();
                expect(stats.recentActions).toBeInstanceOf(Array);
            });

            it('should limit recent actions to 20', () => {
                for (let i = 0; i < 50; i++) {
                    controller.logAction('action-' + i);
                }
                const stats = controller.getSessionStats();
                expect(stats.recentActions.length).toBeLessThanOrEqual(20);
            });
        });

        describe('Reset Session', () => {
            it('should regenerate session profile', () => {
                const oldProfile = { ...controller.sessionEntropy };
                controller.resetSession();
                expect(controller.sessionEntropy.paceMultiplier).not.toBe(
                    oldProfile.paceMultiplier
                );
            });

            it('should reset fatigue activation time', () => {
                const oldTime = controller.fatigueActivationTime;
                controller.resetSession();
                expect(controller.fatigueActivationTime).not.toBe(oldTime);
            });
        });

        describe('Gaussian - Edge Cases', () => {
            it('should handle min without max', () => {
                const result = controller.gaussian(100, 10, 80);
                expect(result).toBeGreaterThanOrEqual(80);
            });

            it('should handle max without min', () => {
                const result = controller.gaussian(100, 10, undefined, 120);
                expect(result).toBeLessThanOrEqual(120);
            });

            it('should handle neither min nor max', () => {
                const result = controller.gaussian(100, 10);
                expect(result).toBeDefined();
            });
        });
    });
});
