/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for the new HealthMonitor class
 * Tests circuit-breaker based health monitoring
 * @module tests/unit/core/health-monitor.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock logger
vi.mock('@api/api/core/logger.js', () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

// Mock health-alerts
vi.mock('@api/api/core/health-alerts.js', () => ({
  recordStatusChange: vi.fn(),
  recordProviderFailure: vi.fn(),
}));

import { HealthMonitor, HealthStatus, getHealth } from '@api/core/health-monitor.js';

describe('api/core/health-monitor.js (NEW)', () => {
  let healthMonitor;
  let mockCircuitBreaker;

  beforeEach(() => {
    vi.clearAllMocks();
    
    // Create mock circuit breaker
    mockCircuitBreaker = {
      getAllStatus: vi.fn().mockReturnValue({
        'provider-1': { state: 'CLOSED', failureRate: '5%', failures: 1, successes: 19 },
        'provider-2': { state: 'OPEN', failureRate: '65%', failures: 13, successes: 7 },
      }),
      getHealth: vi.fn().mockImplementation((providerId) => {
        const status = {
          'provider-1': { status: 'healthy', failureRate: 5 },
          'unhealthy-provider': { status: 'unhealthy', failureRate: 65 },
        };
        return status[providerId] || null;
      }),
    };

    healthMonitor = new HealthMonitor({
      checkInterval: 0, // Disable automatic checks for tests
      maxHistory: 100,
    });
    
    healthMonitor.setCircuitBreaker(mockCircuitBreaker);
  });

  afterEach(() => {
    healthMonitor.destroy();
  });

  describe('constructor', () => {
    it('should create HealthMonitor instance with defaults', () => {
      const monitor = new HealthMonitor();
      
      expect(monitor).toBeInstanceOf(HealthMonitor);
      expect(monitor.checkInterval).toBe(30000);
      expect(monitor.maxHistory).toBe(100);
      expect(monitor.healthHistory).toEqual([]);
      
      monitor.destroy();
    });

    it('should accept custom options', () => {
      const monitor = new HealthMonitor({
        checkInterval: 10000,
        maxHistory: 50,
      });
      
      expect(monitor.checkInterval).toBe(10000);
      expect(monitor.maxHistory).toBe(50);
      
      monitor.destroy();
    });
  });

  describe('setCircuitBreaker', () => {
    it('should set circuit breaker instance', () => {
      const mockBreaker = { getAllStatus: vi.fn() };
      healthMonitor.setCircuitBreaker(mockBreaker);
      
      expect(healthMonitor.circuitBreaker).toBe(mockBreaker);
    });
  });

  describe('getHealth', () => {
    it('should return health structure', () => {
      const health = healthMonitor.getHealth();
      
      expect(health).toHaveProperty('overall');
      expect(health).toHaveProperty('timestamp');
      expect(health).toHaveProperty('lastCheck');
      expect(health).toHaveProperty('circuitBreakers');
      expect(health).toHaveProperty('browsers');
      expect(health).toHaveProperty('system');
    });

    it('should return UNKNOWN status initially', () => {
      const health = healthMonitor.getHealth();
      
      expect(health.overall).toBe(HealthStatus.UNKNOWN);
    });
  });

  describe('getCircuitBreakerHealth', () => {
    it('should return empty object without circuit breaker', () => {
      healthMonitor.setCircuitBreaker(null);
      const health = healthMonitor.getCircuitBreakerHealth();
      
      expect(health).toEqual({});
    });

    it('should return provider health data', () => {
      const health = healthMonitor.getCircuitBreakerHealth();
      
      expect(health).toHaveProperty('provider-1');
      expect(health).toHaveProperty('provider-2');
      
      expect(health['provider-1']).toEqual({
        state: 'CLOSED',
        failureRate: '5%',
        status: 'healthy',
        details: { failures: 1, successes: 19 }
      });
      
      expect(health['provider-2']).toEqual({
        state: 'OPEN',
        failureRate: '65%',
        status: 'unhealthy',
        details: { failures: 13, successes: 7 }
      });
    });
  });

  describe('getSystemHealth', () => {
    it('should return system metrics', () => {
      const system = healthMonitor.getSystemHealth();
      
      expect(system).toHaveProperty('memory');
      expect(system).toHaveProperty('uptime');
      expect(system).toHaveProperty('status');
      
      expect(system.memory).toHaveProperty('heapUsed');
      expect(system.memory).toHaveProperty('heapTotal');
      expect(system.memory).toHaveProperty('usagePercent');
    });

    it('should calculate memory status correctly', () => {
      const system = healthMonitor.getSystemHealth();
      const usagePercent = parseFloat(system.memory.usagePercent);
      
      if (usagePercent > 90) {
        expect(system.memory.status).toBe(HealthStatus.UNHEALTHY);
      } else if (usagePercent > 70) {
        expect(system.memory.status).toBe(HealthStatus.DEGRADED);
      } else {
        expect(system.memory.status).toBe(HealthStatus.HEALTHY);
      }
    });
  });

  describe('isProviderHealthy', () => {
    it('should return true without circuit breaker', () => {
      healthMonitor.setCircuitBreaker(null);
      expect(healthMonitor.isProviderHealthy('test')).toBe(true);
    });

    it('should return true for healthy provider', () => {
      mockCircuitBreaker.getAllStatus.mockReturnValue({
        'healthy-provider': { state: 'CLOSED', failureRate: '5%' }
      });
      
      expect(healthMonitor.isProviderHealthy('healthy-provider')).toBe(true);
    });

    it('should return false for unhealthy provider', () => {
      mockCircuitBreaker.getAllStatus.mockReturnValue({
        'unhealthy-provider': { state: 'OPEN', failureRate: '70%' }
      });
      
      expect(healthMonitor.isProviderHealthy('unhealthy-provider')).toBe(false);
    });
  });

  describe('getRecommendedProvider', () => {
    it('should return null without circuit breaker', () => {
      healthMonitor.setCircuitBreaker(null);
      expect(healthMonitor.getRecommendedProvider()).toBe(null);
    });

    it('should return provider with lowest failure rate', () => {
      mockCircuitBreaker.getAllStatus.mockReturnValue({
        'provider-a': { state: 'CLOSED', failureRate: '30%' },
        'provider-b': { state: 'CLOSED', failureRate: '5%' },
        'provider-c': { state: 'CLOSED', failureRate: '15%' },
      });
      
      expect(healthMonitor.getRecommendedProvider()).toBe('provider-b');
    });

    it('should return null when no healthy providers', () => {
      mockCircuitBreaker.getAllStatus.mockReturnValue({
        'provider-a': { state: 'OPEN', failureRate: '80%' },
        'provider-b': { state: 'OPEN', failureRate: '90%' },
      });
      
      expect(healthMonitor.getRecommendedProvider()).toBe(null);
    });
  });

  describe('performHealthCheck', () => {
    it('should update health status', () => {
      healthMonitor.performHealthCheck();
      
      expect(healthMonitor.lastCheck).toBeDefined();
      expect(healthMonitor.healthHistory.length).toBe(1);
    });

    it('should calculate overall health based on providers', () => {
      healthMonitor.performHealthCheck();
      
      const health = healthMonitor.getHealth();
      // With one healthy and one unhealthy provider, should be degraded
      expect(health.overall).toBe(HealthStatus.DEGRADED);
    });
  });

  describe('getHistory', () => {
    it('should return empty array initially', () => {
      expect(healthMonitor.getHistory()).toEqual([]);
    });

    it('should store health history', () => {
      healthMonitor.performHealthCheck();
      healthMonitor.performHealthCheck();
      
      const history = healthMonitor.getHistory();
      expect(history.length).toBe(2);
      expect(history[0]).toHaveProperty('timestamp');
      expect(history[0]).toHaveProperty('health');
    });

    it('should respect maxHistory limit', () => {
      const monitor = new HealthMonitor({ maxHistory: 5, checkInterval: 0 });
      
      for (let i = 0; i < 10; i++) {
        monitor.performHealthCheck();
      }
      
      expect(monitor.getHistory().length).toBe(5);
      monitor.destroy();
    });
  });

  describe('toJSON', () => {
    it('should return serializable health data', () => {
      const json = healthMonitor.toJSON();
      
      expect(json).toHaveProperty('status');
      expect(json).toHaveProperty('timestamp');
      expect(json).toHaveProperty('providers');
      expect(json).toHaveProperty('browsers');
      expect(json).toHaveProperty('system');
      expect(json).toHaveProperty('recommendations');
    });
  });

  describe('HealthStatus constants', () => {
    it('should have correct status values', () => {
      expect(HealthStatus.HEALTHY).toBe('healthy');
      expect(HealthStatus.DEGRADED).toBe('degraded');
      expect(HealthStatus.UNHEALTHY).toBe('unhealthy');
      expect(HealthStatus.UNKNOWN).toBe('unknown');
    });
  });

  describe('getHealth convenience function', () => {
    it('should return health from default monitor', () => {
      const health = getHealth();
      
      expect(health).toBeDefined();
      expect(health).toHaveProperty('overall');
    });
  });
});
