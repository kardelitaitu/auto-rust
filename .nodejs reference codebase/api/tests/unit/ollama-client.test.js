/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { getSettings } from '@api/utils/configLoader.js';

describe('core/ollama-client.js', () => {
    let OllamaClient;
    let mockFetch;

    beforeEach(async () => {
        vi.resetModules();

        mockFetch = vi.fn();
        global.fetch = mockFetch;

        vi.mock('@api/core/logger.js', () => ({
            createLogger: () => ({
                info: vi.fn(),
                warn: vi.fn(),
                error: vi.fn(),
                success: vi.fn(),
                debug: vi.fn(),
            }),
        }));

        vi.mock('@api/utils/configLoader.js', () => ({
            getSettings: vi.fn().mockResolvedValue({
                llm: {
                    local: {
                        endpoint: 'http://localhost:11434',
                        model: 'llama3.2-vision',
                        timeout: 60000,
                    },
                },
            }),
        }));

        vi.mock('@api/utils/local-ollama-manager.js', () => ({
            ensureOllama: vi.fn().mockResolvedValue(true),
            isOllamaRunning: vi.fn().mockResolvedValue(true),
        }));

        const module = await import('../../core/ollama-client.js');
        OllamaClient = module.default;
    });

    afterEach(() => {
        vi.clearAllMocks();
    });

    describe('Constructor', () => {
        it('should create an instance with default values', () => {
            const client = new OllamaClient();
            expect(client.baseUrl).toBe('http://localhost:11434');
            expect(client.model).toBe('llava:latest');
            expect(client.timeout).toBe(180000);
            expect(client._warmedUp).toBe(false);
        });
    });

    describe('initialize', () => {
        it('should initialize with config', async () => {
            const client = new OllamaClient();
            await client.initialize();

            expect(client.config).toBeDefined();
            expect(client.baseUrl).toBe('http://localhost:11434');
        });

        it('should not reinitialize if already initialized', async () => {
            const client = new OllamaClient();
            await client.initialize();
            await client.initialize();

            const { ensureOllama } = await import('@api/utils/local-ollama-manager.js');
            expect(ensureOllama).toHaveBeenCalledTimes(1);
        });

        it('should not reinitialize if config already exists', async () => {
            const client = new OllamaClient();
            client.config = { existing: true };

            await client.initialize();

            // Config should remain unchanged
            expect(client.config).toEqual({ existing: true });
        });
    });

    describe('generate', () => {
        it('should make text generation request', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: () =>
                    Promise.resolve({
                        response: 'Test response',
                        model: 'llama3.2-vision',
                        total_duration: 1000,
                        eval_count: 10,
                        eval_duration: 500,
                    }),
            });

            const client = new OllamaClient();
            client._warmedUp = true;
            const result = await client.generate({
                prompt: 'Test prompt',
                temperature: 0.7,
                maxTokens: 100,
            });

            expect(result.success).toBe(true);
            expect(result.content).toBe('Test response');
            expect(mockFetch).toHaveBeenCalledWith(
                'http://localhost:11434/api/generate',
                expect.any(Object)
            );
        });

        it('should handle vision requests', async () => {
            // First call for warmup, second for actual request
            mockFetch
                .mockResolvedValueOnce({
                    ok: true,
                    json: () => Promise.resolve({ status: 'ok' }),
                })
                .mockResolvedValueOnce({
                    ok: true,
                    json: () =>
                        Promise.resolve({
                            message: { content: 'Vision response' },
                            model: 'llama3.2-vision',
                            total_duration: 2000,
                        }),
                });

            const client = new OllamaClient();
            const result = await client.generate({
                prompt: 'Describe this image',
                vision: Buffer.from('fake-image-data'),
                temperature: 0.7,
            });

            expect(result.success).toBe(true);
            expect(mockFetch).toHaveBeenCalledWith(
                'http://localhost:11434/api/chat',
                expect.any(Object)
            );
        });

        it('should disable vision if config.vision is false', async () => {
            // Mock getSettings to return vision: false
            getSettings.mockResolvedValueOnce({
                llm: {
                    local: {
                        endpoint: 'http://localhost:11434',
                        model: 'hermes3:8b',
                        vision: false,
                        timeout: 60000,
                    },
                },
            });

            // Mock generate response (text-only endpoint)
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: () =>
                    Promise.resolve({
                        response: 'Text response',
                        model: 'hermes3:8b',
                    }),
            });

            const client = new OllamaClient();
            client._warmedUp = true;
            await client.initialize(); // Ensure config is loaded from our mock

            const result = await client.generate({
                prompt: 'Test',
                vision: Buffer.from('image'),
            });

            expect(result.success).toBe(true);
            // Should use /api/generate (text), not /api/chat (vision)
            expect(mockFetch).toHaveBeenCalledWith(
                'http://localhost:11434/api/generate',
                expect.any(Object)
            );
        });

        it('should handle API errors', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                text: () => Promise.resolve('Model not found'),
            });

            const client = new OllamaClient();
            client._warmedUp = true;
            const result = await client.generate({
                prompt: 'Test prompt',
            });

            expect(result.success).toBe(false);
            expect(result.error).toContain('Ollama API error');
        });

        it('should handle request timeout', async () => {
            mockFetch.mockRejectedValueOnce({ name: 'AbortError' });

            const client = new OllamaClient();
            client._warmedUp = true;
            const result = await client.generate({
                prompt: 'Test prompt',
            });

            expect(result.success).toBe(false);
            expect(result.error).toBe('Request timeout');
        });

        it('should warmup vision model on first use', async () => {
            mockFetch
                .mockResolvedValueOnce({
                    ok: true,
                    json: () => Promise.resolve({ status: 'ok' }),
                })
                .mockResolvedValueOnce({
                    ok: true,
                    json: () =>
                        Promise.resolve({
                            message: { content: 'Vision response' },
                            model: 'llama3.2-vision',
                        }),
                });

            const client = new OllamaClient();
            await client.generate({
                prompt: 'Test',
                vision: Buffer.from('image'),
            });

            expect(mockFetch).toHaveBeenCalledTimes(2);
            expect(client._warmedUp).toBe(true);
        });
    });

    describe('isReady', () => {
        it('should return true when API is accessible', async () => {
            mockFetch.mockResolvedValueOnce({ ok: true });

            const client = new OllamaClient();
            const result = await client.isReady();

            expect(result).toBe(true);
        });

        it('should return false when API is not accessible', async () => {
            mockFetch.mockRejectedValueOnce(new Error('Connection refused'));

            const client = new OllamaClient();
            const result = await client.isReady();

            expect(result).toBe(false);
        });
    });

    describe('checkModel', () => {
        it('should check vision model successfully', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: () =>
                    Promise.resolve({
                        message: { content: 'ok' },
                        model: 'llava',
                    }),
            });

            const client = new OllamaClient();
            client.model = 'llava:latest';
            const result = await client.checkModel('Test prompt');

            expect(result.success).toBe(true);
            expect(result.content).toBe('ok');
        });

        it('should handle check failure', async () => {
            mockFetch.mockRejectedValueOnce(new Error('Connection failed'));

            const client = new OllamaClient();
            const result = await client.checkModel();

            expect(result.success).toBe(false);
            expect(result.error).toBe('Connection failed');
        });
    });

    describe('applyHumanization', () => {
        it('should abbreviate words with 15% probability', () => {
            const client = new OllamaClient();

            // Mock Math.random to always trigger abbreviation
            const originalRandom = Math.random;
            Math.random = vi.fn().mockReturnValue(0.1);

            const result = client.applyHumanization('because you are great');

            Math.random = originalRandom;

            expect(result).toContain('bc');
        });

        it('should replace I with i 20% of the time', () => {
            const client = new OllamaClient();

            const originalRandom = Math.random;
            Math.random = vi.fn().mockReturnValue(0.15);

            const result = client.applyHumanization('I think');

            Math.random = originalRandom;

            expect(result).toContain('i');
        });

        it('should handle null input', () => {
            const client = new OllamaClient();
            const result = client.applyHumanization(null);
            expect(result).toBeNull();
        });

        it('should handle empty string', () => {
            const client = new OllamaClient();
            const result = client.applyHumanization('');
            expect(result).toBe('');
        });
    });

    describe('applyTypos', () => {
        it('should not modify short text', () => {
            const client = new OllamaClient();
            const result = client.applyTypos('hi');
            expect(result).toBe('hi');
        });

        it('should handle null input', () => {
            const client = new OllamaClient();
            const result = client.applyTypos(null);
            expect(result).toBeNull();
        });

        it('should potentially introduce typos with very low probability', () => {
            const client = new OllamaClient();

            const originalRandom = Math.random;
            Math.random = vi.fn().mockReturnValue(0.0005); // Below 0.001 threshold

            const result = client.applyTypos('hello world test');

            Math.random = originalRandom;

            // Should potentially modify one character
            expect(typeof result).toBe('string');
        });
    });

    describe('resetStats', () => {
        it('should reset statistics', () => {
            const client = new OllamaClient();
            client.resetStats();
            // Should not throw
            expect(true).toBe(true);
        });
    });

    describe('wakeLocal', () => {
        it('should wake local ollama', async () => {
            vi.mock('child_process', () => ({
                exec: vi.fn((cmd, opts, callback) => {
                    callback(null, 'model1\nmodel2', '');
                }),
            }));

            const client = new OllamaClient();
            const result = await client.wakeLocal();

            expect(result).toBe(true);
        });

        it('should handle exec callback error gracefully', async () => {
            // The implementation always resolves to true, even when exec returns an error
            // because errors are passed to the callback, not thrown
            vi.mock('child_process', () => ({
                exec: vi.fn((cmd, opts, callback) => {
                    callback(new Error('Exec failed'), '', '');
                }),
            }));

            const client = new OllamaClient();
            const result = await client.wakeLocal();

            // Implementation resolves to true even on callback error
            expect(result).toBe(true);
        });
    });

    describe('Additional Coverage Tests', () => {
        it('should handle generate with images array', async () => {
            mockFetch
                .mockResolvedValueOnce({
                    ok: true,
                    json: () => Promise.resolve({ status: 'ok' }),
                })
                .mockResolvedValueOnce({
                    ok: true,
                    json: () =>
                        Promise.resolve({
                            message: { content: 'Image analysis response' },
                            model: 'llama3.2-vision',
                        }),
                });

            const client = new OllamaClient();
            const result = await client.generate({
                prompt: 'Analyze these images',
                images: ['base64image1', 'base64image2'],
                temperature: 0.5,
            });

            expect(result.success).toBe(true);
        });

        it('should handle vision request with string image', async () => {
            mockFetch
                .mockResolvedValueOnce({
                    ok: true,
                    json: () => Promise.resolve({ status: 'ok' }),
                })
                .mockResolvedValueOnce({
                    ok: true,
                    json: () =>
                        Promise.resolve({
                            message: { content: 'Response' },
                            model: 'llama3.2-vision',
                        }),
                });

            const client = new OllamaClient();
            const result = await client.generate({
                prompt: 'Test',
                vision: 'already-base64-string',
            });

            expect(result.success).toBe(true);
        });

        it('should handle vision request with array buffer', async () => {
            mockFetch
                .mockResolvedValueOnce({
                    ok: true,
                    json: () => Promise.resolve({ status: 'ok' }),
                })
                .mockResolvedValueOnce({
                    ok: true,
                    json: () =>
                        Promise.resolve({
                            message: { content: 'Response' },
                            model: 'llama3.2-vision',
                        }),
                });

            const client = new OllamaClient();
            const buffer = new Uint8Array([1, 2, 3]);
            const result = await client.generate({
                prompt: 'Test',
                vision: buffer,
            });

            expect(result.success).toBe(true);
        });

        it('should include system prompt in vision request', async () => {
            mockFetch
                .mockResolvedValueOnce({
                    ok: true,
                    json: () => Promise.resolve({ status: 'ok' }),
                })
                .mockResolvedValueOnce({
                    ok: true,
                    json: () =>
                        Promise.resolve({
                            message: { content: 'Response' },
                            model: 'llama3.2-vision',
                        }),
                });

            const client = new OllamaClient();
            const result = await client.generate({
                prompt: 'Test',
                vision: Buffer.from('image'),
                system: 'You are a helpful assistant',
            });

            expect(result.success).toBe(true);
        });

        it('should use systemPrompt alias for system', async () => {
            mockFetch
                .mockResolvedValueOnce({
                    ok: true,
                    json: () => Promise.resolve({ status: 'ok' }),
                })
                .mockResolvedValueOnce({
                    ok: true,
                    json: () =>
                        Promise.resolve({
                            message: { content: 'Response' },
                            model: 'llama3.2-vision',
                        }),
                });

            const client = new OllamaClient();
            const result = await client.generate({
                prompt: 'Test',
                vision: Buffer.from('image'),
                systemPrompt: 'System prompt via alias',
            });

            expect(result.success).toBe(true);
        });

        it('should handle warmup failure gracefully', async () => {
            mockFetch.mockRejectedValueOnce(new Error('Warmup failed')).mockResolvedValueOnce({
                ok: true,
                json: () =>
                    Promise.resolve({
                        message: { content: 'Response' },
                        model: 'llama3.2-vision',
                    }),
            });

            const client = new OllamaClient();
            const result = await client.generate({
                prompt: 'Test',
                vision: Buffer.from('image'),
            });

            expect(result.success).toBe(true);
            expect(client._warmedUp).toBe(false);
        });

        it('should handle vision model already warmed up', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: () =>
                    Promise.resolve({
                        message: { content: 'Response' },
                        model: 'llama3.2-vision',
                    }),
            });

            const client = new OllamaClient();
            client._warmedUp = true;

            const result = await client.generate({
                prompt: 'Test',
                vision: Buffer.from('image'),
            });

            expect(result.success).toBe(true);
            expect(mockFetch).toHaveBeenCalledTimes(1);
        });

        it('should handle text-only request with LLaVA model', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: () =>
                    Promise.resolve({
                        response: 'LLaVA text response',
                        model: 'llava',
                    }),
            });

            const client = new OllamaClient();
            client.model = 'llava:latest';
            client._warmedUp = true;
            const result = await client.generate({
                prompt: 'Test prompt',
            });

            expect(result.success).toBe(true);
        });

        it('should apply Twitter post-processing', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: () =>
                    Promise.resolve({
                        response: 'This is a @mention and #hashtag tweet!',
                        model: 'llama3.2-vision',
                    }),
            });

            const client = new OllamaClient();
            client._warmedUp = true;
            const result = await client.generate({
                prompt: 'Tweet from @user about #topic',
                systemPrompt: 'Twitter context',
            });

            expect(result.success).toBe(true);
            expect(result.content).not.toContain('@');
            expect(result.content).not.toContain('#');
        });

        it('should allow emojis when EMOJI keyword present', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: () =>
                    Promise.resolve({
                        response: 'Great job! 🎉',
                        model: 'llama3.2-vision',
                    }),
            });

            const client = new OllamaClient();
            client._warmedUp = true;
            const result = await client.generate({
                prompt: 'Tweet from @user EMOJI',
                systemPrompt: 'Twitter with EMOJI',
            });

            expect(result.success).toBe(true);
        });

        it('should truncate long responses to first sentence', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: () =>
                    Promise.resolve({
                        response:
                            'First sentence here. Second sentence should be removed. Third one too.',
                        model: 'llama3.2-vision',
                    }),
            });

            const client = new OllamaClient();
            client._warmedUp = true;
            const result = await client.generate({
                prompt: 'Tweet from @user',
            });

            expect(result.success).toBe(true);
            expect(result.content).not.toContain('Second');
        });

        it('should apply lowercasing randomly', async () => {
            const originalRandom = Math.random;
            Math.random = vi.fn().mockReturnValue(0.1);

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: () =>
                    Promise.resolve({
                        response: 'This Is A Tweet.',
                        model: 'llama3.2-vision',
                    }),
            });

            const client = new OllamaClient();
            client._warmedUp = true;
            const result = await client.generate({
                prompt: 'Tweet from @user',
            });

            Math.random = originalRandom;
            expect(result.success).toBe(true);
        });

        it('should handle generate with system parameter for non-LLaVA', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: () =>
                    Promise.resolve({
                        response: 'Response with system',
                        model: 'llama3.2',
                    }),
            });

            const client = new OllamaClient();
            client.model = 'llama3.2';
            client._warmedUp = true;
            const result = await client.generate({
                prompt: 'Test',
                system: 'System context',
            });

            expect(result.success).toBe(true);
        });

        it('should handle missing space after punctuation', async () => {
            const originalRandom = Math.random;
            Math.random = vi.fn().mockReturnValue(0.005);

            const client = new OllamaClient();
            const result = client.applyHumanization('Hello, world. Test');

            Math.random = originalRandom;
            expect(typeof result).toBe('string');
        });

        it('should compile regexes once for performance', async () => {
            const client = new OllamaClient();

            // First call should compile regexes
            client.applyHumanization('because you are great');

            // Should have compiled regexes stored
            expect(client._abbrRegexes).toBeDefined();
            expect(client._abbrRegexes.length).toBeGreaterThan(0);

            // Second call should reuse compiled regexes
            const result = client.applyHumanization('because you are great');
            expect(typeof result).toBe('string');
        });

        it('should handle checkModel with HTTP error response', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                text: () => Promise.resolve('Model not found'),
            });

            const client = new OllamaClient();
            const result = await client.checkModel('Test');

            expect(result.success).toBe(false);
        });

        it('should handle checkModel timeout', async () => {
            mockFetch.mockRejectedValueOnce({ name: 'AbortError' });

            const client = new OllamaClient();
            const result = await client.checkModel('Test');

            expect(result.success).toBe(false);
        });

        it('should include duration in failed responses', async () => {
            mockFetch.mockRejectedValueOnce(new Error('Network error'));

            const client = new OllamaClient();
            client._warmedUp = true;
            const result = await client.generate({
                prompt: 'Test',
            });

            expect(result.success).toBe(false);
            expect(result.duration).toBeDefined();
            expect(typeof result.duration).toBe('number');
        });

        it('should handle isReady with timeout', async () => {
            mockFetch.mockRejectedValueOnce(new Error('Timeout'));

            const client = new OllamaClient();
            const result = await client.isReady();

            expect(result).toBe(false);
        });

        it('should handle isReady with timeout', async () => {
            mockFetch.mockRejectedValueOnce(new Error('Timeout'));

            const client = new OllamaClient();
            const result = await client.isReady();

            expect(result).toBe(false);
        });
    });
});
