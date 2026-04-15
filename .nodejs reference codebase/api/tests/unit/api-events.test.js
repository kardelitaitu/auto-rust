/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

describe('api/core/events', () => {
    let eventsModule;
    let apiEvents;

    beforeEach(async () => {
        vi.resetModules();
        eventsModule = await import('../../../api/core/events.js');
        apiEvents = eventsModule.apiEvents;
    });

    afterEach(() => {
        vi.restoreAllMocks();
    });

    describe('getAvailableHooks', () => {
        it('should return array of hook names', () => {
            const hooks = eventsModule.getAvailableHooks();
            expect(Array.isArray(hooks)).toBe(true);
            expect(hooks.length).toBeGreaterThan(0);
        });

        it('should include lifecycle hooks', () => {
            const hooks = eventsModule.getAvailableHooks();
            expect(hooks).toContain('before:init');
            expect(hooks).toContain('after:init');
        });

        it('should include navigation hooks', () => {
            const hooks = eventsModule.getAvailableHooks();
            expect(hooks).toContain('before:navigate');
            expect(hooks).toContain('after:navigate');
        });

        it('should include action hooks', () => {
            const hooks = eventsModule.getAvailableHooks();
            expect(hooks).toContain('before:click');
            expect(hooks).toContain('after:click');
            expect(hooks).toContain('before:type');
            expect(hooks).toContain('after:type');
        });

        it('should include error hooks', () => {
            const hooks = eventsModule.getAvailableHooks();
            expect(hooks).toContain('on:error');
            expect(hooks).toContain('on:action:error');
            expect(hooks).toContain('on:recovery');
            expect(hooks).toContain('on:detection');
        });
    });

    describe('getHookDescription', () => {
        it('should return description for valid hook', () => {
            const desc = eventsModule.getHookDescription('before:click');
            expect(desc).toBe('Called before click action');
        });

        it('should return description for navigation hook', () => {
            const desc = eventsModule.getHookDescription('after:navigate');
            expect(desc).toBe('Called after navigation completes');
        });

        it('should return description for error hook', () => {
            const desc = eventsModule.getHookDescription('on:error');
            expect(desc).toBe('Called on any error');
        });

        it('should return undefined for unknown hook', () => {
            const desc = eventsModule.getHookDescription('unknown:hook');
            expect(desc).toBeUndefined();
        });
    });

    describe('APIEvents', () => {
        it('should create instance with increased max listeners', () => {
            const instance = new eventsModule.APIEvents();
            expect(instance.getMaxListeners()).toBe(50);
        });

        it('should have emitAsync method', () => {
            expect(typeof apiEvents.emitAsync).toBe('function');
        });

        it('should have emitStrict method', () => {
            expect(typeof apiEvents.emitStrict).toBe('function');
        });

        it('should have emitSafe method', () => {
            expect(typeof apiEvents.emitSafe).toBe('function');
        });

        it('should have emitSync method', () => {
            expect(typeof apiEvents.emitSync).toBe('function');
        });
    });

    describe('emitAsync', () => {
        it('should call async handlers and return results', async () => {
            const handler = vi.fn().mockResolvedValue('result');
            apiEvents.on('test:async', handler);

            const results = await apiEvents.emitAsync('test:async', 'arg1', 'arg2');

            expect(handler).toHaveBeenCalledWith('arg1', 'arg2');
            expect(results).toContain('result');
            apiEvents.removeAllListeners();
        });

        it('should handle handler errors gracefully', async () => {
            const errorHandler = vi.fn().mockRejectedValue(new Error('Handler error'));
            const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
            apiEvents.on('test:error', errorHandler);

            const results = await apiEvents.emitAsync('test:error');

            expect(results[0]).toBeNull();
            expect(consoleSpy).toHaveBeenCalled();
            consoleSpy.mockRestore();
            apiEvents.removeAllListeners();
        });

        it('should handle error with no message property', async () => {
            const errorHandler = vi.fn().mockRejectedValue('string error');
            const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
            apiEvents.on('test:noMsg', errorHandler);

            const results = await apiEvents.emitAsync('test:noMsg');

            expect(results[0]).toBeNull();
            expect(consoleSpy).toHaveBeenCalled();
            consoleSpy.mockRestore();
            apiEvents.removeAllListeners();
        });

        it('should return null for rejected handlers', async () => {
            const handler = vi.fn().mockRejectedValue(new Error('fail'));
            apiEvents.on('test:reject', handler);

            const results = await apiEvents.emitAsync('test:reject');

            expect(results[0]).toBeNull();
            apiEvents.removeAllListeners();
        });

        it('should handle mixed success and failure', async () => {
            const successHandler = vi.fn().mockResolvedValue('ok');
            const failHandler = vi.fn().mockRejectedValue(new Error('fail'));
            apiEvents.on('test:mixed', successHandler);
            apiEvents.on('test:mixed', failHandler);

            const results = await apiEvents.emitAsync('test:mixed');

            expect(results[0]).toBe('ok');
            expect(results[1]).toBeNull();
            apiEvents.removeAllListeners();
        });
    });

    describe('emitStrict', () => {
        it('should return results for successful handlers', async () => {
            const handler = vi.fn().mockResolvedValue('result');
            apiEvents.on('test:strict', handler);

            const results = await apiEvents.emitStrict('test:strict', 'arg');

            expect(handler).toHaveBeenCalledWith('arg');
            expect(results).toContain('result');
            apiEvents.removeAllListeners();
        });

        it('should throw AggregateError when handlers fail', async () => {
            const handler = vi.fn().mockRejectedValue(new Error('fail'));
            apiEvents.on('test:strict:fail', handler);

            await expect(apiEvents.emitStrict('test:strict:fail')).rejects.toThrow(AggregateError);
            apiEvents.removeAllListeners();
        });

        it('should include all errors in AggregateError', async () => {
            const handler1 = vi.fn().mockRejectedValue(new Error('error1'));
            const handler2 = vi.fn().mockRejectedValue(new Error('error2'));
            apiEvents.on('test:strict:multi', handler1);
            apiEvents.on('test:strict:multi', handler2);

            try {
                await apiEvents.emitStrict('test:strict:multi');
            } catch (e) {
                expect(e.errors).toHaveLength(2);
                expect(e.errors[0].message).toBe('error1');
                expect(e.errors[1].message).toBe('error2');
            }
            apiEvents.removeAllListeners();
        });
    });

    describe('emitSafe', () => {
        it('should emit hook asynchronously', async () => {
            const handler = vi.fn();
            apiEvents.on('test:safe', handler);

            apiEvents.emitSafe('test:safe', 'arg');

            // Should not call immediately
            expect(handler).not.toHaveBeenCalled();

            // Wait for next tick
            await new Promise((resolve) => process.nextTick(resolve));
            expect(handler).toHaveBeenCalledWith('arg');
            apiEvents.removeAllListeners();
        });

        it('should pass multiple arguments', () => {
            const handler = vi.fn();
            apiEvents.on('test:safe:args', handler);

            apiEvents.emitSafe('test:safe:args', 'a', 'b', 'c');

            process.nextTick(() => {
                expect(handler).toHaveBeenCalledWith('a', 'b', 'c');
                apiEvents.removeAllListeners();
            });
        });
    });

    describe('emitSync', () => {
        it('should call handlers synchronously', () => {
            const handler = vi.fn().mockReturnValue('sync result');
            apiEvents.on('test:sync', handler);

            const results = apiEvents.emitSync('test:sync', 'arg');

            expect(handler).toHaveBeenCalledWith('arg');
            expect(results).toContain('sync result');
            apiEvents.removeAllListeners();
        });

        it('should catch and log errors', () => {
            const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
            const handler = vi.fn(() => {
                throw new Error('sync error');
            });
            apiEvents.on('test:sync:error', handler);

            const results = apiEvents.emitSync('test:sync:error');

            expect(consoleSpy).toHaveBeenCalled();
            expect(results[0]).toBeNull();
            consoleSpy.mockRestore();
            apiEvents.removeAllListeners();
        });

        it('should handle multiple handlers', () => {
            const handler1 = vi.fn().mockReturnValue('r1');
            const handler2 = vi.fn().mockReturnValue('r2');
            apiEvents.on('test:sync:multi', handler1);
            apiEvents.on('test:sync:multi', handler2);

            const results = apiEvents.emitSync('test:sync:multi');

            expect(results).toHaveLength(2);
            expect(results).toContain('r1');
            expect(results).toContain('r2');
            apiEvents.removeAllListeners();
        });
    });

    describe('createHookWrapper', () => {
        it('should wrap async function and emit before/after hooks', async () => {
            const fn = vi.fn().mockResolvedValue('wrapped result');
            const wrapped = eventsModule.createHookWrapper('test', fn);

            const result = await wrapped('arg1', 'arg2');

            expect(fn).toHaveBeenCalledWith('arg1', 'arg2');
            expect(result).toBe('wrapped result');
        });

        it('should not emit before hook when emitBefore is false', async () => {
            const fn = vi.fn().mockResolvedValue('result');
            const emitSpy = vi.spyOn(apiEvents, 'emitSafe');
            const wrapped = eventsModule.createHookWrapper('test', fn, { emitBefore: false });

            await wrapped('arg');

            const beforeCalls = emitSpy.mock.calls.filter((c) => c[0] === 'before:test');
            expect(beforeCalls).toHaveLength(0);
            emitSpy.mockRestore();
        });

        it('should not emit after hook when emitAfter is false', async () => {
            const fn = vi.fn().mockResolvedValue('result');
            const emitSpy = vi.spyOn(apiEvents, 'emitSafe');
            const wrapped = eventsModule.createHookWrapper('test', fn, { emitAfter: false });

            await wrapped('arg');

            const afterCalls = emitSpy.mock.calls.filter((c) => c[0] === 'after:test');
            expect(afterCalls).toHaveLength(0);
            emitSpy.mockRestore();
        });

        it('should emit on:action:error when function throws', async () => {
            const fn = vi.fn().mockRejectedValue(new Error('wrapped error'));
            const emitSpy = vi.spyOn(apiEvents, 'emitSafe');
            const wrapped = eventsModule.createHookWrapper('test', fn);

            await expect(wrapped('arg')).rejects.toThrow('wrapped error');

            const errorCalls = emitSpy.mock.calls.filter((c) => c[0] === 'on:action:error');
            expect(errorCalls).toHaveLength(1);
            expect(errorCalls[0][1].action).toBe('test');
            emitSpy.mockRestore();
        });

        it('should pass result to after hook', async () => {
            const fn = vi.fn().mockResolvedValue('result');
            const emitSpy = vi.spyOn(apiEvents, 'emitSafe');
            const wrapped = eventsModule.createHookWrapper('test', fn);

            await wrapped('arg');

            const afterCalls = emitSpy.mock.calls.filter((c) => c[0] === 'after:test');
            expect(afterCalls[0][2]).toBe('result');
            emitSpy.mockRestore();
        });
    });

    describe('withErrorHook', () => {
        it('should execute function and return result', async () => {
            const fn = vi.fn().mockResolvedValue('success');

            const result = await eventsModule.withErrorHook('test:context', fn);

            expect(fn).toHaveBeenCalled();
            expect(result).toBe('success');
        });

        it('should emit on:error on failure', async () => {
            const fn = vi.fn().mockRejectedValue(new Error('test error'));
            const emitSpy = vi.spyOn(apiEvents, 'emitSafe');

            await expect(eventsModule.withErrorHook('test:context', fn)).rejects.toThrow(
                'test error'
            );

            const errorCalls = emitSpy.mock.calls.filter((c) => c[0] === 'on:error');
            expect(errorCalls).toHaveLength(1);
            expect(errorCalls[0][1].context).toBe('test:context');
            emitSpy.mockRestore();
        });

        it('should emit on:detection on failure', async () => {
            const fn = vi.fn().mockRejectedValue(new Error('detection error'));
            const emitSpy = vi.spyOn(apiEvents, 'emitSafe');

            await expect(eventsModule.withErrorHook('test:context', fn)).rejects.toThrow(
                'detection error'
            );

            const detectionCalls = emitSpy.mock.calls.filter((c) => c[0] === 'on:detection');
            expect(detectionCalls).toHaveLength(1);
            expect(detectionCalls[0][1].type).toBe('error');
            emitSpy.mockRestore();
        });

        it('should re-throw error after emitting hooks', async () => {
            const fn = vi.fn().mockRejectedValue(new Error('original error'));
            const emitSpy = vi.spyOn(apiEvents, 'emitSafe');

            await expect(eventsModule.withErrorHook('test:context', fn)).rejects.toThrow(
                'original error'
            );

            emitSpy.mockRestore();
        });
    });

    describe('default export', () => {
        it('should export all expected members', () => {
            expect(eventsModule.default).toBeDefined();
            expect(eventsModule.default.APIEvents).toBeDefined();
            expect(eventsModule.default.apiEvents).toBeDefined();
            expect(eventsModule.default.getAvailableHooks).toBeDefined();
            expect(eventsModule.default.getHookDescription).toBeDefined();
            expect(eventsModule.default.createHookWrapper).toBeDefined();
            expect(eventsModule.default.withErrorHook).toBeDefined();
        });
    });
});
