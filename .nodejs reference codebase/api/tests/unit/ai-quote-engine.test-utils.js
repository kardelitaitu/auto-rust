/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { vi } from 'vitest';

export const createPageMock = (overrides = {}) => {
    const locator = {
        count: vi.fn().mockResolvedValue(1),
        click: vi.fn().mockResolvedValue(true),
        first: function () {
            return this;
        },
        textContent: vi.fn().mockResolvedValue('default text'),
        isVisible: vi.fn().mockResolvedValue(true),
        getAttribute: vi.fn().mockResolvedValue(''),
        scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(),
        all: vi.fn().mockResolvedValue([]),
        fill: vi.fn().mockResolvedValue(),
        press: vi.fn().mockResolvedValue(),
    };
    locator.all = vi.fn().mockResolvedValue([locator]);

    const page = {
        evaluate: vi.fn((fn, arg) => fn(arg)),
        keyboard: {
            press: vi.fn().mockResolvedValue(),
            type: vi.fn().mockResolvedValue(),
        },
        mouse: {
            click: vi.fn().mockResolvedValue(),
            move: vi.fn().mockResolvedValue(),
        },
        locator: vi.fn(() => locator),
        waitVisible: vi.fn().mockResolvedValue(),
        waitForSelector: vi.fn().mockResolvedValue(),
        waitForTimeout: vi.fn().mockResolvedValue(),
        url: vi.fn().mockReturnValue('https://x.com/status/1'),
        content: vi.fn().mockResolvedValue('<html></html>'),
        ...overrides,
    };

    return { page, locator };
};

export const createHumanMock = (overrides = {}) => ({
    logStep: vi.fn(),
    verifyComposerOpen: vi
        .fn()
        .mockResolvedValue({ open: true, selector: '[data-testid="tweetTextarea_0"]' }),
    typeText: vi.fn(),
    postTweet: vi.fn().mockResolvedValue({ success: true, reason: 'posted' }),
    safeHumanClick: vi.fn().mockResolvedValue(true),
    fixation: vi.fn(),
    microMove: vi.fn(),
    hesitation: vi.fn(),
    ensureFocus: vi.fn().mockResolvedValue(true),
    selectMethod: vi.fn((methods) => methods[0]),
    ...overrides,
});

export const baseSentiment = {
    score: 0.5,
    isNegative: false,
    composite: {
        score: 0.5,
        label: 'neutral',
        engagementStyle: 'neutral',
        riskLevel: 'low',
        conversationType: 'general',
    },
    dimensions: {
        valence: { valence: 0.1 },
        arousal: { arousal: 0.1 },
        dominance: { dominance: 0.1 },
        sarcasm: { sarcasm: 0.1 },
        toxicity: { toxicity: 0.1 },
        intent: { label: 'observation' },
    },
};

export const sampleReplies = [
    { text: 'Great point!', author: 'user1' },
    { text: 'I disagree completely.', author: 'user2' },
];

export const sampleTweet = {
    text: 'This is a sample tweet for testing.',
    author: 'mainuser',
    url: 'https://x.com/mainuser/status/123',
};
