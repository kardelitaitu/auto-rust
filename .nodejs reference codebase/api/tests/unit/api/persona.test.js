/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

import {
    setPersona,
    getPersona,
    getPersonaParam,
    getPersonaName,
    listPersonas,
    getSessionDuration,
    PERSONAS,
} from '@api/behaviors/persona.js';

describe('api/behaviors/persona.js', () => {
    beforeEach(() => {
        vi.restoreAllMocks();
    });

    afterEach(() => {
        vi.restoreAllMocks();
    });

    describe('setPersona', () => {
        it('should set valid persona', () => {
            setPersona('focused');
            expect(getPersonaName()).toBe('focused');
            expect(getPersona().speed).toBe(PERSONAS.focused.speed);
        });

        it('should throw on invalid persona', () => {
            expect(() => setPersona('invalid')).toThrow('Unknown persona');
        });

        it('should allow "custom" persona with overrides', () => {
            setPersona('custom', { speed: 5.0 });
            expect(getPersonaName()).toBe('custom');
            expect(getPersona().speed).toBe(5.0);
        });

        it('should apply overrides', () => {
            setPersona('casual', { speed: 99 });
            expect(getPersona().speed).toBe(99);
        });

        it('should apply biometric randomization', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.1);

            const baseKp = PERSONAS.casual.muscleModel.Kp;
            setPersona('casual');

            const newKp = getPersona().muscleModel.Kp;
            expect(newKp).not.toBe(baseKp);
            expect(newKp).toBeCloseTo(baseKp * 0.92, 4);

            vi.restoreAllMocks();
        });
    });

    describe('getPersona', () => {
        it('should return active persona config', () => {
            const persona = getPersona();
            expect(persona).toBeDefined();
            expect(persona.speed).toBeDefined();
        });
    });

    describe('getPersonaParam', () => {
        it('should return specific parameter', () => {
            expect(getPersonaParam('speed')).toBe(PERSONAS.casual.speed);
        });
    });

    describe('listPersonas', () => {
        it('should list all available personas', () => {
            const personas = listPersonas();
            expect(personas).toContain('casual');
            expect(personas).toContain('focused');
            expect(personas.length).toBeGreaterThan(10);
        });
    });

    describe('getSessionDuration', () => {
        it('should return positive duration', async () => {
            await new Promise((r) => setTimeout(r, 10));
            const duration = getSessionDuration();
            expect(duration).toBeGreaterThan(0);
        });
    });

    describe('PERSONAS constant', () => {
        it('should have all expected persona profiles', () => {
            const expected = [
                'casual',
                'efficient',
                'researcher',
                'power',
                'glitchy',
                'elderly',
                'teen',
                'professional',
                'gamer',
                'typer',
                'hesitant',
                'impulsive',
                'distracted',
                'focused',
                'newbie',
                'expert',
            ];
            expect(listPersonas()).toEqual(expected);
        });

        it('should have required properties for each persona', () => {
            for (const [name, persona] of Object.entries(PERSONAS)) {
                expect(persona).toHaveProperty('speed');
                expect(persona).toHaveProperty('hoverMin');
                expect(persona).toHaveProperty('hoverMax');
                expect(persona).toHaveProperty('typoRate');
                expect(persona).toHaveProperty('correctionRate');
                expect(persona).toHaveProperty('scrollSpeed');
                expect(persona).toHaveProperty('pathStyle');
                expect(persona).toHaveProperty('muscleModel');
                expect(persona.muscleModel).toHaveProperty('Kp');
                expect(persona.muscleModel).toHaveProperty('Ki');
                expect(persona.muscleModel).toHaveProperty('Kd');
            }
        });
    });
});
