/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit Tests for AITwitterAgent - Real Implementation
 * Tests the actual AITwitterAgent class with mocked dependencies
 * @module tests/unit/ai-twitterAgent.real.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { api } from '@api/index.js';
import { AITwitterAgent } from '@api/twitter/ai-twitterAgent.js';

// ============================================================================
// MOCK DEPENDENCIES
// ============================================================================

// Hoisted mocks for spies
const contextEngineMocks = vi.hoisted(() => ({
    extractEnhancedContext: vi.fn().mockResolvedValue({
        sentiment: { overall: 'positive' },
        tone: { primary: 'casual' },
        engagementLevel: 'medium',
        replies: [],
    }),
}));

const replyEngineMocks = vi.hoisted(() => ({
    shouldReply: vi.fn().mockResolvedValue({ decision: 'reply', reply: 'Test reply' }),
    generateReply: vi.fn().mockResolvedValue({ success: true, reply: 'Test reply' }),
    executeReply: vi.fn().mockResolvedValue({ success: true, method: 'test' }),
}));

// Mock TwitterAgent parent class
vi.mock('@api/twitter/twitterAgent.js', () => ({
    TwitterAgent: class {
        constructor(page, profile, logger) {
            this.page = page;
            this.config = profile;
            this.profile = profile;
            this.logger = logger;
            this.state = {
                consecutiveLoginFailures: 0,
                replies: 0,
                quotes: 0,
                activityMode: 'NORMAL',
                burstEndTime: 0,
                lastRefreshAt: 0,
            };
            this.sessionStart = Date.now();
            this.loopIndex = 0;
            this.ghost = { click: vi.fn().mockResolvedValue({ success: true }) };
            this.human = {
                sessionStart: vi.fn(),
                sessionEnd: vi.fn(),
                cycleComplete: vi.fn(),
                session: {
                    shouldEndSession: vi.fn().mockReturnValue(false),
                    boredomPause: vi.fn(),
                },
                think: vi.fn().mockResolvedValue(),
                recoverFromError: vi.fn().mockResolvedValue(),
            };
            this.navigation = { navigateHome: vi.fn().mockResolvedValue() };
            this.engagement = { diveTweet: vi.fn() };
            this.session = { updatePhase: vi.fn() };
            this.log = vi.fn().mockImplementation((msg) => {
                if (this.logger && this.logger.info) this.logger.info(msg);
            });
        }
        isSessionExpired() {
            return false;
        }
        simulateReading() {
            return Promise.resolve();
        }
        navigateHome() {
            return Promise.resolve();
        }
        checkLoginState() {
            return Promise.resolve(true);
        }
        normalizeProbabilities(p) {
            return p;
        }
        diveProfile() {
            return Promise.resolve();
        }
        shutdown() {}
    },
}));

// Mock Engines
vi.mock('@api/agent/ai-reply-engine/index.js', () => {
    const AIReplyEngine = vi.fn(function () {
        this.config = { REPLY_PROBABILITY: 0.5 };
        this.shouldReply = replyEngineMocks.shouldReply;
        this.generateReply = replyEngineMocks.generateReply;
        this.executeReply = replyEngineMocks.executeReply;
    });
    return { AIReplyEngine };
});

vi.mock('@api/agent/ai-quote-engine.js', () => ({
    AIQuoteEngine: vi.fn(function () {
        return {
            generateQuote: vi.fn().mockResolvedValue({ success: true, quote: 'Test quote' }),
            executeQuote: vi.fn().mockResolvedValue({ success: true, method: 'test' }),
        };
    }),
}));

vi.mock('@api/agent/ai-context-engine.js', () => {
    const AIContextEngine = vi.fn(function () {
        this.extractEnhancedContext = contextEngineMocks.extractEnhancedContext;
    });
    return { AIContextEngine };
});

// Mock Utils
vi.mock('@api/behaviors/micro-interactions.js', () => ({
    microInteractions: {
        createMicroInteractionHandler: vi.fn().mockReturnValue({
            config: {},
            executeMicroInteraction: vi.fn().mockResolvedValue({ success: true, type: 'test' }),
            textHighlight: vi.fn().mockResolvedValue({ success: true }),
            startFidgetLoop: vi.fn(),
            stopFidgetLoop: vi.fn(),
        }),
    },
}));

