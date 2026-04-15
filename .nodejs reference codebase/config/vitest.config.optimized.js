/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { defineConfig } from 'vitest/config';
import { mkdirSync, existsSync } from 'fs';
import { resolve } from 'path';
import { cpus } from 'os';

// ============================================================================
// Module A: Environment Detection & Hardware Topology
// ============================================================================
const isCI = !!process.env.CI;
const isWindows = process.platform === 'win32';
const cpuCount = cpus().length;

// Allow CI/human override via env vars, fallback to calculated values
const envMaxThreads = parseInt(process.env.VITEST_MAX_THREADS || '', 10);
const envMinThreads = parseInt(process.env.VITEST_MIN_THREADS || '', 10);

// CI: conservative (leave headroom for shared runners)
// Local: aggressive (reserve 1 core for main thread)
const defaultMaxThreads = isCI ? Math.max(4, cpuCount - 4) : Math.max(1, cpuCount - 1);
const defaultMinThreads = isCI
    ? Math.max(2, Math.floor(cpuCount / 4))
    : Math.max(1, Math.floor(cpuCount / 2));

const calculatedThreads = envMaxThreads || defaultMaxThreads;
const calculatedMinThreads = envMinThreads || Math.min(calculatedThreads, defaultMinThreads);

console.log(`\n=== [SYSTEM_NODE] Test Execution Orchestrator ===`);
console.log(`Environment: ${isCI ? 'CI' : 'Local'} (${isWindows ? 'Windows' : 'Linux/Mac'})`);
console.log(`Hardware Detected: ${cpuCount} Logical Cores`);
console.log(
    `Thread Allocation: ${calculatedThreads} Max / ${calculatedThreads === envMaxThreads ? 'env-override' : 'auto'} workers`
);
console.log(`Coverage Engine: v8 (native, fast)`);
console.log(`Retry (CI only): ${isCI ? 2 : 0} attempts`);
console.log(`=================================================\n`);

// ============================================================================
// Module B: Pathing & Directory Hygiene
// ============================================================================
const rootDir = resolve(__dirname, '..');
const coverageRoot = resolve(rootDir, 'api/coverage');

if (!existsSync(coverageRoot)) {
    mkdirSync(coverageRoot, { recursive: true });
}

// ============================================================================
// Module C: Core Vitest Configuration
// ============================================================================
export default defineConfig({
    test: {
        globals: false,
        environment: 'node',

        setupFiles: [resolve(rootDir, './api/tests/vitest.setup.js')],
        include: ['**/*.{test,spec}.{js,ts}'],
        exclude: [
            'node_modules',
            'dist',
            '.git',
            '.opencode',
            'api/ui/electron-dashboard/node_modules',
            'api/ui/electron-dashboard/renderer/node_modules',
        ],

        // Timeouts: generous for integration/browser tests
        testTimeout: 30_000,
        hookTimeout: 30_000,

        // Retry flaky tests in CI, never locally (hides real bugs)
        retry: isCI ? 2 : 0,

        cache: true,
        cacheDir: process.env.VITEST_CACHE_DIR || resolve(rootDir, 'node_modules/.vitest'),

        // --------------------------------------------------------------------
        // Concurrency Engine - Adaptive Thread Pool
        // --------------------------------------------------------------------
        pool: 'threads',
        poolOptions: {
            threads: {
                maxThreads: calculatedThreads,
                minThreads: calculatedMinThreads,
                isolate: true,
            },
        },
        fileParallelism: true,
        logHeapUsage: true,

        // --------------------------------------------------------------------
        // Coverage Engine - v8 Native (2-5x faster than istanbul)
        // --------------------------------------------------------------------
        coverage: {
            provider: 'v8',
            reporter: isCI ? ['text', 'json', 'lcov'] : ['text', 'json', 'html'],
            reportsDirectory: coverageRoot,
            clean: true,
            cleanOnRerun: true,
            include: ['core/**/*.js', 'utils/**/*.js', 'api/**/*.js'],
            exclude: [
                'node_modules/',
                'dist/',
                'git/',
                'tests/',
                'backup/',
                '**/*.test.js',
                '**/*.spec.js',
                '**/*.manual.js',
                'local-agent/',
                'api/ui/electron-dashboard/',
                'api/tests/unit/semantic-parser-runner.js',
                'api/tests/edge-cases/',
                'api/tests/dashboard-data-generator.js',
                'api/tests/simulate-task-history.js',
                'api/tests/mocks/index.js',
                'api/tests/integration/twitter-agent.test.js',
                '**/index.js',
            ],
            thresholds: {
                statements: 85,
                branches: 80,
                functions: 85,
                lines: 85,
                autoUpdate: false,
            },
        },

        // Reporter: dot for speed, verbose available via --reporter=verbose
        reporters: ['dot'],
    },

    resolve: {
        alias: {
            '@tests': resolve(rootDir, './api/tests'),
            '@unit': resolve(rootDir, './api/tests/unit'),
            '@integration': resolve(rootDir, './api/tests/integration'),
            '@edge-cases': resolve(rootDir, './api/tests/edge-cases'),
            '@api': resolve(rootDir, './api'),
            '@tasks': resolve(rootDir, './tasks'),
        },
    },
});
