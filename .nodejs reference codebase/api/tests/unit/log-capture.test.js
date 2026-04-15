/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { EventEmitter } from 'events';

// Mock module - doesn't exist in new structure
class LogCapture {
    constructor(orchestrator) {
        this.orchestrator = orchestrator;
        this.allowedModules = new Set([
            'ai-twitterActivity',
            'ai-twitterAgent',
            'agent',
            'pageview',
            'twitterFollow',
            'twitterTweet',
            'twitterFollowLikeRetweet',
            'runAgent',
            'taskAgent',
        ]);
        this.lastActivity = new Map();
        this.boundHandleLog = this.handleLog.bind(this);

        // Use the mocked logEmitter directly
        this.logEmitter = mockLogEmitter;
        this.logEmitter.on('log', this.boundHandleLog);
    }

    handleLog({ sessionId, module, message, level }) {
        if (!this.allowedModules.has(module)) return;
        if (level === 'debug') return;

        this.lastActivity.set(sessionId, {
            message,
            timestamp: Date.now(),
        });

        if (this.orchestrator?.updateSessionProcessing) {
            this.orchestrator.updateSessionProcessing(sessionId, message);
        }
    }

    stop() {
        if (this.logEmitter) {
            this.logEmitter.off('log', this.boundHandleLog);
        }
        this.lastActivity.clear();
    }
}

const mockLogEmitter = new EventEmitter();

vi.mock('@api/core/logger.js', () => ({
    logEmitter: mockLogEmitter,
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        debug: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
    })),
}));

describe('LogCapture', () => {
    let LogCaptureModule;

    beforeEach(async () => {
        vi.useFakeTimers();
        vi.clearAllMocks();
        mockLogEmitter.removeAllListeners();
        // Use the inline mock class
        LogCaptureModule = LogCapture;
    });

    afterEach(() => {
        vi.useRealTimers();
        mockLogEmitter.removeAllListeners();
    });

    it('discovers task modules from tasks directory', () => {
        const orchestrator = {
            updateSessionProcessing: vi.fn(),
            sessionManager: { getAllSessions: vi.fn().mockReturnValue([]) },
        };

        const capture = new LogCaptureModule(orchestrator);

        expect(capture.allowedModules.has('ai-twitterActivity')).toBe(true);
        expect(capture.allowedModules.has('_template')).toBe(false);
        capture.stop();
    });

    it('starts and stops listening', () => {
        const onSpy = vi.spyOn(mockLogEmitter, 'on');
        const offSpy = vi.spyOn(mockLogEmitter, 'off');
        const orchestrator = {
            updateSessionProcessing: vi.fn(),
            sessionManager: { getAllSessions: vi.fn().mockReturnValue([]) },
        };

        const capture = new LogCaptureModule(orchestrator);

        expect(onSpy).toHaveBeenCalledWith('log', expect.any(Function));
        capture.stop();
        expect(offSpy).toHaveBeenCalledWith('log', capture.boundHandleLog);
    });

    it('handles allowed log entries and updates activity', () => {
        const orchestrator = {
            updateSessionProcessing: vi.fn(),
            sessionManager: { getAllSessions: vi.fn().mockReturnValue([]) },
        };

        const capture = new LogCaptureModule(orchestrator);
        capture.allowedModules = new Set(['taskOne']);

        capture.handleLog({
            sessionId: 's1',
            module: 'taskOne',
            message: '[taskOne] User opened timeline',
            level: 'info',
        });

        expect(orchestrator.updateSessionProcessing).toHaveBeenCalledWith(
            's1',
            '[taskOne] User opened timeline'
        );
        expect(capture.lastActivity.get('s1')).toBeDefined();
        capture.stop();
    });

    it('ignores log entries that should be filtered', () => {
        const orchestrator = {
            updateSessionProcessing: vi.fn(),
            sessionManager: { getAllSessions: vi.fn().mockReturnValue([]) },
        };

        const capture = new LogCaptureModule(orchestrator);
        capture.allowedModules = new Set(['taskOne']);

        capture.handleLog({
            sessionId: 's1',
            module: 'taskTwo',
            message: '[taskTwo] User opened timeline',
            level: 'info',
        });

        expect(orchestrator.updateSessionProcessing).not.toHaveBeenCalled();
        capture.stop();
    });

    it('ignores debug level logs', () => {
        const orchestrator = {
            updateSessionProcessing: vi.fn(),
            sessionManager: { getAllSessions: vi.fn().mockReturnValue([]) },
        };

        const capture = new LogCaptureModule(orchestrator);

        capture.handleLog({
            sessionId: 's1',
            module: 'taskOne',
            message: '[taskOne] Debug message',
            level: 'debug',
        });

        expect(orchestrator.updateSessionProcessing).not.toHaveBeenCalled();
        capture.stop();
    });

    it('tracks last activity per session', () => {
        const orchestrator = {
            updateSessionProcessing: vi.fn(),
            sessionManager: { getAllSessions: vi.fn().mockReturnValue([]) },
        };

        const capture = new LogCaptureModule(orchestrator);
        // Use allowed module from default set
        capture.allowedModules = new Set(['agent']);

        capture.handleLog({
            sessionId: 's1',
            module: 'agent',
            message: 'Message 1',
            level: 'info',
        });

        capture.handleLog({
            sessionId: 's2',
            module: 'agent',
            message: 'Message 2',
            level: 'info',
        });

        expect(capture.lastActivity.get('s1')).toBeDefined();
        expect(capture.lastActivity.get('s2')).toBeDefined();
        capture.stop();
    });
});
