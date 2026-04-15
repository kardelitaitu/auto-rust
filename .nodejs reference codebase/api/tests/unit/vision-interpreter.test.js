/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import VisionInterpreter from '@api/core/vision-interpreter.js';

// Mock Logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn().mockReturnValue({
        info: vi.fn(),
        debug: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
    }),
}));

describe('VisionInterpreter', () => {
    let interpreter;

    beforeEach(() => {
        interpreter = new VisionInterpreter();
    });

    describe('buildPrompt', () => {
        it('should include the goal in the prompt', () => {
            const context = { goal: 'test goal', semanticTree: [] };
            const prompt = interpreter.buildPrompt(context);
            expect(prompt).toContain('User Goal: "test goal"');
        });

        it('should handle empty semantic tree with Blind Mode message', () => {
            const context = { goal: 'test goal', semanticTree: [] };
            const prompt = interpreter.buildPrompt(context);
            expect(prompt).toContain('No interactive elements detected (Blind Mode).');
        });

        it('should handle missing semantic tree in context', () => {
            const context = { goal: 'test goal' };
            const prompt = interpreter.buildPrompt(context);
            expect(prompt).toContain('No interactive elements detected (Blind Mode).');
        });

        it('should format elements correctly in the prompt', () => {
            const context = {
                goal: 'test goal',
                semanticTree: [
                    { name: 'Button 1', role: 'button', coordinates: { x: 10, y: 20 } },
                    { text: 'Link 1', coordinates: { x: 30, y: 40 } },
                    { accessibilityId: 'input-1', role: 'textbox' },
                    {}, // Unknown element
                ],
            };
            const prompt = interpreter.buildPrompt(context);
            expect(prompt).toContain('0. [button] "Button 1" @ (10,20)');
            expect(prompt).toContain('1. [element] "Link 1" @ (30,40)');
            expect(prompt).toContain('2. [textbox] "input-1" @ (0,0)');
            expect(prompt).toContain('3. [element] "Unknown" @ (0,0)');
        });

        it('should limit to top 30 elements', () => {
            const semanticTree = Array.from({ length: 50 }, (_, i) => ({ name: `El ${i}` }));
            const context = { goal: 'test goal', semanticTree };
            const prompt = interpreter.buildPrompt(context);
            expect(prompt).toContain('29. [element] "El 29"');
            expect(prompt).not.toContain('30. [element] "El 30"');
        });
    });

    describe('parseResponse', () => {
        it('should handle empty response', () => {
            const result = interpreter.parseResponse('');
            expect(result.success).toBe(false);
            expect(result.error).toBe('Empty response');
        });

        it('should parse JSON from markdown blocks', () => {
            const rawText = 'Here is the plan:\n```json\n{"actions": [{"type": "click"}]}\n```';
            const result = interpreter.parseResponse(rawText);
            expect(result.success).toBe(true);
            expect(result.data.actions[0].type).toBe('click');
        });

        it('should parse raw JSON object from text', () => {
            const rawText = 'Just the JSON: {"actions": [{"type": "wait"}]}';
            const result = interpreter.parseResponse(rawText);
            expect(result.success).toBe(true);
            expect(result.data.actions[0].type).toBe('wait');
        });

        it('should handle no JSON found', () => {
            const result = interpreter.parseResponse('No JSON here!');
            expect(result.success).toBe(false);
            expect(result.error).toBe('No JSON found in response');
        });

        it('should validate JSON structure (missing actions)', () => {
            const rawText = '{"thought": "nothing"}';
            const result = interpreter.parseResponse(rawText);
            expect(result.success).toBe(false);
            expect(result.error).toContain("missing 'actions' array");
        });

        it('should handle JSON parse errors', () => {
            const rawText = '{"actions": [invalid]}';
            const result = interpreter.parseResponse(rawText);
            expect(result.success).toBe(false);
            expect(result.error).toContain('JSON parse error');
        });
    });
});
