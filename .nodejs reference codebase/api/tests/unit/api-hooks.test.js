/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

// Declare mockFn at the top level for all tests
let mockFn;
import { createHookWrapper, withErrorHook } from '@api/core/hooks.js';
import * as context from '@api/core/context.js';

// Mock the context module to control getEvents
vi.mock('@api/core/context.js', () => ({
    getEvents: vi.fn(),
}));

describe('api/core/hooks.js', () => {
    let mockEvents;
    let mockFn;

    beforeEach(() => {
        vi.clearAllMocks();

        mockFn = vi.fn();

        mockEvents = {
            emitSafe: vi.fn(),
            emit: vi.fn(),
            listeners: vi.fn(),
        };

        context.getEvents.mockReturnValue(mockEvents);
    });

    describe('createHookWrapper', () => {
        it('should create a wrapper function that emits before and after events by default', async () => {
            const wrapper = createHookWrapper('click', mockFn);

            await wrapper('arg1', 'arg2');

            expect(mockEvents.emitSafe).toHaveBeenCalledWith('before:click', 'arg1', 'arg2');
            expect(mockEvents.emitSafe).toHaveBeenCalledWith(
                'after:click',
                'arg1',
                'arg2',
                undefined
            );
            expect(mockFn).toHaveBeenCalledWith('arg1', 'arg2');
        });

        it('should not emit before event when emitBefore is false', async () => {
            const wrapper = createHookWrapper('click', mockFn, { emitBefore: false });

            await wrapper('arg1');

            expect(mockEvents.emitSafe).not.toHaveBeenCalledWith('before:click', 'arg1');
            expect(mockEvents.emitSafe).toHaveBeenCalledWith('after:click', 'arg1', undefined);
        });

        it('should not emit after event when emitAfter is false', async () => {
            const wrapper = createHookWrapper('click', mockFn, { emitAfter: false });

            await wrapper('arg1');

            expect(mockEvents.emitSafe).toHaveBeenCalledWith('before:click', 'arg1');
            expect(mockEvents.emitSafe).not.toHaveBeenCalledWith('after:click', 'arg1', undefined);
        });

        it('should emit on:action:error when function throws', async () => {
            const error = new Error('test error');
            mockFn.mockRejectedValue(error);

            const wrapper = createHookWrapper('click', mockFn);

            await expect(wrapper('arg1')).rejects.toThrow('test error');

            expect(mockEvents.emitSafe).toHaveBeenCalledWith('on:action:error', {
                action: 'click',
                error: error,
                args: ['arg1'],
            });
        });

        it('should rethrow the original error after emitting on:action:error', async () => {
            const error = new Error('test error');
            mockFn.mockRejectedValue(error);

            const wrapper = createHookWrapper('click', mockFn);

            await expect(wrapper('arg1')).rejects.toThrow('test error');
            expect(mockFn).toHaveBeenCalled();
        });

        it('should pass through function return value', async () => {
            const result = { success: true };
            mockFn.mockResolvedValue(result);

            const wrapper = createHookWrapper('click', mockFn);

            const actual = await wrapper('arg1');

            expect(actual).toEqual(result);
        });

        it('should handle multiple arguments correctly', async () => {
            const wrapper = createHookWrapper('type', mockFn);

            await wrapper('arg1', 'arg2', 'arg3');

            expect(mockEvents.emitSafe).toHaveBeenCalledWith('before:type', 'arg1', 'arg2', 'arg3');
            expect(mockEvents.emitSafe).toHaveBeenCalledWith(
                'after:type',
                'arg1',
                'arg2',
                'arg3',
                undefined
            );
        });

        it('should work with this context', async () => {
            const context = { foo: 'bar' };
            const wrapper = createHookWrapper('click', function (arg) {
                expect(this).toBe(context);
                return arg;
            });

            await wrapper.call(context, 'test');
        });
    });

    describe('withErrorHook', () => {
        it('should call the function and return its result on success', async () => {
            const result = { success: true };
            mockFn.mockResolvedValue(result);

            const actual = await withErrorHook('test-context', mockFn);

            expect(actual).toEqual(result);
            expect(mockEvents.emitSafe).not.toHaveBeenCalledWith('on:error');
            expect(mockEvents.emitSafe).not.toHaveBeenCalledWith('on:detection');
        });

        it('should emit on:error and on:detection hooks when function throws', async () => {
            const error = new Error('test error');
            mockFn.mockRejectedValue(error);

            await expect(withErrorHook('test-context', mockFn)).rejects.toThrow('test error');

            expect(mockEvents.emitSafe).toHaveBeenCalledWith('on:error', {
                context: 'test-context',
                error: error,
            });

            expect(mockEvents.emitSafe).toHaveBeenCalledWith('on:detection', {
                type: 'error',
                details: 'test error',
            });
        });

        it('should rethrow the original error after emitting hooks', async () => {
            const error = new Error('test error');
            mockFn.mockRejectedValue(error);

            await expect(withErrorHook('test-context', mockFn)).rejects.toThrow('test error');
            expect(mockFn).toHaveBeenCalled();
        });

        it('should handle different error types', async () => {
            const typeError = new TypeError('type error');
            mockFn.mockRejectedValue(typeError);

            await expect(withErrorHook('test-context', mockFn)).rejects.toThrow('type error');

            expect(mockEvents.emitSafe).toHaveBeenCalledWith('on:error', {
                context: 'test-context',
                error: typeError,
            });

            expect(mockEvents.emitSafe).toHaveBeenCalledWith('on:detection', {
                type: 'error',
                details: 'type error',
            });
        });

        it('should include error message in on:detection details', async () => {
            const error = new Error('specific error message');
            mockFn.mockRejectedValue(error);

            await expect(withErrorHook('test-context', mockFn)).rejects.toThrow(
                'specific error message'
            );

            expect(mockEvents.emitSafe).toHaveBeenCalledWith('on:detection', {
                type: 'error',
                details: 'specific error message',
            });
        });
    });

    describe('error handling and edge cases', () => {
        it('should handle empty name', () => {
            const wrapper = createHookWrapper('', mockFn);
            expect(wrapper).toBeDefined();
        });
    });

    describe('coverage completeness', () => {
        it('should cover success path with both emissions', async () => {
            const wrapper = createHookWrapper('type', mockFn);
            await wrapper('arg1');
            expect(mockEvents.emitSafe).toHaveBeenCalledWith('before:type', 'arg1');
            expect(mockEvents.emitSafe).toHaveBeenCalledWith('after:type', 'arg1', undefined);
        });

        it('should cover error path with rethrow', async () => {
            const error = new Error('test');
            mockFn.mockRejectedValue(error);

            const wrapper = createHookWrapper('click', mockFn);
            await expect(wrapper('arg1')).rejects.toThrow('test');
            expect(mockEvents.emitSafe).toHaveBeenCalledWith('on:action:error', {
                action: 'click',
                error: error,
                args: ['arg1'],
            });
        });

        it('should cover withErrorHook success path', async () => {
            mockFn.mockResolvedValue('success');
            await withErrorHook('test', mockFn);
            expect(mockEvents.emitSafe).not.toHaveBeenCalledWith('on:error');
            expect(mockEvents.emitSafe).not.toHaveBeenCalledWith('on:detection');
        });

        it('should cover withErrorHook error path', async () => {
            const error = new Error('test');
            mockFn.mockRejectedValue(error);

            await expect(withErrorHook('test', mockFn)).rejects.toThrow('test');
            expect(mockEvents.emitSafe).toHaveBeenCalledWith('on:error', {
                context: 'test',
                error: error,
            });
            expect(mockEvents.emitSafe).toHaveBeenCalledWith('on:detection', {
                type: 'error',
                details: 'test',
            });
        });
    });

    describe('integration with context', () => {
        it('should work with multiple wrappers', async () => {
            const mockFn1 = vi.fn().mockResolvedValue('success1');
            const mockFn2 = vi.fn().mockResolvedValue('success2');

            const wrapper1 = createHookWrapper('click', mockFn1);
            const wrapper2 = createHookWrapper('type', mockFn2);

            await wrapper1('arg1');
            await wrapper2('arg2');

            expect(mockEvents.emitSafe).toHaveBeenCalledWith('before:click', 'arg1');
            expect(mockEvents.emitSafe).toHaveBeenCalledWith('after:click', 'arg1', 'success1');
            expect(mockEvents.emitSafe).toHaveBeenCalledWith('before:type', 'arg2');
            expect(mockEvents.emitSafe).toHaveBeenCalledWith('after:type', 'arg2', 'success2');
        });
    });

    describe('edge case testing', () => {
        it('should handle functions that return non-promise values', async () => {
            mockFn.mockReturnValue('sync-result');

            const wrapper = createHookWrapper('click', mockFn);
            const result = await wrapper('arg1');

            expect(result).toBe('sync-result');
            expect(mockEvents.emitSafe).toHaveBeenCalledWith('after:click', 'arg1', 'sync-result');
        });

        it('should handle functions that throw synchronously', async () => {
            const error = new Error('sync error');
            mockFn.mockImplementation(() => {
                throw error;
            });

            const wrapper = createHookWrapper('click', mockFn);

            await expect(wrapper('arg1')).rejects.toThrow('sync error');
            expect(mockEvents.emitSafe).toHaveBeenCalledWith('on:action:error', {
                action: 'click',
                error: error,
                args: ['arg1'],
            });
        });
    });
});
