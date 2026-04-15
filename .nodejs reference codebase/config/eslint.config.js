/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import js from '@eslint/js';
import prettier from 'eslint-config-prettier';
import globals from 'globals';

export default [
    // 1. GLOBAL IGNORES (Must be its own object at the top!)
    {
        ignores: [
            'coverage/',
            'docs/',
            'dist/',
            'build/',
            '.aider.tags.cache.v4/',
            '.qodo/',
            'config/',
            'data/',
            'examples/',
            'local-agent/',
            'node_modules/',
            'screenshots/',
            'tools/',
            'backup/',
            'api/ui/',
            'api/coverage/',

            '**/*.min.js',
            '**/dist/**',
            '**/node_modules/**',
        ],
    },

    // 2. Base Configs
    js.configs.recommended,

    // 3. Your Custom Configuration
    {
        languageOptions: {
            ecmaVersion: 2022,
            sourceType: 'module',
            globals: {
                ...globals.node,
                ...globals.browser,
                ...globals.es2021,
                // Manually defining Vitest globals is fine,
                // but you can also use `globals.jest` or similar if supported.
                describe: 'readonly',
                it: 'readonly',
                expect: 'readonly',
                vi: 'readonly',
                beforeEach: 'readonly',
                afterEach: 'readonly',
                beforeAll: 'readonly',
                afterAll: 'readonly',
                test: 'readonly',
            },
        },
        rules: {
            // Disable unused-vars warnings
            'no-unused-vars': 'off',

            // Performance: Allow console.log, but warn so you don't miss them in Prod
            'no-console': 'off',

            // Speed: Turn off strict equality checks (as per your request)
            eqeqeq: 'off',

            // Warn on empty catch blocks
            'no-empty': ['warn', { allowEmptyCatch: false }],
        },
    },

    // 4. Prettier (Must be last to override others)
    prettier,
];