vi.mock('@api/behaviors/motor-control.js', () => ({
    motorControl: {
        createMotorController: vi.fn().mockReturnValue({
            smartClick: vi.fn().mockResolvedValue({ success: true, x: 100, y: 100 }),
        }),
    },
}));

vi.mock('@api/core/agent-connector.js', () => ({
    default: vi.fn(),
}));

vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        randomInRange: vi.fn((min, max) => min),
        roll: vi.fn(() => true),
        gaussian: vi.fn((mean) => mean),
        sample: vi.fn((arr) => arr?.[0] || null),
    },
}));

vi.mock('@api/utils/entropyController.js', () => ({
    entropy: {
        retryDelay: vi.fn(() => 100),
        scrollSettleTime: vi.fn(() => 100),
    },
}));

vi.mock('@api/utils/engagement-limits.js', () => ({
    engagementLimits: {
        createEngagementTracker: vi.fn().mockReturnValue({
            canPerform: vi.fn().mockReturnValue(true),
            record: vi.fn().mockReturnValue(true),
            getProgress: vi.fn().mockReturnValue('0/10'),
            getStatus: vi.fn().mockReturnValue({}),
            getSummary: vi.fn().mockReturnValue('Summary'),
            getUsageRate: vi.fn().mockReturnValue(0),
        }),
    },
}));

vi.mock('@api/twitter/session-phases.js', () => ({
    sessionPhases: {
        getSessionPhase: vi.fn().mockReturnValue('active'),
        getPhaseStats: vi.fn().mockReturnValue({ description: 'Active phase' }),
        getPhaseModifier: vi.fn().mockReturnValue(1.0),
    },
}));

vi.mock('@api/utils/sentiment-service.js', () => ({
    sentimentService: {
        analyze: vi.fn().mockReturnValue({
            isNegative: false,
            score: 0.8,
            dimensions: {
                valence: { valence: 0.8 },
                arousal: { arousal: 0.5 },
                dominance: { dominance: 0.5 },
                sarcasm: { sarcasm: 0.1 },
                toxicity: { toxicity: 0 },
            },
            engagement: { warnings: [], canLike: true },
            composite: {
                riskLevel: 'low',
                conversationType: 'casual',
                engagementStyle: 'positive',
            },
        }),
    },
}));

vi.mock('@api/behaviors/scroll-helper.js', () => ({
    scrollDown: vi.fn(),
    scrollUp: vi.fn(),
    scrollRandom: vi.fn(),
}));

vi.mock('@api/utils/config-service.js', () => ({
    config: {
        getEngagementLimits: vi.fn(),
    },
}));

vi.mock('@api/utils/async-queue.js', () => ({
    DiveQueue: vi.fn(function () {
        return {
            addDive: vi.fn().mockImplementation(async (fn) => {
                try {
                    const result = await fn();
                    return { success: true, result };
                } catch (e) {
                    return { success: false, error: e.message };
                }
            }),
            canEngage: vi.fn().mockReturnValue(true),
            recordEngagement: vi.fn().mockReturnValue(true),
            getEngagementProgress: vi.fn().mockReturnValue({
                likes: { current: 0, limit: 10 },
                replies: { current: 0, limit: 10 },
                quotes: { current: 0, limit: 10 },
                bookmarks: { current: 0, limit: 10 },
            }),
            getFullStatus: vi.fn().mockReturnValue({
                queueLength: 0,
                activeCount: 0,
                utilization: 0,
                capacity: 30,
                maxQueueSize: 30,
            }),
            isHealthy: vi.fn().mockReturnValue(true),
            enableQuickMode: vi.fn(),
            disableQuickMode: vi.fn(),
            resetEngagement: vi.fn(),
        };
    }),
}));

