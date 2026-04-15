/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect } from 'vitest';
import { withPage, getPlugins, getEvents } from '@api/core/context.js';
import { BasePlugin } from '@api/core/plugins/base.js';

describe('Plugin System Integration', () => {
    // Mock page factory
    const createMockPage = () => ({
        on: () => {},
        context: () => ({ browser: () => ({ isConnected: () => true }) }),
        isClosed: () => false,
        evaluate: async () => {},
    });

    class AuditPlugin extends BasePlugin {
        constructor() {
            super('audit-logger', '1.0.0');
            this.logs = [];
        }

        getHooks() {
            return {
                'before:click': async (selector) => {
                    this.logs.push(`Clicking ${selector}`);
                },
                'after:click': async (selector) => {
                    this.logs.push(`Clicked ${selector}`);
                },
            };
        }
    }

    class BlockerPlugin extends BasePlugin {
        constructor() {
            super('action-blocker', '1.0.0');
        }

        getHooks() {
            return {
                'before:click': async () => {
                    throw new Error('Action Blocked by Plugin');
                },
            };
        }
    }

    it('should register and execute plugin hooks in context', async () => {
        await withPage(createMockPage(), async () => {
            const plugins = getPlugins();
            const events = getEvents();

            const auditPlugin = new AuditPlugin();
            plugins.register(auditPlugin);

            expect(plugins.list()).toContain('audit-logger');
            expect(plugins.isEnabled('audit-logger')).toBe(true);

            // Simulate an action triggering events
            await events.emitAsync('before:click', '#btn');
            await events.emitAsync('after:click', '#btn');

            expect(auditPlugin.logs).toEqual(['Clicking #btn', 'Clicked #btn']);
        });
    });

    it('should handle plugin errors gracefully', async () => {
        await withPage(createMockPage(), async () => {
            const plugins = getPlugins();
            const events = getEvents();

            plugins.register(new BlockerPlugin());

            // The event system should catch the error and log it, returning null for that handler
            const results = await events.emitAsync('before:click', '#btn');
            expect(results).toContain(null);
        });
    });

    it('should isolate plugins between contexts', async () => {
        const context1Log = [];
        const context2Log = [];

        // Context 1
        await withPage(createMockPage(), async () => {
            const plugins = getPlugins();
            const p1 = new AuditPlugin();
            p1.logs = context1Log;
            plugins.register(p1);
            await getEvents().emitAsync('before:click', 'ctx1');
        });

        // Context 2
        await withPage(createMockPage(), async () => {
            const plugins = getPlugins();
            const p2 = new AuditPlugin();
            p2.logs = context2Log;
            plugins.register(p2);
            await getEvents().emitAsync('before:click', 'ctx2');
        });

        expect(context1Log).toEqual(['Clicking ctx1']);
        expect(context2Log).toEqual(['Clicking ctx2']);
    });
});
