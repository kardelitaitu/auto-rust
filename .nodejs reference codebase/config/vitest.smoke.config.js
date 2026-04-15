import { defineConfig } from 'vitest/config';
import { resolve } from 'path';

const rootDir = resolve(__dirname, '..');

export default defineConfig({
    test: {
        globals: false,
        environment: 'node',
        // Specifically include only the smoke test
        include: ['api/tests/unit/smoke.test.js'],
        // Disable heavy setup and pools
        setupFiles: [],
        pool: 'vmThreads',
        poolOptions: {
            vmThreads: {
                isolate: false,
            },
        },
        // Disable coverage for the smoke run
        coverage: {
            enabled: false,
        },
        retry: 0,
        silent: true,
    },
    resolve: {
        alias: {
            '@tests': resolve(rootDir, './api/tests'),
            '@unit': resolve(rootDir, './api/tests/unit'),
            '@api': resolve(rootDir, './api'),
        },
    },
});
