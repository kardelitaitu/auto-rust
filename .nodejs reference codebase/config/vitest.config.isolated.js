/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { defineConfig } from 'vitest/config';
import { mkdirSync, existsSync } from 'fs';
import { resolve } from 'path';
import { cpus } from 'os';

const cpuCount = cpus().length;
const envMaxThreads = parseInt(process.env.VITEST_MAX_THREADS || '', 10);
const envMinThreads = parseInt(process.env.VITEST_MIN_THREADS || '', 10);
const calculatedThreads = envMaxThreads || 2;
const calculatedMinThreads =
    envMinThreads || 1;

const isSilent = process.argv.includes('--silent') || process.env.VITEST_SILENT === 'true';
if (!isSilent) {
    console.log(`\n=== [ISOLATED_COVERAGE] Per-File Coverage Analysis ===`);
    console.log(`Hardware Detected: ${cpuCount} Logical Cores`);
    console.log(`Thread Allocation: ${calculatedThreads} Max Workers`);
    console.log(`Mode: Isolated per-file (forks pool)`);
    console.log(`========================================================\n`);
}

const rootDir = resolve(__dirname, '..');
const coverageRoot = resolve(rootDir, 'api/coverage-isolated');

if (!existsSync(coverageRoot)) {
    mkdirSync(coverageRoot, { recursive: true });
}

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

        testTimeout: 10000,
        hookTimeout: 10000,
        cache: true,
        cacheDir: process.env.VITEST_CACHE_DIR || resolve(rootDir, 'node_modules/.vitest-isolated'),

        // Forks pool: each test file runs in its own child process
        // Each process has its own Node.js module cache
        pool: 'forks',
        poolOptions: {
            forks: {
                maxForks: calculatedThreads,
                minForks: calculatedMinThreads,
                isolate: true,
            },
        },
        fileParallelism: false,
        isolate: true,

        coverage: {
            provider: 'v8',
            reporter: ['text', 'json', 'html'],
            reportsDirectory: coverageRoot,
            clean: true,
            cleanOnRerun: true,
            include: ['core/**/*.js', 'utils/**/*.js', 'api/**/*.js'],
            exclude: [
                'node_modules/',
                '__pycache__/',
                '_diag/',
                '.cline/',
                '.github/',
                '.opencode/',
                '.vscode/',
                '.husky/',
                'dist/',
                '.git/',
                'backup/',
                '**/*.test.js',
                '**/*.spec.js',
                '**/*.manual.js',
                'api/ui/health-dashboard/app.js',
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
            // Thresholds disabled for isolated testing - user can override via CLI
            thresholds: {
                statements: 0,
                branches: 0,
                functions: 0,
                lines: 0,
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
