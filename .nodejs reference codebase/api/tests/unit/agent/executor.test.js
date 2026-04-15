/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/executor.js
 * @module tests/unit/agent/executor.test
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock dependencies
vi.mock('@api/interactions/actions.js', () => ({
    click: vi.fn().mockResolvedValue(undefined),
    type: vi.fn().mockResolvedValue(undefined),
    hover: vi.fn().mockResolvedValue(undefined),
    drag: vi.fn().mockResolvedValue(undefined),
    clickAt: vi.fn().mockResolvedValue(undefined),
    multiSelect: vi.fn().mockResolvedValue(undefined),
    press: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@api/core/context-state.js', () => ({
    getStateAgentElementMap: vi.fn(() => [
        { id: 1, label: 'Button', selector: '#btn' },
        { id: 2, label: 'Input Field', selector: '#input' },
        { id: 3, label: 'Search Box', selector: '[data-testid="search"]' },
    ]),
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('api/agent/executor.js', () => {
    let doAction;

    beforeEach(async () => {
        vi.clearAllMocks();
        const module = await import('@api/agent/executor.js');
        doAction = module.doAction;
    });

    describe('doAction()', () => {
        describe('click action', () => {
            it('should click element by ID', async () => {
                const { click } = await import('@api/interactions/actions.js');
                await doAction('click', 1);
                expect(click).toHaveBeenCalledWith('#btn');
            });

            it('should click element by label (exact match)', async () => {
                const { click } = await import('@api/interactions/actions.js');
                await doAction('click', 'Button');
                expect(click).toHaveBeenCalledWith('#btn');
            });

            it('should click element by label (partial match)', async () => {
                const { click } = await import('@api/interactions/actions.js');
                await doAction('click', 'Input');
                expect(click).toHaveBeenCalledWith('#input');
            });

            it('should throw for unknown element', async () => {
                await expect(doAction('click', 'NonExistent')).rejects.toThrow('not found');
            });
        });

        describe('type/fill action', () => {
            it('should type into element by ID', async () => {
                const { type } = await import('@api/interactions/actions.js');
                await doAction('type', 2, 'Hello');
                expect(type).toHaveBeenCalledWith('#input', 'Hello');
            });

            it('should fill element (alias for type)', async () => {
                const { type } = await import('@api/interactions/actions.js');
                await doAction('fill', 2, 'World');
                expect(type).toHaveBeenCalledWith('#input', 'World');
            });
        });

        describe('hover action', () => {
            it('should hover over element', async () => {
                const { hover } = await import('@api/interactions/actions.js');
                await doAction('hover', 1);
                expect(hover).toHaveBeenCalledWith('#btn');
            });
        });

        describe('drag action', () => {
            it('should drag element to target', async () => {
                const { drag } = await import('@api/interactions/actions.js');
                await doAction('drag', 1, 2);
                expect(drag).toHaveBeenCalledWith('#btn', 2, {});
            });

            it('should throw if drag target is missing', async () => {
                await expect(doAction('drag', 1)).rejects.toThrow('requires a target');
            });
        });

        describe('clickAt action', () => {
            it('should click at coordinates', async () => {
                const { clickAt } = await import('@api/interactions/actions.js');
                await doAction('clickAt', { x: 100, y: 200 });
                expect(clickAt).toHaveBeenCalledWith(100, 200, undefined);
            });

            it('should clickAt with options', async () => {
                const { clickAt } = await import('@api/interactions/actions.js');
                await doAction('clickAt', { x: 50, y: 75 }, { delay: 100 });
                expect(clickAt).toHaveBeenCalledWith(50, 75, { delay: 100 });
            });

            it('should throw for invalid clickAt target', async () => {
                await expect(doAction('clickAt', { x: 100 })).rejects.toThrow('coordinates');
            });
        });

        describe('press action', () => {
            it('should press a key', async () => {
                const { press } = await import('@api/interactions/actions.js');
                await doAction('press', 'Enter');
                expect(press).toHaveBeenCalledWith('Enter', undefined);
            });

            it('should press key with options', async () => {
                const { press } = await import('@api/interactions/actions.js');
                await doAction('press', 'Enter', { delay: 100 });
                expect(press).toHaveBeenCalledWith('Enter', { delay: 100 });
            });

            it('should handle key alias', async () => {
                const { press } = await import('@api/interactions/actions.js');
                await doAction('key', 'Tab');
                expect(press).toHaveBeenCalledWith('Tab', undefined);
            });
        });

        describe('multiSelect action', () => {
            it('should select multiple elements', async () => {
                const { multiSelect } = await import('@api/interactions/actions.js');
                await doAction('multiSelect', [1, 2, 3], { mode: 'add' });
                expect(multiSelect).toHaveBeenCalledWith([1, 2, 3], { mode: 'add' });
            });

            it('should use empty options if not provided', async () => {
                const { multiSelect } = await import('@api/interactions/actions.js');
                await doAction('multiSelect', [1, 2]);
                expect(multiSelect).toHaveBeenCalledWith([1, 2], {});
            });

            it('should throw for non-array target', async () => {
                await expect(doAction('multiSelect', 1)).rejects.toThrow('array');
            });
        });

        describe('unsupported action', () => {
            it('should throw for unsupported action', async () => {
                await expect(doAction('unknown', 1)).rejects.toThrow('Unsupported');
            });
        });

        describe('case insensitivity', () => {
            it('should handle uppercase action names', async () => {
                const { click } = await import('@api/interactions/actions.js');
                await doAction('CLICK', 1);
                expect(click).toHaveBeenCalledWith('#btn');
            });
        });
    });
});