// Mock Actions
vi.mock('@api/actions/ai-twitter-reply.js', () => ({
    AIReplyAction: vi.fn(function () {
        return { getStats: vi.fn() };
    }),
}));
vi.mock('@api/actions/ai-twitter-quote.js', () => ({
    AIQuoteAction: vi.fn(function () {
        return { getStats: vi.fn() };
    }),
}));
vi.mock('@api/actions/ai-twitter-like.js', () => ({
    LikeAction: vi.fn(function () {
        return { getStats: vi.fn() };
    }),
}));
vi.mock('@api/actions/ai-twitter-bookmark.js', () => ({
    BookmarkAction: vi.fn(function () {
        return { getStats: vi.fn() };
    }),
}));
vi.mock('@api/actions/ai-twitter-go-home.js', () => ({
    GoHomeAction: vi.fn(function () {
        return { getStats: vi.fn() };
    }),
}));
vi.mock('@api/actions/advanced-index.js', () => ({
    ActionRunner: vi.fn(function () {
        return {
            selectAction: vi.fn().mockReturnValue('reply'),
            executeAction: vi
                .fn()
                .mockResolvedValue({ success: true, executed: true, reason: 'Test' }),
            getStats: vi.fn(),
        };
    }),
}));

vi.mock('@api/behaviors/human-interaction.js', () => ({
    HumanInteraction: vi.fn(function () {
        return {
            findWithFallback: vi.fn(),
            sessionStart: vi.fn(),
            sessionEnd: vi.fn(),
            cycleComplete: vi.fn(),
        };
    }),
}));

vi.mock('@api/core/logger.js', () => ({
    createBufferedLogger: vi.fn().mockReturnValue({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        shutdown: vi.fn(),
    }),
    createLogger: vi.fn().mockReturnValue({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    }),
}));

vi.mock('@api/index.js', () => ({
    api: {
        getCurrentUrl: vi.fn().mockReturnValue('https://x.com/home'),
        goto: vi.fn().mockResolvedValue(undefined),
        waitForURL: vi.fn().mockResolvedValue(undefined),
        waitVisible: vi.fn().mockResolvedValue(undefined),
        wait: vi.fn((ms) => new Promise((resolve) => setTimeout(resolve, ms))),
        getPersona: vi.fn().mockReturnValue({ hoverMin: 1000, hoverMax: 3000 }),
        maybeDistract: vi.fn().mockResolvedValue(undefined),
        isSessionActive: vi.fn().mockReturnValue(true),
        emulateMedia: vi.fn().mockResolvedValue(undefined),
        think: vi.fn().mockResolvedValue(undefined),
    },
}));

