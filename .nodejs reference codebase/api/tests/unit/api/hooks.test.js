/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createHookWrapper, withErrorHook } from '@api/core/hooks.js';
import * as context from '@api/core/context.js';

vi.mock('@api/core/context.js', () => ({
    getEvents: vi.fn(),
}));

describe('hooks', () => {
    let mockEvents;

    beforeEach(() => {
        vi.clearAllMocks();
        mockEvents = {
            emitSafe: vi.fn(),
        };
        context.getEvents.mockReturnValue(mockEvents);
    });

    describe('createHookWrapper', () => {
        it('should emit before and after hooks', async () => {
            const fn = vi.fn().mockResolvedValue('success');
            const wrapped = createHookWrapper('test', fn);

            const result = await wrapped('arg1', 'arg2');

            expect(result).toBe('success');
            expect(fn).toHaveBeenCalledWith('arg1', 'arg2');
            expect(mockEvents.emitSafe).toHaveBeenCalledWith('before:test', 'arg1', 'arg2');
            expect(mockEvents.emitSafe).toHaveBeenCalledWith(
                'after:test',
                'arg1',
                'arg2',
                'success'
            );
        });

        it('should respect emitBefore/emitAfter options', async () => {
            const fn = vi.fn().mockResolvedValue('ok');
            const wrapped = createHookWrapper('test', fn, { emitBefore: false, emitAfter: false });

            await wrapped();
            expect(mockEvents.emitSafe).not.toHaveBeenCalled();
        });

        it('should emit on:action:error and rethrow on failure', async () => {
            const error = new Error('fail');
            const fn = vi.fn().mockRejectedValue(error);
            const wrapped = createHookWrapper('test', fn);

            await expect(wrapped('bad')).rejects.toThrow(error);
            expect(mockEvents.emitSafe).toHaveBeenCalledWith('on:action:error', {
                action: 'test',
                error,
                args: ['bad'],
            });
        });
    });

    describe('withErrorHook', () => {
        it('should return result if successful', async () => {
            const result = await withErrorHook('my-ctx', async () => 'yay');
            expect(result).toBe('yay');
            expect(mockEvents.emitSafe).not.toHaveBeenCalled();
        });

        it('should emit errors and rethrow on failure', async () => {
            const error = new Error('boom');
            const fn = vi.fn().mockRejectedValue(error);

            await expect(withErrorHook('my-ctx', fn)).rejects.toThrow(error);
            expect(mockEvents.emitSafe).toHaveBeenCalledWith('on:error', {
                context: 'my-ctx',
                error,
            });
            expect(mockEvents.emitSafe).toHaveBeenCalledWith('on:detection', {
                type: 'error',
                details: 'boom',
            });
        });
    });
});
