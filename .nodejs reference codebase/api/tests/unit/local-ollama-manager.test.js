/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        error: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
        success: vi.fn(),
    })),
}));

vi.mock('@api/utils/configLoader.js', () => ({
    getSettings: vi.fn().mockResolvedValue({
        llm: {
            local: {
                endpoint: 'http://localhost:11434',
                model: 'test-model',
            },
        },
    }),
}));

vi.mock('child_process', () => ({
    exec: vi.fn((cmd, cb) => cb && cb(null)),
    execSync: vi.fn(),
}));

describe('local-ollama-manager', () => {
    let originalAbortSignal;

    beforeEach(async () => {
        vi.clearAllMocks();
        originalAbortSignal = global.AbortSignal;
        const manager = await import('../../utils/local-ollama-manager.js');
        manager.clearOllamaCache();
    });

    afterEach(() => {
        delete process.env.ALLOW_OLLAMA_MODEL_OPS;
        vi.resetModules();
        if (originalAbortSignal) {
            global.AbortSignal = originalAbortSignal;
        }
        vi.useRealTimers();
    });

    describe('isOllamaRunning', () => {
        it('should return false if process not running', async () => {
            const { execSync } = await import('child_process');
            execSync.mockReturnValue('node.exe');

            global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'));

            const { isOllamaRunning } = await import('../../utils/local-ollama-manager.js');
            const result = await isOllamaRunning();

            expect(result).toBe(false);
        });

        it('should return false when tasklist check throws', async () => {
            const { execSync } = await import('child_process');
            execSync.mockImplementation(() => {
                throw new Error('tasklist fail');
            });

            global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'));

            const { isOllamaRunning } = await import('../../utils/local-ollama-manager.js');
            const result = await isOllamaRunning();

            expect(result).toBe(false);
        });

        it('should return true if API responds', async () => {
            const { execSync } = await import('child_process');
            execSync.mockReturnValue('ollama.exe');

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
            });

            const { isOllamaRunning } = await import('../../utils/local-ollama-manager.js');
            const result = await isOllamaRunning();

            expect(result).toBe(true);
        });

        it('should return true if API fails but process is running', async () => {
            const { execSync } = await import('child_process');
            execSync.mockReturnValue('ollama.exe');

            global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'));

            const { isOllamaRunning } = await import('../../utils/local-ollama-manager.js');
            const result = await isOllamaRunning();

            expect(result).toBe(true);
        });

        it('should use default endpoint when settings are missing', async () => {
            const { execSync } = await import('child_process');
            const { getSettings } = await import('@api/utils/configLoader.js');
            execSync.mockReturnValue('ollama.exe');
            getSettings.mockResolvedValueOnce({});

            global.fetch = vi.fn().mockResolvedValue({ ok: true });

            const { isOllamaRunning } = await import('../../utils/local-ollama-manager.js');
            const result = await isOllamaRunning();

            expect(result).toBe(true);
            expect(global.fetch).toHaveBeenCalledWith(
                'http://localhost:11434/',
                expect.any(Object)
            );
        });
    });

    describe('ensureOllama', () => {
        beforeEach(() => {
            process.env.ALLOW_OLLAMA_MODEL_OPS = 'true';
        });

        afterEach(() => {
            delete process.env.ALLOW_OLLAMA_MODEL_OPS;
        });

        it('should return true if already running', async () => {
            const { execSync } = await import('child_process');
            execSync.mockReturnValue('ollama.exe');

            global.fetch = vi.fn().mockResolvedValue({ ok: true });

            const { ensureOllama } = await import('../../utils/local-ollama-manager.js');
            const result = await ensureOllama();

            expect(result).toBe(true);
        });

        it('should return true after starting successfully', async () => {
            process.env.ALLOW_OLLAMA_MODEL_OPS = 'true';

            const { execSync } = await import('child_process');
            const { exec } = await import('child_process');

            let callCount = 0;
            execSync.mockImplementation(() => {
                callCount++;
                return callCount >= 2 ? 'ollama.exe' : '';
            });

            exec.mockImplementation((cmd, cb) => {
                if (cb) cb(null);
            });

            let fetchCount = 0;
            global.fetch = vi.fn().mockImplementation(() => {
                fetchCount++;
                return fetchCount >= 2
                    ? Promise.resolve({ ok: true })
                    : Promise.reject(new Error('Not ready'));
            });

            const { ensureOllama } = await import('../../utils/local-ollama-manager.js');
            const result = await ensureOllama();

            expect(result).toBe(true);
        });

        it('should return false when start fails and verification never succeeds', async () => {
            process.env.ALLOW_OLLAMA_MODEL_OPS = 'true';
            vi.useFakeTimers();

            try {
                const { execSync } = await import('child_process');
                execSync.mockReturnValue('');
                global.fetch = vi.fn().mockRejectedValue(new Error('fail'));

                const module = await import('../../utils/local-ollama-manager.js');
                vi.spyOn(module, 'startOllama').mockResolvedValue(false);

                const { ensureOllama } = module;
                const resultPromise = ensureOllama();
                await vi.runAllTimersAsync();
                const result = await resultPromise;

                expect(result).toBe(false);
            } finally {
                vi.useRealTimers();
                delete process.env.ALLOW_OLLAMA_MODEL_OPS;
            }
        });
    });

    describe('startOllama', () => {
        it('should return false when API never becomes ready', async () => {
            process.env.ALLOW_OLLAMA_MODEL_OPS = 'true';
            vi.useFakeTimers();

            try {
                const { execSync, exec } = await import('child_process');
                execSync.mockImplementation((cmd) => {
                    if (cmd.includes('tasklist')) return 'ollama.exe';
                    if (cmd.includes('ollama list')) return 'model';
                    return '';
                });
                exec.mockImplementation((cmd, options, cb) => {
                    if (typeof options === 'function') {
                        cb = options;
                    }
                    if (cb) cb(null);
                    return { on: vi.fn() };
                });

                global.fetch = vi.fn().mockResolvedValue({ ok: false });

                const { startOllama } = await import('../../utils/local-ollama-manager.js');
                const resultPromise = startOllama();
                await vi.runAllTimersAsync();
                const result = await resultPromise;
                expect(result).toBe(false);
            } finally {
                vi.useRealTimers();
            }
        });

        it('should return true when model exists and api becomes ready', async () => {
            vi.useFakeTimers();
            let originalAbortSignal;

            try {
                const { execSync, exec } = await import('child_process');
                execSync.mockImplementation((cmd) => {
                    if (cmd.includes('tasklist')) return 'ollama.exe';
                    if (cmd.includes('ollama list')) return 'test-model';
                    return '';
                });
                exec.mockImplementation((cmd, options, cb) => {
                    if (typeof options === 'function') {
                        cb = options;
                    }
                    if (cb) cb(null);
                    return { on: vi.fn() };
                });

                originalAbortSignal = global.AbortSignal;
                global.AbortSignal = { timeout: () => new AbortController().signal };
                global.fetch = vi.fn().mockResolvedValue({ ok: true });

                const { startOllama } = await import('../../utils/local-ollama-manager.js');
                const resultPromise = startOllama();
                await vi.runAllTimersAsync();
                const result = await resultPromise;
                expect(result).toBe(true);
            } finally {
                global.AbortSignal = originalAbortSignal;
                vi.useRealTimers();
            }
        });

        it('should return false when pull fails', async () => {
            process.env.ALLOW_OLLAMA_MODEL_OPS = 'true';
            const { execSync, exec } = await import('child_process');
            execSync.mockImplementation((cmd) => {
                if (cmd.includes('tasklist')) return 'ollama.exe';
                if (cmd.includes('ollama list')) return '';
                return '';
            });
            exec.mockImplementation((cmd, options, cb) => {
                if (typeof options === 'function') {
                    cb = options;
                }
                if (cmd.startsWith('ollama pull')) {
                    return { on: (_event, handler) => handler(1) };
                }
                if (cb) cb(null);
                return { on: vi.fn() };
            });

            global.fetch = vi.fn().mockResolvedValue({ ok: true });

            const { startOllama } = await import('../../utils/local-ollama-manager.js');
            const result = await startOllama();
            expect(result).toBe(false);
        });

        it('should pull missing model successfully', async () => {
            process.env.ALLOW_OLLAMA_MODEL_OPS = 'true';
            const { execSync, exec } = await import('child_process');
            const { getSettings } = await import('@api/utils/configLoader.js');
            execSync.mockImplementation((cmd) => {
                if (cmd.includes('tasklist')) return 'ollama.exe';
                if (cmd.includes('ollama list')) throw new Error('list fail');
                return '';
            });
            getSettings.mockResolvedValueOnce({});
            exec.mockImplementation((cmd, options, cb) => {
                if (typeof options === 'function') {
                    cb = options;
                }
                if (cmd.startsWith('ollama pull')) {
                    return { on: (_event, handler) => handler(0) };
                }
                if (cb) cb(null);
                return { on: vi.fn() };
            });

            global.fetch = vi.fn().mockResolvedValue({ ok: true });

            const { startOllama } = await import('../../utils/local-ollama-manager.js');
            const result = await startOllama();
            expect(result).toBe(true);
        });

        it('should attempt alternate start commands when list trigger fails', async () => {
            vi.useFakeTimers();
            try {
                const { execSync, exec } = await import('child_process');
                let tasklistCalls = 0;
                execSync.mockImplementation((cmd) => {
                    if (cmd.includes('tasklist')) {
                        tasklistCalls += 1;
                        return tasklistCalls >= 3 ? 'ollama.exe' : '';
                    }
                    if (cmd.includes('ollama list')) return 'test-model';
                    return '';
                });
                exec.mockImplementation((cmd, options, cb) => {
                    if (typeof options === 'function') {
                        cb = options;
                    }
                    if (cmd === 'ollama list') {
                        if (cb) cb(new Error('list trigger fail'));
                        return { on: vi.fn() };
                    }
                    if (cmd === 'start "" "ollama app.exe"') {
                        if (cb) cb(new Error('app fail'));
                        return { on: vi.fn() };
                    }
                    return { on: vi.fn() };
                });

                global.fetch = vi.fn().mockResolvedValue({ ok: true });

                const { startOllama } = await import('../../utils/local-ollama-manager.js');
                const resultPromise = startOllama();
                await vi.runAllTimersAsync();
                const result = await resultPromise;
                expect(result).toBe(true);
            } finally {
                vi.useRealTimers();
            }
        });

        it('should start process when not running', async () => {
            vi.useFakeTimers();
            let originalAbortSignal;

            try {
                const { execSync, exec } = await import('child_process');
                let tasklistCalls = 0;
                execSync.mockImplementation((cmd) => {
                    if (cmd.includes('tasklist')) {
                        tasklistCalls += 1;
                        return tasklistCalls >= 2 ? 'ollama.exe' : '';
                    }
                    if (cmd.includes('ollama list')) return 'test-model';
                    return '';
                });
                exec.mockImplementation((cmd, options, cb) => {
                    if (typeof options === 'function') {
                        cb = options;
                    }
                    if (cb) cb(null);
                    return { on: vi.fn() };
                });

                originalAbortSignal = global.AbortSignal;
                global.AbortSignal = { timeout: () => new AbortController().signal };
                global.fetch = vi.fn().mockResolvedValue({ ok: true });

                const { startOllama } = await import('../../utils/local-ollama-manager.js');
                const resultPromise = startOllama();
                await vi.runAllTimersAsync();
                const result = await resultPromise;
                expect(result).toBe(true);
            } finally {
                global.AbortSignal = originalAbortSignal;
                vi.useRealTimers();
            }
        });
    });

    describe('startOllama error paths', () => {
        it('should log error when startOllama catch block executes', async () => {
            process.env.ALLOW_OLLAMA_MODEL_OPS = 'true';
            vi.useFakeTimers();
            try {
                const { getSettings } = await import('@api/utils/configLoader.js');

                // Make getSettings throw
                getSettings.mockRejectedValueOnce(new Error('Config error'));

                const { startOllama } = await import('../../utils/local-ollama-manager.js');
                const result = await startOllama();

                expect(result).toBe(false);
                // Error should be logged via logger.error
            } finally {
                vi.useRealTimers();
            }
        });
    });
});