describe('AITwitterAgent (Real Implementation)', () => {
    let agent;
    let mockPage;
    let mockLogger;
    let mockProfile;

    afterEach(async () => {
        if (agent && typeof agent.shutdown === 'function') {
            await agent.shutdown();
        }
    });

    beforeEach(() => {
        vi.clearAllMocks();

        // Setup Mock Page
        mockPage = {
            url: vi.fn().mockReturnValue('https://x.com/home'),
            evaluate: vi.fn().mockResolvedValue({ readyState: 'complete' }),
            waitForTimeout: vi.fn().mockResolvedValue(),
            locator: vi.fn().mockReturnValue({
                first: vi.fn().mockReturnValue({
                    count: vi.fn().mockResolvedValue(1),
                    isVisible: vi.fn().mockResolvedValue(true),
                    innerText: vi.fn().mockResolvedValue('Tweet text'),
                    boundingBox: vi.fn().mockResolvedValue({ x: 0, y: 0, width: 100, height: 100 }),
                    click: vi.fn().mockResolvedValue(),
                    scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(),
                    getAttribute: vi.fn().mockResolvedValue(''),
                }),
                count: vi.fn().mockResolvedValue(1),
            }),
            context: vi.fn().mockReturnValue({
                browser: vi.fn().mockReturnValue({
                    isConnected: vi.fn().mockReturnValue(true),
                }),
            }),
            keyboard: {
                press: vi.fn().mockResolvedValue(),
            },
            mouse: {
                move: vi.fn().mockResolvedValue(),
            },
            viewportSize: vi.fn().mockReturnValue({ width: 1280, height: 720 }),
        };

        mockLogger = {
            info: vi.fn(),
            warn: vi.fn(),
            error: vi.fn(),
            debug: vi.fn(),
        };

        mockProfile = {
            id: 'test-profile',
        };

        agent = new AITwitterAgent(mockPage, mockProfile, mockLogger, {
            engagementLimits: { likes: 10 },
        });
    });

    describe('Initialization', () => {
        it('should initialize all components', () => {
            expect(agent).toBeDefined();
            expect(agent.diveQueue).toBeDefined();
            expect(agent.agentConnector).toBeDefined();
            expect(agent.replyEngine).toBeDefined();
            expect(agent.quoteEngine).toBeDefined();
            expect(agent.contextEngine).toBeDefined();
            expect(agent.engagementTracker).toBeDefined();
            expect(agent.microHandler).toBeDefined();
            expect(agent.motorHandler).toBeDefined();
            expect(agent.actionRunner).toBeDefined();
        });

        it('should initialize synchronization between tracker and queue', () => {
            // Test canPerform override
            agent.engagementTracker.canPerform('likes');
            expect(agent.diveQueue.canEngage).toHaveBeenCalledWith('likes');

            // Test record override
            agent.engagementTracker.record('likes');
            expect(agent.diveQueue.recordEngagement).toHaveBeenCalledWith('likes');
        });
    });

    describe('Dive Lock Mechanism', () => {
        it('should acquire and release dive lock', async () => {
            expect(agent.isDiving()).toBe(false);

            await agent.startDive();
            expect(agent.isDiving()).toBe(true);
            expect(agent.operationLock).toBe(true);
            expect(agent.scrollingEnabled).toBe(false);

            await agent.endDive(true, false);
            expect(agent.isDiving()).toBe(false);
            expect(agent.operationLock).toBe(false);
            // scrollingEnabled is re-enabled in endDive
            expect(agent.scrollingEnabled).toBe(true);
        });

        it('should return to home on endDive if requested', async () => {
            // Mock safeNavigateHome via parent or internal logic
            // Since _safeNavigateHome calls this.page.url(), we need to ensure it behaves correctly
            // But _safeNavigateHome is an async method on the class itself, so we can test it implicitly

            // Spy on _safeNavigateHome (since it's not mocked)
            const navigateSpy = vi.spyOn(agent, '_safeNavigateHome');

            await agent.startDive();
            await agent.endDive(false, true); // Failed dive, return home

            expect(navigateSpy).toHaveBeenCalled();
            expect(agent.pageState).toBe('HOME');
        });
    });

    describe('Health Check', () => {
        it('should report healthy when everything is fine', async () => {
            const result = await agent.performHealthCheck();
            expect(result.healthy).toBe(true);
        });

        it('should report unhealthy if browser disconnected', async () => {
            mockPage.context().browser().isConnected.mockReturnValue(false);
            const result = await agent.performHealthCheck();
            expect(result.healthy).toBe(false);
            expect(result.reason).toBe('browser_disconnected');
        });
    });

    describe('AI Reply Handling', () => {
        // Skipped: test isolation issues (pass individually, fail in full suite)
        it('should skip negative sentiment tweets', async () => {
            // Mock negative sentiment
            const { sentimentService } = await import('@api/utils/sentiment-service.js');
            sentimentService.analyze.mockReturnValueOnce({
                isNegative: true,
                score: -0.8,
                dimensions: {
                    valence: { valence: -0.8 },
                    arousal: { arousal: 0.5 },
                    dominance: { dominance: 0.5 },
                    sarcasm: { sarcasm: 0 },
                    toxicity: { toxicity: 0 },
                },
                engagement: { warnings: [] },
                composite: { riskLevel: 'high' },
            });

            await agent.handleAIReply('I hate this', 'user');

            expect(mockLogger.info).toHaveBeenCalledWith(
                expect.stringContaining('Skipped (negative sentiment)')
            );
        });

        // Skipped: test isolation issues (pass individually, fail in full suite)
        it('should execute reply if sentiment is positive and limits allow', async () => {
            await agent.handleAIReply('I love this', 'user');

            expect(agent.contextEngine.extractEnhancedContext).toHaveBeenCalled();
            expect(agent.replyEngine.shouldReply).toHaveBeenCalled();
            expect(agent.replyEngine.executeReply).toHaveBeenCalled();
        });
    });
});
