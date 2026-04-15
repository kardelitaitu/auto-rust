/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
    ValenceAnalyzer,
    ArousalAnalyzer,
    DominanceAnalyzer,
    SarcasmAnalyzer,
    UrgencyAnalyzer,
    ToxicityAnalyzer,
} from '@api/utils/sentiment-analyzers.js';

describe('sentiment-analyzers.js', () => {
    describe('ValenceAnalyzer', () => {
        let analyzer;

        beforeEach(() => {
            analyzer = new ValenceAnalyzer();
        });

        it('should analyze positive text', () => {
            const result = analyzer.analyze('This is amazing and wonderful!');
            expect(result.valence).toBeGreaterThan(0);
        });

        it('should analyze negative text', () => {
            const result = analyzer.analyze('This is terrible and horrible');
            expect(result.valence).toBeLessThan(0);
        });

        it('should analyze neutral text', () => {
            const result = analyzer.analyze('This is a statement');
            expect(result.valence).toBe(0);
        });

        it('should return confidence level', () => {
            const result = analyzer.analyze('love hate love');
            expect(result.confidence).toBeDefined();
        });

        it('should detect positive and negative matches', () => {
            const result = analyzer.analyze('I love this but hate that');
            expect(result.positiveMatches).toBeGreaterThan(0);
            expect(result.negativeMatches).toBeGreaterThan(0);
        });
    });

    describe('ArousalAnalyzer', () => {
        let analyzer;

        beforeEach(() => {
            analyzer = new ArousalAnalyzer();
        });

        it('should detect high arousal', () => {
            const result = analyzer.analyze('OMG THIS IS AMAZING!!!');
            expect(result.arousal).toBeGreaterThan(0.5);
        });

        it('should detect low arousal', () => {
            const result = analyzer.analyze('okay i guess');
            expect(result.arousal).toBeLessThan(0.5);
        });

        it('should count exclamation marks', () => {
            const result = analyzer.analyze('Wow!!! Really???');
            expect(result.exclamations).toBe(3);
        });

        it('should detect caps words', () => {
            const result = analyzer.analyze('This is REALLY exciting');
            expect(result.capsWords).toBeGreaterThan(0);
        });

        it('should detect repetitions', () => {
            const result = analyzer.analyze('sooooo happy');
            expect(result.repetitions).toBeGreaterThan(0);
        });
    });

    describe('DominanceAnalyzer', () => {
        let analyzer;

        beforeEach(() => {
            analyzer = new DominanceAnalyzer();
        });

        it('should detect assertive text', () => {
            const result = analyzer.analyze('I will definitely do this. It is required.');
            expect(result.dominance).toBeGreaterThan(0.5);
        });

        it('should detect submissive text', () => {
            const result = analyzer.analyze('Could you maybe consider perhaps?');
            expect(result.dominance).toBeLessThan(0.5);
        });

        it('should count questions', () => {
            const result = analyzer.analyze('What is this? How does it work? Why?');
            expect(result.questions).toBe(3);
        });

        it('should count statements', () => {
            const result = analyzer.analyze('This is a statement. Another one. And another.');
            expect(result.statements).toBe(3);
        });
    });

    describe('SarcasmAnalyzer', () => {
        let analyzer;

        beforeEach(() => {
            analyzer = new SarcasmAnalyzer();
        });

        it('should detect sarcasm markers', () => {
            const result = analyzer.analyze('Oh great, another meeting');
            expect(result.sarcasm).toBeGreaterThan(0);
        });

        it('should detect sarcastic emojis', () => {
            const result = analyzer.analyze('Yeah right 🙄');
            expect(result.sarcasm).toBeGreaterThan(0);
        });

        it('should detect contradictory phrasing', () => {
            const result = analyzer.analyze('What a wonderful disaster');
            expect(result.sarcasm).toBeGreaterThan(0);
        });

        it('should return confidence level', () => {
            const result = analyzer.analyze('Sure, totally not biased');
            expect(result.confidence).toBeDefined();
        });
    });

    describe('UrgencyAnalyzer', () => {
        let analyzer;

        beforeEach(() => {
            analyzer = new UrgencyAnalyzer();
        });

        it('should detect urgent text', () => {
            const result = analyzer.analyze('URGENT: Must respond NOW!!!');
            expect(result.urgency).toBeGreaterThan(0.5);
        });

        it('should detect relaxed text', () => {
            const result = analyzer.analyze('Whenever you have time, no rush');
            expect(result.urgency).toBeLessThan(0.5);
        });

        it('should detect time-sensitive markers', () => {
            const result = analyzer.analyze('Deadline is tomorrow at 5pm');
            expect(result.urgency).toBeGreaterThan(0);
        });
    });

    describe('ToxicityAnalyzer', () => {
        let analyzer;

        beforeEach(() => {
            analyzer = new ToxicityAnalyzer();
        });

        it('should detect toxic text', () => {
            const result = analyzer.analyze('You are such an idiot and suck');
            expect(result.toxicity).toBeGreaterThan(0);
        });

        it('should detect profanity', () => {
            const result = analyzer.analyze('What the fuck is this');
            expect(result.toxicity).toBeGreaterThan(0);
        });

        it('should return severity level', () => {
            const result = analyzer.analyze('This is terrible and awful');
            expect(result.severityLevel).toBeDefined();
        });

        it('should detect slurs and insults', () => {
            const result = analyzer.analyze('You are stupid and dumb');
            expect(result.toxicity).toBeGreaterThan(0);
        });
    });
});
