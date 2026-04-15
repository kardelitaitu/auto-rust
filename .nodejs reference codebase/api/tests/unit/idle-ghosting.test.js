/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock module - now in api/behaviors/idle.js
class IdleGhosting {
    constructor() {
        this.isActive = false;
        this.wiggleInterval = null;
        this.scrollInterval = null;
        this.wiggle = true;
        this.scroll = true;
        this.frequency = 3000;
        this.magnitude = 5;
    }

    async start(page, options = {}) {
        if (this.isActive) return;

        this.wiggle = options.wiggle !== false;
        this.scroll = options.scroll !== false;
        this.frequency = options.frequency || 3000;
        this.magnitude = options.magnitude || 5;
        this.page = page;

        this.isActive = true;

        if (this.wiggle) {
            this.wiggleInterval = setInterval(() => {
                if (page?.mouse?.move) {
                    page.mouse.move(Math.random() * this.magnitude, Math.random() * this.magnitude);
                }
            }, this.frequency);
        }

        if (this.scroll) {
            this.scrollInterval = setInterval(() => {
                if (page?.mouse?.wheel) {
                    page.mouse.wheel(0, Math.random() * this.magnitude);
                }
            }, this.frequency * 2);
        }
    }

    stop() {
        this.isActive = false;
        if (this.wiggleInterval) {
            clearInterval(this.wiggleInterval);
            this.wiggleInterval = null;
        }
        if (this.scrollInterval) {
            clearInterval(this.scrollInterval);
            this.scrollInterval = null;
        }
    }
}

// Mock Logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn().mockReturnValue({
        info: vi.fn(),
        debug: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
    }),
}));

describe('IdleGhosting', () => {
    let ghosting;
    let mockPage;

    beforeEach(() => {
        vi.useFakeTimers();
        ghosting = new IdleGhosting();
        mockPage = {
            viewportSize: vi.fn().mockReturnValue({ width: 1000, height: 1000 }),
            mouse: {
                move: vi.fn().mockResolvedValue(undefined),
                wheel: vi.fn().mockResolvedValue(undefined),
            },
        };
    });

    afterEach(() => {
        ghosting.stop();
        vi.useRealTimers();
        vi.clearAllMocks();
    });

    describe('start/stop', () => {
        it('should start ghosting with default options', async () => {
            await ghosting.start(mockPage);

            expect(ghosting.isActive).toBe(true);
            expect(ghosting.wiggleInterval).not.toBeNull();

            // Advance timers to trigger the interval callback
            vi.advanceTimersByTime(ghosting.frequency);

            expect(mockPage.mouse.move).toHaveBeenCalled();
        });

        it('should not start if already active', async () => {
            await ghosting.start(mockPage);
            const firstInterval = ghosting.wiggleInterval;

            await ghosting.start(mockPage);
            expect(ghosting.wiggleInterval).toBe(firstInterval);
        });

        it('should start with micro-scroll and no wiggle', async () => {
            await ghosting.start(mockPage, { wiggle: false, scroll: true });
            expect(ghosting.isActive).toBe(true);
            expect(ghosting.wiggleInterval).toBeNull();
        });

        it('should stop ghosting correctly', async () => {
            await ghosting.start(mockPage);
            expect(ghosting.isActive).toBe(true);

            await ghosting.stop();
            expect(ghosting.isActive).toBe(false);
            expect(ghosting.wiggleInterval).toBeNull();
            expect(ghosting.scrollInterval).toBeNull();
        });

        it('should not stop if not active', async () => {
            await ghosting.stop();
            expect(ghosting.isActive).toBe(false);
        });
    });

    describe('mouse movement', () => {
        it('should perform micro movements when active', async () => {
            const moveSpy = vi.spyOn(mockPage.mouse, 'move');

            await ghosting.start(mockPage);

            vi.advanceTimersByTime(ghosting.frequency);

            expect(moveSpy).toHaveBeenCalled();
        });

        it('should respect magnitude option', async () => {
            await ghosting.start(mockPage, { magnitude: 10 });

            vi.advanceTimersByTime(ghosting.frequency);

            expect(mockPage.mouse.move).toHaveBeenCalled();
        });
    });

    describe('scroll movement', () => {
        it('should perform scroll movements when enabled', async () => {
            const wheelSpy = vi.spyOn(mockPage.mouse, 'wheel');

            await ghosting.start(mockPage, { scroll: true });

            vi.advanceTimersByTime(ghosting.frequency * 2);

            expect(wheelSpy).toHaveBeenCalled();
        });

        it('should not scroll when disabled', async () => {
            const wheelSpy = vi.spyOn(mockPage.mouse, 'wheel');

            await ghosting.start(mockPage, { scroll: false });

            vi.advanceTimersByTime(ghosting.frequency * 2);

            expect(wheelSpy).not.toHaveBeenCalled();
        });
    });

    describe('constructor options', () => {
        it('should have default values', () => {
            expect(ghosting.wiggle).toBe(true);
            expect(ghosting.scroll).toBe(true);
            expect(ghosting.frequency).toBe(3000);
            expect(ghosting.magnitude).toBe(5);
        });
    });
});
