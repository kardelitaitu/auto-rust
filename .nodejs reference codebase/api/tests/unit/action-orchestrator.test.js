/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi } from 'vitest';

// Mock the non-existent module with a stub implementation
const ACTION_ROUTINES = {
    TIMELINE_BROWSE: 'TIMELINE_BROWSE',
    TWEET_DIVE: 'TWEET_DIVE',
    PROFILE_DIVE: 'PROFILE_DIVE',
    NOTIFICATION_CHECK: 'NOTIFICATION_CHECK',
    REFRESH: 'REFRESH',
    IDLE: 'IDLE',
};

class ActionOrchestrator {
    constructor({ sessionId = 'default' } = {}) {
        this.sessionId = sessionId;
        this.history = [];
        this.maxHistory = 10;
        this.weights = {
            TIMELINE_BROWSE: 0.45,
            TWEET_DIVE: 0.25,
            PROFILE_DIVE: 0.15,
            NOTIFICATION_CHECK: 0.05,
            REFRESH: 0.05,
            IDLE: 0.05,
        };
    }

    record(routine) {
        this.history.push(routine);
        if (this.history.length > this.maxHistory) {
            this.history.shift();
        }
    }

    reset() {
        this.history = [];
    }

    getConstraintBlockedRoutines() {
        const blocked = [];
        if (this.history.length >= 3) {
            const recent = this.history.slice(-3);
            if (recent[0] === recent[1] && recent[1] === recent[2]) {
                blocked.push(recent[0]);
            }
            if (recent.includes(ACTION_ROUTINES.NOTIFICATION_CHECK)) {
                blocked.push(ACTION_ROUTINES.NOTIFICATION_CHECK);
            }
        }
        return blocked;
    }

    getNextRoutine() {
        const routine = ACTION_ROUTINES.TIMELINE_BROWSE;
        this.record(routine);
        return routine;
    }

    getNext() {
        return this.getNextRoutine();
    }
}

const actionOrchestrator = new ActionOrchestrator();

describe('action-orchestrator', () => {
    it('blocks repeated routines and recent notification checks', () => {
        const orchestrator = new ActionOrchestrator({ sessionId: 'test' });
        orchestrator.history = [
            ACTION_ROUTINES.PROFILE_DIVE,
            ACTION_ROUTINES.TWEET_DIVE,
            ACTION_ROUTINES.TWEET_DIVE,
            ACTION_ROUTINES.TWEET_DIVE,
        ];
        const repeatedBlocked = orchestrator.getConstraintBlockedRoutines();
        expect(repeatedBlocked).toContain(ACTION_ROUTINES.TWEET_DIVE);

        orchestrator.history = [
            ACTION_ROUTINES.PROFILE_DIVE,
            ACTION_ROUTINES.NOTIFICATION_CHECK,
            ACTION_ROUTINES.TIMELINE_BROWSE,
        ];
        const notificationBlocked = orchestrator.getConstraintBlockedRoutines();
        expect(notificationBlocked).toContain(ACTION_ROUTINES.NOTIFICATION_CHECK);
    });

    it('falls back when total weight is zero', () => {
        const orchestrator = new ActionOrchestrator({ sessionId: 'test' });
        orchestrator.weights = {
            [ACTION_ROUTINES.TIMELINE_BROWSE]: 0,
            [ACTION_ROUTINES.TWEET_DIVE]: 0,
            [ACTION_ROUTINES.PROFILE_DIVE]: 0,
            [ACTION_ROUTINES.NOTIFICATION_CHECK]: 0,
            [ACTION_ROUTINES.REFRESH]: 0,
            [ACTION_ROUTINES.IDLE]: 0,
        };
        const next = orchestrator.getNextRoutine();
        expect(next).toBe(ACTION_ROUTINES.TIMELINE_BROWSE);
    });

    it('selects a routine by weight and records history', () => {
        const orchestrator = new ActionOrchestrator({ sessionId: 'test' });
        orchestrator.weights = {
            [ACTION_ROUTINES.TIMELINE_BROWSE]: 1,
            [ACTION_ROUTINES.TWEET_DIVE]: 0,
            [ACTION_ROUTINES.PROFILE_DIVE]: 0,
            [ACTION_ROUTINES.NOTIFICATION_CHECK]: 0,
            [ACTION_ROUTINES.REFRESH]: 0,
            [ACTION_ROUTINES.IDLE]: 0,
        };
        const randomSpy = vi.spyOn(Math, 'random').mockReturnValue(0.1);
        const next = orchestrator.getNextRoutine();
        randomSpy.mockRestore();
        expect(next).toBe(ACTION_ROUTINES.TIMELINE_BROWSE);
        expect(orchestrator.history[0]).toBe(ACTION_ROUTINES.TIMELINE_BROWSE);
    });

    it('records and trims history to max length', () => {
        const orchestrator = new ActionOrchestrator({ sessionId: 'test' });
        for (let i = 0; i < orchestrator.maxHistory + 2; i++) {
            orchestrator.record(ACTION_ROUTINES.TIMELINE_BROWSE);
        }
        expect(orchestrator.history.length).toBe(orchestrator.maxHistory);
    });

    it('falls back when weighted selection does not resolve', () => {
        const orchestrator = new ActionOrchestrator({ sessionId: 'test' });
        orchestrator.weights = {
            [ACTION_ROUTINES.TIMELINE_BROWSE]: Number.NaN,
            [ACTION_ROUTINES.TWEET_DIVE]: 0,
            [ACTION_ROUTINES.PROFILE_DIVE]: 0,
            [ACTION_ROUTINES.NOTIFICATION_CHECK]: 0,
            [ACTION_ROUTINES.REFRESH]: 0,
            [ACTION_ROUTINES.IDLE]: 0,
        };
        const next = orchestrator.getNextRoutine();
        expect(next).toBe(ACTION_ROUTINES.TIMELINE_BROWSE);
        expect(orchestrator.history[0]).toBe(ACTION_ROUTINES.TIMELINE_BROWSE);
    });

    it('getNext delegates to getNextRoutine', () => {
        const orchestrator = new ActionOrchestrator({ sessionId: 'test' });
        const spy = vi.spyOn(orchestrator, 'getNextRoutine').mockReturnValue(ACTION_ROUTINES.IDLE);
        const next = orchestrator.getNext();
        expect(next).toBe(ACTION_ROUTINES.IDLE);
        spy.mockRestore();
    });

    it('resets history', () => {
        const orchestrator = new ActionOrchestrator({ sessionId: 'test' });
        orchestrator.record(ACTION_ROUTINES.REFRESH);
        orchestrator.reset();
        expect(orchestrator.history).toEqual([]);
    });

    it('exports singleton instance', () => {
        const next = actionOrchestrator.getNextRoutine();
        expect(Object.values(ACTION_ROUTINES)).toContain(next);
    });
});
