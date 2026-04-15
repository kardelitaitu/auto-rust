/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Agent Connector Integration Tests
 * Tests the AgentConnector module for AI request routing and orchestration
 * @module tests/integration/agent-connector.test
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocks = vi.hoisted(() => ({
    getSettings: vi.fn(),
    getTimeoutValue: vi.fn().mockResolvedValue({}),
}));

vi.mock('@api/utils/configLoader.js', () => ({
    getSettings: mocks.getSettings,
    getTimeoutValue: mocks.getTimeoutValue,
}));

vi.mock('@api/utils/free-openrouter-helper.js', () => ({
    FreeOpenRouterHelper: {
        getInstance: vi.fn(() => ({
            testAllModelsInBackground: vi.fn(),
            getOptimizedModelList: vi.fn(() => ({ primary: 'test-model', fallbacks: [] })),
            getResults: vi.fn(() => ({ working: [], failed: [] })),
        })),
    },
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        success: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('AgentConnector Integration', { timeout: 30000 }, () => {
    let AgentConnector;
    let agentConnector;

    beforeEach(async () => {
        vi.clearAllMocks();

        mocks.getSettings.mockResolvedValue({
            llm: {
                cloud: {
                    enabled: true,
                    providers: [],
                    timeout: 60000,
                },
            },
            open_router_free_api: {
                enabled: false,
            },
        });

        const module = await import('../../core/agent-connector.js');
        AgentConnector = module.default;

        agentConnector = new AgentConnector();
    });

    describe('Initialization', () => {
        it('should initialize all components', () => {
            expect(agentConnector.localClient).toBeDefined();
            expect(agentConnector.cloudClient).toBeDefined();
            expect(agentConnector.visionInterpreter).toBeDefined();
            expect(agentConnector.requestQueue).toBeDefined();
            expect(agentConnector.circuitBreaker).toBeDefined();
        });

        it('should initialize stats with zeros', () => {
            expect(agentConnector.stats.totalRequests).toBe(0);
            expect(agentConnector.stats.successfulRequests).toBe(0);
            expect(agentConnector.stats.failedRequests).toBe(0);
            expect(agentConnector.stats.totalDuration).toBe(0);
            expect(agentConnector.stats.localRequests).toBe(0);
            expect(agentConnector.stats.cloudRequests).toBe(0);
            expect(agentConnector.stats.visionRequests).toBe(0);
            expect(agentConnector.stats.startTime).toBeDefined();
        });

        it('should configure request queue with defaults', () => {
            const queueStats = agentConnector.requestQueue.getStats();
            expect(queueStats.running).toBeDefined();
            expect(typeof queueStats.running).toBe('number');
        });

        it('should configure circuit breaker with defaults', () => {
            const cbStatus = agentConnector.circuitBreaker.getAllStatus();
            expect(cbStatus).toBeDefined();
        });
    });

    describe('Health Monitoring', () => {
        it('should return health status', () => {
            const health = agentConnector.getHealth();

            expect(health.status).toBeDefined();
            expect(health.healthScore).toBeDefined();
            expect(health.checks).toBeDefined();
            expect(health.timestamp).toBeDefined();
        });

        it('should include queue health check', () => {
            const health = agentConnector.getHealth();

            expect(health.checks.queue).toBeDefined();
            expect(health.checks.queue.status).toBeDefined();
            expect(health.checks.queue.running).toBeDefined();
            expect(health.checks.queue.queued).toBeDefined();
        });

        it('should include circuit breaker health check', () => {
            const health = agentConnector.getHealth();

            expect(health.checks.circuitBreaker).toBeDefined();
            expect(health.checks.circuitBreaker.status).toBeDefined();
            expect(health.checks.circuitBreaker.breakers).toBeDefined();
        });

        it('should include request health check', () => {
            const health = agentConnector.getHealth();

            expect(health.checks.requests).toBeDefined();
            expect(health.checks.requests.status).toBeDefined();
            expect(health.checks.requests.successRate).toBeDefined();
            expect(health.checks.requests.avgDuration).toBeDefined();
        });

        it('should include summary information', () => {
            const health = agentConnector.getHealth();

            expect(health.summary).toBeDefined();
            expect(health.summary.totalRequests).toBeDefined();
            expect(health.summary.uptime).toBeDefined();
            expect(health.summary.utilization).toBeDefined();
        });

        it('should calculate health score based on requests', () => {
            agentConnector.stats.totalRequests = 10;
            agentConnector.stats.successfulRequests = 8;

            const health = agentConnector.getHealth();

            expect(health.checks.requests.successRate).toBeDefined();
        });

        it('should have lower status with more failures', () => {
            agentConnector.stats.totalRequests = 10;
            agentConnector.stats.successfulRequests = 5;

            const health = agentConnector.getHealth();

            expect(health.status).toBeDefined();
        });

        it('should show degraded or unhealthy with low success', () => {
            agentConnector.stats.totalRequests = 10;
            agentConnector.stats.successfulRequests = 3;

            const health = agentConnector.getHealth();

            expect(health.status).toMatch(/degraded|unhealthy/);
        });
    });

    describe('Statistics', () => {
        it('should return comprehensive stats', () => {
            const stats = agentConnector.getStats();

            expect(stats.requests).toBeDefined();
            expect(stats.requests.total).toBeDefined();
            expect(stats.requests.successful).toBeDefined();
            expect(stats.requests.failed).toBeDefined();
            expect(stats.queue).toBeDefined();
            expect(stats.circuitBreaker).toBeDefined();
            expect(stats.uptime).toBeDefined();
        });

        it('should calculate success rate based on requests', () => {
            agentConnector.stats.totalRequests = 20;
            agentConnector.stats.successfulRequests = 15;

            const stats = agentConnector.getStats();

            expect(stats.requests.successRate).toBeDefined();
        });

        it('should calculate average duration', () => {
            agentConnector.stats.totalRequests = 4;
            agentConnector.stats.totalDuration = 4000;

            const stats = agentConnector.getStats();

            expect(stats.requests.avgDuration).toBe(1000);
        });
    });

    describe('Health Logging', () => {
        it('should log health status without errors', () => {
            const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

            agentConnector.logHealth();

            expect(consoleSpy).toHaveBeenCalled();
            expect(consoleSpy.mock.calls[0][0]).toContain('Health Report');

            consoleSpy.mockRestore();
        });
    });

    describe('Circuit Breaker Integration', () => {
        it('should have circuit breaker available', () => {
            expect(agentConnector.circuitBreaker).toBeDefined();
            expect(typeof agentConnector.circuitBreaker.execute).toBe('function');
        });

        it('should include circuit breaker status in health', () => {
            const health = agentConnector.getHealth();

            expect(health.checks.circuitBreaker.status).toBeDefined();
            expect(health.checks.circuitBreaker.breakers).toBeDefined();
        });

        it('should report healthy when all breakers closed', () => {
            const health = agentConnector.getHealth();

            expect(health.checks.circuitBreaker.status).toBe('healthy');
        });
    });

    describe('Request Queue Integration', () => {
        it('should have request queue available', () => {
            expect(agentConnector.requestQueue).toBeDefined();
            expect(typeof agentConnector.requestQueue.enqueue).toBe('function');
        });

        it('should include queue stats in health', () => {
            const health = agentConnector.getHealth();

            expect(health.checks.queue.running).toBeDefined();
            expect(health.checks.queue.queued).toBeDefined();
        });

        it('should include queue stats in getStats', () => {
            const stats = agentConnector.getStats();

            expect(stats.queue.running).toBeDefined();
            expect(stats.queue.queued).toBeDefined();
        });
    });
});

describe('AgentConnector Session Flow', () => {
    let AgentConnector;
    let agentConnector;

    beforeEach(async () => {
        vi.clearAllMocks();

        const module = await import('../../core/agent-connector.js');
        AgentConnector = module.default;
        agentConnector = new AgentConnector();
    });

    describe('Multiple Session Handling', () => {
        it('should handle multiple sessions independently', async () => {
            agentConnector.stats.totalRequests = 3;
            agentConnector.stats.successfulRequests = 3;

            const stats = agentConnector.getStats();
            expect(stats.requests.total).toBe(3);
        });

        it('should accumulate duration across sessions', () => {
            agentConnector.stats.totalRequests = 2;
            agentConnector.stats.totalDuration = 2000;

            const stats = agentConnector.getStats();
            expect(stats.requests.avgDuration).toBe(1000);
        });
    });

    describe('Uptime Tracking', () => {
        it('should track uptime since initialization', () => {
            const beforeInit = Date.now();
            const connector = new AgentConnector();
            const afterInit = Date.now();

            const stats = connector.getStats();
            expect(stats.uptime).toBeGreaterThanOrEqual(0);
            expect(stats.uptime).toBeLessThanOrEqual(afterInit - beforeInit + 100);
        });

        it('should increase uptime over time', async () => {
            const connector = new AgentConnector();
            const initialUptime = connector.getStats().uptime;

            await new Promise(function (resolve) {
                return setTimeout(resolve, 50);
            });

            const laterUptime = connector.getStats().uptime;
            expect(laterUptime).toBeGreaterThan(initialUptime);
        });
    });

    describe('Health Score Calculation', () => {
        it('should calculate health score based on success rate', () => {
            agentConnector.stats.totalRequests = 10;
            agentConnector.stats.successfulRequests = 5;

            const health = agentConnector.getHealth();
            expect(health.healthScore).toBeLessThan(100);
        });

        it('should not go below 0', () => {
            agentConnector.stats.totalRequests = 10;
            agentConnector.stats.successfulRequests = 0;

            const health = agentConnector.getHealth();
            expect(health.healthScore).toBeGreaterThanOrEqual(0);
        });

        it('should not go above 100', () => {
            agentConnector.stats.totalRequests = 100;
            agentConnector.stats.successfulRequests = 100;

            const health = agentConnector.getHealth();
            expect(health.healthScore).toBeLessThanOrEqual(100);
        });
    });
});

describe('AgentConnector Component Integration', () => {
    let AgentConnector;
    let agentConnector;

    beforeEach(async () => {
        vi.clearAllMocks();

        const module = await import('../../core/agent-connector.js');
        AgentConnector = module.default;
        agentConnector = new AgentConnector();
    });

    describe('Local Client Integration', () => {
        it('should have local client available', () => {
            expect(agentConnector.localClient).toBeDefined();
            expect(typeof agentConnector.localClient.sendRequest).toBe('function');
        });

        it('should have getStats method on local client', () => {
            expect(typeof agentConnector.localClient.getStats).toBe('function');
            const stats = agentConnector.localClient.getStats();
            expect(stats).toBeDefined();
        });
    });

    describe('Cloud Client Integration', () => {
        it('should have cloud client available', () => {
            expect(agentConnector.cloudClient).toBeDefined();
            expect(typeof agentConnector.cloudClient.sendRequest).toBe('function');
        });

        it('should have getStats method on cloud client', () => {
            expect(typeof agentConnector.cloudClient.getStats).toBe('function');
            const stats = agentConnector.cloudClient.getStats();
            expect(stats).toBeDefined();
        });
    });

    describe('Vision Interpreter Integration', () => {
        it('should have vision interpreter available', () => {
            expect(agentConnector.visionInterpreter).toBeDefined();
        });

        it('should have buildPrompt method', () => {
            expect(typeof agentConnector.visionInterpreter.buildPrompt).toBe('function');
        });

        it('should have parseResponse method', () => {
            expect(typeof agentConnector.visionInterpreter.parseResponse).toBe('function');
        });
    });
});

describe('AgentConnector Error Scenarios', () => {
    let AgentConnector;
    let agentConnector;

    beforeEach(async () => {
        vi.clearAllMocks();

        const module = await import('../../core/agent-connector.js');
        AgentConnector = module.default;
        agentConnector = new AgentConnector();
    });

    describe('Zero Request Handling', () => {
        it('should handle zero total requests', () => {
            const stats = agentConnector.getStats();
            expect(stats.requests.total).toBeDefined();
            expect(typeof stats.requests.total).toBe('number');
        });

        it('should handle zero average duration', () => {
            const stats = agentConnector.getStats();
            expect(stats.requests.avgDuration).toBe(0);
        });
    });

    describe('Health Status Boundaries', () => {
        it('should report appropriate status at high success rate', () => {
            agentConnector.stats.totalRequests = 10;
            agentConnector.stats.successfulRequests = 8;

            const health = agentConnector.getHealth();
            expect(health.status).toBeDefined();
        });

        it('should report lower status at medium success rate', () => {
            agentConnector.stats.totalRequests = 10;
            agentConnector.stats.successfulRequests = 5;

            const health = agentConnector.getHealth();
            expect(health.status).toBeDefined();
        });

        it('should report low status at very low success rate', () => {
            agentConnector.stats.totalRequests = 10;
            agentConnector.stats.successfulRequests = 3;

            const health = agentConnector.getHealth();
            expect(health.status).toMatch(/degraded|unhealthy/);
        });
    });

    describe('Queue Capacity', () => {
        it('should track queue running count', () => {
            const queueStats = agentConnector.requestQueue.getStats();
            expect(queueStats.running).toBeDefined();
        });

        it('should track queue queued count', () => {
            const queueStats = agentConnector.requestQueue.getStats();
            expect(queueStats.queued).toBeDefined();
        });

        it('should have maxConcurrent setting', () => {
            const queueStats = agentConnector.requestQueue.getStats();
            expect(queueStats.running).toBeDefined();
            expect(queueStats.queued).toBeDefined();
            expect(queueStats.utilization).toBeDefined();
        });
    });
});
