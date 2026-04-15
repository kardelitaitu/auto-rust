/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';

class AgentCortex {
    constructor(sessionId, goal, steps = []) {
        this.sessionId = sessionId;
        this.goal = goal;
        this.history = [];
        this.steps = steps;
        this.currentStep = 1;
    }
    async planNextStep() {
        return { thought: 'thinking', actions: [] };
    }
    recordResult(action, success) {
        this.history.push({ action, success });
        if (success) this.currentStep++;
    }
}

vi.mock('@api/core/logger.js', () => ({
    createLogger: () => ({
        debug: vi.fn(),
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        success: vi.fn(),
    }),
}));

describe('AgentCortex', () => {
    let cortex;
    beforeEach(() => {
        vi.clearAllMocks();
        cortex = new AgentCortex('test', 'goal', ['step1']);
    });
    it('should initialize', () => {
        expect(cortex.sessionId).toBe('test');
    });
    it('should plan next step', async () => {
        const plan = await cortex.planNextStep();
        expect(plan.thought).toBe('thinking');
    });
    it('should record result', () => {
        cortex.recordResult({ type: 'click' }, true);
        expect(cortex.currentStep).toBe(2);
    });
});
