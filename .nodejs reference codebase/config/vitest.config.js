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
// Module A: Hardware Topology & Execution Strategy
// ============================================================================
const cpuCount = cpus().length;
// Allow CI override, fallback to calculated threads (reserve 2 for OS/IDE stability)
const envMaxThreads = parseInt(process.env.VITEST_MAX_THREADS || '', 10);
const envMinThreads = parseInt(process.env.VITEST_MIN_THREADS || '', 10);
// Cap threads to avoid OOM while maintaining speed
const calculatedThreads = envMaxThreads || Math.max(4, Math.min(8, Math.floor(cpuCount / 4)));
const calculatedMinThreads = envMinThreads || 1;

// Only show system info if not silent
const isSilent = process.argv.includes('--silent') || process.env.VITEST_SILENT === 'true';
if (!isSilent) {
    console.log(`\n=== [SYSTEM_NODE] Test Execution Orchestrator ===`);
    console.log(`Hardware Detected: ${cpuCount} Logical Cores`);
    console.log(
        `Thread Allocation: ${calculatedThreads} Max Workers (env override: ${process.env.VITEST_MAX_THREADS || 'none'})`
    );
    console.log(`Coverage Engine: v8 (High Performance)`);
    console.log(`=================================================\n`);
}

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
            include: ['api/tests/unit/**/*.{test,spec}.{js,ts}'],
            exclude: [
                'node_modules',
                'dist',
                '.git',
                '.opencode',
                'api/ui/electron-dashboard/node_modules',
                'api/ui/electron-dashboard/renderer/node_modules',
            ],

        testTimeout: 10000,
        hookTimeout: 10000,
        cache: true,
        cacheDir: process.env.VITEST_CACHE_DIR || resolve(rootDir, 'node_modules/.vitest'),

        // --------------------------------------------------------------------
        // Concurrency Engine - Required: forks for AsyncLocalStorage isolation
        // --------------------------------------------------------------------
        pool: 'threads',
        poolOptions: {
            threads: {
                maxThreads: calculatedThreads,
                minThreads: calculatedMinThreads,
            },
        },
        fileParallelism: true,

        // --------------------------------------------------------------------
        // Coverage Engine - V8 Provider (Faster than Istanbul)
        // --------------------------------------------------------------------
        coverage: {
            provider: 'v8',
            reporter: ['text', 'json', 'html'],
            reportsDirectory: coverageRoot,
            clean: true,
            cleanOnRerun: true,
            include: ['core/**/*.js', 'utils/**/*.js', 'api/**/*.js'],
            exclude: [
                'node_modules/',
                'dist/',
                '.git/',
                // 'tests/',  // Commented out to include test utilities
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
                'api/tests/integration/',
            ],
            thresholds: {
                statements: 0,
                branches: 0,
                functions: 0,
                lines: 0,
                autoUpdate: false,
            },
        },

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
