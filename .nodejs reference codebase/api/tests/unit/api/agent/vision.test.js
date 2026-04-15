/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

vi.mock('@api/core/context.js', () => ({
    getPage: vi.fn(),
    getStateAgentElementMap: vi.fn().mockReturnValue([]),
}));

vi.mock('@api/tests/unit/api/utils/roi-detector.js', () => ({
    identifyROI: vi.fn().mockResolvedValue(null),
}));

vi.mock('sharp', () => {
    const mockResize = vi.fn().mockReturnThis();
    const mockJpeg = vi.fn().mockReturnThis();
    const mockToBuffer = vi.fn().mockResolvedValue(Buffer.from('compressed'));
    const mockMetadata = vi.fn().mockResolvedValue({ width: 1920, height: 1080 });

    return {
        default: vi.fn(() => ({
            resize: mockResize,
            jpeg: mockJpeg,
            toBuffer: mockToBuffer,
            metadata: mockMetadata,
        })),
    };
});

import { getPage, getStateAgentElementMap } from '@api/core/context.js';
import {
    buildPrompt,
    parseResponse,
    injectAnnotations,
    removeAnnotations,
    captureAXTree,
    captureState,
} from '@api/agent/vision.js';

describe('api/agent/vision.js', () => {
    let mockPage;

    beforeEach(() => {
        vi.clearAllMocks();

        mockPage = {
            accessibility: {
                snapshot: vi.fn().mockResolvedValue({
                    role: 'root',
                    name: 'Test Page',
                    children: [
                        { role: 'button', name: 'Click Me' },
                        { role: 'link', name: 'Go Here' },
                    ],
                }),
            },
            screenshot: vi.fn().mockResolvedValue(Buffer.from('fake-image')),
            evaluate: vi.fn().mockResolvedValue(undefined),
            url: vi.fn().mockReturnValue('https://example.com'),
            viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
        };

        getPage.mockReturnValue(mockPage);
    });

    describe('buildPrompt', () => {
        it('should build prompt with goal and semantic tree', () => {
            const context = {
                goal: 'Click the button',
                semanticTree: [
                    { id: '1', role: 'button', name: 'Click Me', coordinates: { x: 100, y: 200 } },
                ],
            };

            const prompt = buildPrompt(context);

            expect(prompt).toContain('Click the button');
            expect(prompt).toContain('button');
            expect(prompt).toContain('Click Me');
        });

        it('should handle empty semantic tree', () => {
            const context = {
                goal: 'Find something',
                semanticTree: [],
            };

            const prompt = buildPrompt(context);

            expect(prompt).toContain('Blind Mode');
        });

        it('should handle null semantic tree', () => {
            const context = {
                goal: 'Find something',
                semanticTree: null,
            };

            const prompt = buildPrompt(context);

            expect(prompt).toContain('Blind Mode');
        });

        it('should limit elements to 30', () => {
            const elements = Array.from({ length: 50 }, (_, i) => ({
                id: String(i),
                role: 'button',
                name: `Button ${i}`,
            }));

            const context = {
                goal: 'Test',
                semanticTree: elements,
            };

            const prompt = buildPrompt(context);

            expect(prompt).toContain('Button 0');
            expect(prompt).not.toContain('Button 40');
        });

        it('should handle elements without coordinates', () => {
            const context = {
                goal: 'Test',
                semanticTree: [{ id: '1', role: 'button', name: 'Button' }],
            };

            const prompt = buildPrompt(context);

            expect(prompt).toContain('(0,0)');
        });
    });

    describe('parseResponse', () => {
        it('should parse valid JSON response', () => {
            const raw =
                '{"thought": "I will click", "actions": [{"type": "click", "elementId": 1}]}';

            const result = parseResponse(raw);

            expect(result.success).toBe(true);
            expect(result.data.thought).toBe('I will click');
            expect(result.data.actions).toHaveLength(1);
        });

        it('should handle empty response', () => {
            const result = parseResponse('');

            expect(result.success).toBe(false);
            expect(result.error).toBe('Empty response');
        });

        it('should handle null response', () => {
            const result = parseResponse(null);

            expect(result.success).toBe(false);
        });

        it('should extract JSON from text with prefix', () => {
            const raw = 'Here is my plan: {"thought": "test", "actions": []}';

            const result = parseResponse(raw);

            expect(result.success).toBe(true);
        });

        it('should handle JSON parse error', () => {
            const raw = '{invalid json}';

            const result = parseResponse(raw);

            expect(result.success).toBe(false);
            expect(result.error).toContain('JSON parse error');
        });

        it('should require actions array', () => {
            const raw = '{"thought": "no actions"}';

            const result = parseResponse(raw);

            expect(result.success).toBe(false);
            expect(result.error).toContain('actions');
        });
    });

    describe('injectAnnotations', () => {
        it('should create annotation container', () => {
            const mockDoc = {
                createElement: vi.fn((tag) => ({
                    id: tag === 'div' ? 'agent-vision-annotations' : tag,
                    style: {},
                    appendChild: vi.fn(),
                })),
                body: {
                    appendChild: vi.fn(),
                },
                querySelector: vi.fn(),
            };

            injectAnnotations(mockDoc, []);

            expect(mockDoc.createElement).toHaveBeenCalledWith('div');
            expect(mockDoc.body.appendChild).toHaveBeenCalled();
        });

        it('should handle empty element map', () => {
            const mockDoc = {
                createElement: vi.fn(() => ({ style: {}, appendChild: vi.fn() })),
                body: { appendChild: vi.fn() },
                querySelector: vi.fn(),
            };

            injectAnnotations(mockDoc, []);

            expect(mockDoc.createElement).toHaveBeenCalled();
        });
    });

    describe('removeAnnotations', () => {
        it('should remove existing annotations', () => {
            const mockContainer = { remove: vi.fn() };
            const mockDoc = {
                getElementById: vi.fn().mockReturnValue(mockContainer),
            };

            removeAnnotations(mockDoc);

            expect(mockDoc.getElementById).toHaveBeenCalledWith('agent-vision-annotations');
            expect(mockContainer.remove).toHaveBeenCalled();
        });

        it('should handle missing container', () => {
            const mockDoc = {
                getElementById: vi.fn().mockReturnValue(null),
            };

            removeAnnotations(mockDoc);

            expect(mockDoc.getElementById).toHaveBeenCalled();
        });
    });

    describe('captureAXTree', () => {
        it('should capture accessibility tree', async () => {
            const result = await captureAXTree();

            expect(mockPage.accessibility.snapshot).toHaveBeenCalled();
            expect(result).toContain('root');
        });

        it('should simplify tree when option is true', async () => {
            const result = await captureAXTree({ simplified: true });

            expect(result).toBeDefined();
            const parsed = JSON.parse(result);
            expect(parsed).toHaveProperty('role');
        });

        it('should return raw tree when simplified is false', async () => {
            const result = await captureAXTree({ simplified: false });

            expect(result).toBeDefined();
        });

        it('should handle errors gracefully', async () => {
            mockPage.accessibility.snapshot.mockRejectedValue(new Error('Access denied'));

            const result = await captureAXTree();

            expect(result).toBe('');
        });
    });

    describe('captureState', () => {
        it('should capture full state with screenshot and axTree', async () => {
            const result = await captureState();

            expect(result).toHaveProperty('screenshot');
            expect(result).toHaveProperty('axTree');
            expect(result).toHaveProperty('url');
            expect(result.url).toBe('https://example.com');
        });

        it('should skip screenshot when disabled', async () => {
            const result = await captureState({ screenshot: false });

            expect(result.screenshot).toBe('');
            expect(mockPage.screenshot).not.toHaveBeenCalled();
        });

        it('should skip axTree when disabled', async () => {
            const result = await captureState({ axTree: false });

            expect(result.axTree).toBe('');
        });

        it('should handle screenshot errors', async () => {
            mockPage.screenshot.mockRejectedValue(new Error('Screenshot failed'));

            const result = await captureState();

            expect(result.screenshot).toBe('');
        });
    });
});
