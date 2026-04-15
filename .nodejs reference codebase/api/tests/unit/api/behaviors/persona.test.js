/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

import {
    PERSONAS,
    setPersona,
    getPersona,
    getPersonaParam,
    getPersonaName,
    listPersonas,
    getSessionDuration,
} from '@api/behaviors/persona.js';

describe('api/behaviors/persona.js', () => {
    beforeEach(() => {
        vi.spyOn(Math, 'random').mockReturnValue(0.5);
    });

    afterEach(() => {
        setPersona('casual');
        vi.restoreAllMocks();
    });

    describe('PERSONAS', () => {
        it('should have all required personas', () => {
            expect(PERSONAS.casual).toBeDefined();
            expect(PERSONAS.efficient).toBeDefined();
            expect(PERSONAS.researcher).toBeDefined();
            expect(PERSONAS.power).toBeDefined();
            expect(PERSONAS.glitchy).toBeDefined();
            expect(PERSONAS.elderly).toBeDefined();
            expect(PERSONAS.teen).toBeDefined();
            expect(PERSONAS.professional).toBeDefined();
            expect(PERSONAS.gamer).toBeDefined();
            expect(PERSONAS.typer).toBeDefined();
            expect(PERSONAS.hesitant).toBeDefined();
            expect(PERSONAS.impulsive).toBeDefined();
            expect(PERSONAS.distracted).toBeDefined();
            expect(PERSONAS.focused).toBeDefined();
            expect(PERSONAS.newbie).toBeDefined();
            expect(PERSONAS.expert).toBeDefined();
        });

        it('should have all required properties for each persona', () => {
            const requiredProps = [
                'speed',
                'hoverMin',
                'hoverMax',
                'typoRate',
                'correctionRate',
                'hesitation',
                'hesitationDelay',
                'scrollSpeed',
                'clickHold',
                'pathStyle',
                'microMoveChance',
                'idleChance',
                'muscleModel',
            ];

            for (const [name, persona] of Object.entries(PERSONAS)) {
                for (const prop of requiredProps) {
                    expect(persona).toHaveProperty(
                        prop,
                        expect.anything(),
                        `Persona ${name} should have ${prop}`
                    );
                }
            }
        });

        it('should have valid muscleModel for each persona', () => {
            for (const persona of Object.values(PERSONAS)) {
                expect(persona.muscleModel).toHaveProperty('Kp');
                expect(persona.muscleModel).toHaveProperty('Ki');
                expect(persona.muscleModel).toHaveProperty('Kd');
            }
        });
    });

    describe('setPersona', () => {
        it('should set persona by name', () => {
            setPersona('power');

            expect(getPersonaName()).toBe('power');
        });

        it('should throw for unknown persona', () => {
            expect(() => setPersona('invalid')).toThrow('Unknown persona');
        });

        it('should allow custom persona', () => {
            setPersona('custom', { speed: 2.5, hoverMin: 50 });

            const persona = getPersona();
            expect(persona.speed).toBe(2.5);
            expect(persona.hoverMin).toBe(50);
        });

        it('should apply overrides', () => {
            setPersona('casual', { speed: 3.0 });

            const persona = getPersona();
            expect(persona.speed).toBe(3.0);
        });

        it('should apply muscleModel drift', () => {
            const originalKp = getPersona().muscleModel.Kp;

            setPersona('casual');

            const newPersona = getPersona();
            expect(newPersona.muscleModel.Kp).toBeDefined();
        });
    });

    describe('getPersona', () => {
        it('should return current persona', () => {
            setPersona('efficient');

            const persona = getPersona();

            expect(persona).toHaveProperty('speed');
            expect(persona.speed).toBe(PERSONAS.efficient.speed);
        });
    });

    describe('getPersonaParam', () => {
        it('should return specific parameter', () => {
            setPersona('power');

            const speed = getPersonaParam('speed');
            expect(speed).toBe(PERSONAS.power.speed);
        });

        it('should return undefined for unknown param', () => {
            const value = getPersonaParam('nonexistent');
            expect(value).toBeUndefined();
        });
    });

    describe('getPersonaName', () => {
        it('should return current persona name', () => {
            setPersona('researcher');

            expect(getPersonaName()).toBe('researcher');
        });
    });

    describe('listPersonas', () => {
        it('should return all persona names', () => {
            const names = listPersonas();

            expect(Array.isArray(names)).toBe(true);
            expect(names).toContain('casual');
            expect(names).toContain('power');
            expect(names).toContain('glitchy');
            expect(names.length).toBeGreaterThan(10);
        });
    });

    describe('getSessionDuration', () => {
        it('should return positive number', () => {
            const duration = getSessionDuration();

            expect(typeof duration).toBe('number');
            expect(duration).toBeGreaterThanOrEqual(0);
        });

        it('should increase over time', async () => {
            const start = getSessionDuration();
            await new Promise((r) => setTimeout(r, 10));
            const end = getSessionDuration();

            expect(end).toBeGreaterThan(start);
        });
    });
});
