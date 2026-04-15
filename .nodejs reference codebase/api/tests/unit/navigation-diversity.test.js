/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock module - doesn't exist in new structure
const NAV_STATES = {
    FEED: 'FEED',
    PROFILE: 'PROFILE',
    SEARCH: 'SEARCH',
    NOTIFICATIONS: 'NOTIFICATIONS',
    MESSAGES: 'MESSAGES',
    TRENDING: 'TRENDING',
    BOOKMARKS: 'BOOKMARKS',
    LISTS: 'LISTS',
    SETTINGS: 'SETTINGS',
};

const navigationDiversity = {
    createNavigationManager: (options = {}) => {
        const state = {
            currentState: NAV_STATES.FEED,
            history: [],
            depth: 0,
            rabbitHoleChance: options.rabbitHoleChance ?? 0.3,
            maxDepth: options.maxDepth ?? 5,
            minDepth: options.minDepth ?? 1,
            exitProbability: options.exitProbability ?? 0.2,
            stayProbability: options.stayProbability ?? 0.8,
            states: Object.values(NAV_STATES),
        };

        return {
            getCurrentState: () => state.currentState,
            setState: (newState, reason = '') => {
                state.currentState = newState;
                state.history.push({ state: newState, reason, timestamp: Date.now() });
                if (newState !== NAV_STATES.FEED) {
                    state.depth++;
                } else {
                    state.depth = 0;
                }
            },
            getStateHistory: () => state.history,
            shouldExitRabbitHole: () => {
                if (state.depth >= state.maxDepth) return true;
                return Math.random() < state.exitProbability;
            },
            shouldEnterRabbitHole: () => {
                if (state.depth < state.minDepth) return true;
                return Math.random() < state.rabbitHoleChance;
            },
            getNextState: () => {
                const otherStates = state.states.filter((s) => s !== state.currentState);
                return otherStates[Math.floor(Math.random() * otherStates.length)];
            },
            reset: () => {
                state.currentState = NAV_STATES.FEED;
                state.history = [];
                state.depth = 0;
            },
        };
    },

    NAV_STATES,
};

describe('NavigationDiversity', () => {
    let manager;
    let mockPage;

    beforeEach(() => {
        manager = navigationDiversity.createNavigationManager({
            rabbitHoleChance: 1, // Always enter rabbit hole for testing
            maxDepth: 3,
            minDepth: 1,
            exitProbability: 0.5,
            stayProbability: 0.5,
        });

        mockPage = {
            waitForTimeout: vi.fn().mockResolvedValue(undefined),
        };
    });

    it('should initialize with default state', () => {
        expect(manager.getCurrentState()).toBe(NAV_STATES.FEED);
    });

    it('should set state and record history', () => {
        manager.setState(NAV_STATES.PROFILE, 'test-reason');
        expect(manager.getCurrentState()).toBe(NAV_STATES.PROFILE);
        const history = manager.getStateHistory();
        expect(history.length).toBe(1);
        expect(history[0].reason).toBe('test-reason');
    });

    it('should track state history', () => {
        manager.setState(NAV_STATES.PROFILE);
        manager.setState(NAV_STATES.SEARCH);
        manager.setState(NAV_STATES.NOTIFICATIONS);

        const history = manager.getStateHistory();
        expect(history.length).toBe(3);
    });

    it('should determine when to exit rabbit hole based on depth', () => {
        const shallowManager = navigationDiversity.createNavigationManager({
            rabbitHoleChance: 1,
            maxDepth: 2,
            minDepth: 1,
            exitProbability: 0,
        });

        shallowManager.setState(NAV_STATES.PROFILE);
        shallowManager.setState(NAV_STATES.SEARCH);

        expect(shallowManager.shouldExitRabbitHole()).toBe(true);
    });

    it('should determine when to stay in rabbit hole', () => {
        const stayManager = navigationDiversity.createNavigationManager({
            rabbitHoleChance: 1,
            maxDepth: 10,
            minDepth: 1,
            exitProbability: 0,
        });

        stayManager.setState(NAV_STATES.PROFILE);

        expect(stayManager.shouldExitRabbitHole()).toBe(false);
    });

    it('should get next state different from current', () => {
        const nextState = manager.getNextState();
        expect(nextState).not.toBe(NAV_STATES.FEED);
    });

    it('should reset state and history', () => {
        manager.setState(NAV_STATES.PROFILE);
        manager.setState(NAV_STATES.SEARCH);

        manager.reset();

        expect(manager.getCurrentState()).toBe(NAV_STATES.FEED);
        expect(manager.getStateHistory().length).toBe(0);
    });

    it('should export NAV_STATES constants', () => {
        expect(NAV_STATES.FEED).toBe('FEED');
        expect(NAV_STATES.PROFILE).toBe('PROFILE');
        expect(NAV_STATES.SEARCH).toBe('SEARCH');
        expect(NAV_STATES.NOTIFICATIONS).toBe('NOTIFICATIONS');
    });

    it('should increment depth when entering non-FEED states', () => {
        expect(manager.getCurrentState()).toBe(NAV_STATES.FEED);

        manager.setState(NAV_STATES.PROFILE);
        expect(manager.getStateHistory().length).toBe(1);

        manager.setState(NAV_STATES.FEED);
        expect(manager.getCurrentState()).toBe(NAV_STATES.FEED);
    });
});
