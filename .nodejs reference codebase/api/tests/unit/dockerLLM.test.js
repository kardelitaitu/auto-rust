/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for utils/dockerLLM.js
 * @module tests/unit/dockerLLM.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { exec } from 'child_process';

vi.mock('child_process', () => ({
    exec: vi.fn((cmd, opts, cb) => {
        if (typeof opts === 'function') opts(null, '', '');
        else if (typeof cb === 'function') cb(null, '', '');
        return { unref: () => {} };
    }),
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn().mockReturnValue({
        info: vi.fn(),
        error: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
        success: vi.fn(),
    }),
}));

vi.mock('@api/utils/configLoader.js', () => ({
    getSettings: vi.fn(),
}));

describe('utils/dockerLLM', () => {
    let dockerLLM;
    let fetchSpy;

    beforeEach(async () => {
        vi.clearAllMocks();
        vi.useFakeTimers();

        // Mock fetch
        fetchSpy = vi.fn();
        global.fetch = fetchSpy;

        // Setup default config mock
        const configLoader = await import('@api/utils/configLoader.js');
        configLoader.getSettings.mockResolvedValue({
            llm: {
                local: {
                    endpoint: 'http://localhost:11434',
                    provider: 'ollama',
                    model: 'llama3.2-vision',
                },
            },
        });

        const module = await import('../../utils/dockerLLM.js');
        dockerLLM = module;
    });

    afterEach(() => {
        vi.useRealTimers();
        vi.restoreAllMocks();
    });

    describe('ensureDockerLLM', () => {
        it('should return true if LLM is already ready', async () => {
            fetchSpy.mockResolvedValue({ ok: true });

            const result = await dockerLLM.ensureDockerLLM();

            expect(result).toBe(true);
            expect(fetchSpy).toHaveBeenCalledWith('http://localhost:11434/', expect.anything());
        });

        it('should attempt to start LLM if not ready', async () => {
            // First check fails
            fetchSpy.mockResolvedValueOnce({ ok: false });
            // Second check (after start) succeeds
            fetchSpy.mockResolvedValueOnce({ ok: true });

            const promise = dockerLLM.ensureDockerLLM();

            // Fast-forward through timeouts
            await vi.runAllTimersAsync();

            const result = await promise;

            expect(result).toBe(true);
            // Now runs in background, so just verify it was called
            expect(exec).toHaveBeenCalled();
        });

        it('should use docker provider if configured', async () => {
            const { getSettings } = await import('@api/utils/configLoader.js');
            getSettings.mockResolvedValue({
                llm: {
                    local: {
                        provider: 'docker',
                        model: 'test-model',
                    },
                },
            });

            fetchSpy.mockResolvedValueOnce({ ok: false });
            fetchSpy.mockResolvedValueOnce({ ok: true });

            const promise = dockerLLM.ensureDockerLLM();
            await vi.runAllTimersAsync();
            await promise;

            expect(exec).toHaveBeenCalledWith(
                expect.stringContaining('docker model run test-model'),
                expect.anything()
            );
        });

        it('should return false after max attempts if still not ready', async () => {
            fetchSpy.mockResolvedValue({ ok: false });

            const promise = dockerLLM.ensureDockerLLM();

            // Start timeout (5000) + 5 attempts * 3000
            await vi.runAllTimersAsync();

            const result = await promise;

            expect(result).toBe(false);
        });
    });
});
