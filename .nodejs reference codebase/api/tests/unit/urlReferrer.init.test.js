/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import fs from 'fs';
import path from 'path';

vi.mock('fs');
vi.mock('path');
vi.mock('@api/index.js', () => ({
    api: {
        goto: vi.fn().mockResolvedValue(),
        click: vi.fn().mockResolvedValue(),
        waitForURL: vi.fn().mockResolvedValue(),
        setExtraHTTPHeaders: vi.fn().mockResolvedValue(),
    },
}));

describe('ReferrerEngine Initialization', () => {
    beforeEach(() => {
        vi.resetAllMocks();
        vi.resetModules();

        vi.mocked(path.resolve).mockImplementation((...args) =>
            args.filter(Boolean).join('/').replace(/\/+/g, '/')
        );
    });

    afterEach(() => {
        vi.restoreAllMocks();
    });

    it('should load dictionary values when file exists', async () => {
        const mockData = {
            TOPICS: ['mock_tech'],
            ACTIONS: ['mock_read'],
            CONTEXT: ['mock_thread'],
            SUBREDDITS: ['mock_sub'],
        };

        vi.mocked(fs.existsSync).mockImplementation((p) => {
            if (p.includes('ved_data.json')) return true;
            if (p.includes('referrer_dict.json')) return true;
            if (p.includes('tco_links.json')) return true;
            return false;
        });
        vi.mocked(fs.readFileSync).mockImplementation((p) => {
            if (!p) return '{}';
            const pathStr = String(p);
            if (pathStr.includes('referrer_dict.json')) return JSON.stringify(mockData);
            return '{}';
        });

        const { ReferrerEngine } = await import('@api/utils/urlReferrer.js');
        const engine = new ReferrerEngine();

        vi.spyOn(Math, 'random').mockReturnValue(0.2);

        const ctx = engine.generateContext('https://target.com');
        expect(ctx.strategy).toBe('google_search');
        const decoded = decodeURIComponent(ctx.referrer);

        expect(decoded).toMatch(/mock_tech|mock_read/);
    });

    it('should use emergency VEDs when VED dictionary fails to load', async () => {
        vi.mocked(fs.existsSync).mockImplementation((p) => {
            if (p.includes('ved_data.json')) return false;
            if (p.includes('referrer_dict.json')) return true;
            if (p.includes('tco_links.json')) return true;
            return false;
        });
        vi.mocked(fs.readFileSync).mockImplementation((p) => {
            if (!p) return '{}';
            const pathStr = String(p);
            if (pathStr.includes('ved_data.json')) throw new Error('File not found');
            return '{}';
        });

        const { ReferrerEngine } = await import('@api/utils/urlReferrer.js');
        const engine = new ReferrerEngine();

        vi.spyOn(Math, 'random').mockReturnValue(0.2);

        const ctx = engine.generateContext('https://target.com');
        expect(ctx.strategy).toBe('google_search');

        const vedMatch = ctx.referrer.match(/ved=([^&]+)/);
        expect(vedMatch).not.toBeNull();
        const ved = vedMatch[1];

        expect(ved).toMatch(/^0ahUKEwidhIC1qL2RAxWY1zgGHToAHtsQ/);
    });

    it('should warn when VED dictionary loading fails', async () => {
        vi.mocked(fs.existsSync).mockImplementation((p) => {
            if (p.includes('ved_data.json')) return true;
            if (p.includes('referrer_dict.json')) return true;
            if (p.includes('tco_links.json')) return true;
            return false;
        });
        vi.mocked(fs.readFileSync).mockImplementation((p) => {
            if (!p) return '{}';
            const pathStr = String(p);
            if (pathStr.includes('ved_data.json')) throw new Error('Read error');
            return '{}';
        });

        const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
        vi.resetModules();

        await import('../../utils/urlReferrer.js');

        const output = warnSpy.mock.calls.map((c) => c.join(' ')).join(' ');
        expect(output).toContain('VED Dictionary not loaded');
        warnSpy.mockRestore();
    });

    it('should error when Dictionary loading fails', async () => {
        vi.mocked(fs.existsSync).mockImplementation((p) => {
            if (p.includes('ved_data.json')) return true;
            if (p.includes('referrer_dict.json')) return true;
            if (p.includes('tco_links.json')) return true;
            return false;
        });
        vi.mocked(fs.readFileSync).mockImplementation((p) => {
            if (!p) return '{}';
            const pathStr = String(p);
            if (pathStr.includes('referrer_dict.json')) throw new Error('Read error');
            if (pathStr.includes('ved_data.json')) return '[]';
            if (pathStr.includes('tco_links.json')) return '[]';
            return '{}';
        });

        const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
        vi.resetModules();

        await import('../../utils/urlReferrer.js');

        const output = errorSpy.mock.calls.map((c) => c.join(' ')).join(' ');
        expect(output).toContain('Error loading dictionary');
        errorSpy.mockRestore();
    });

    it('should warn when t.co dictionary loading fails', async () => {
        vi.mocked(fs.existsSync).mockImplementation((p) => {
            if (p.includes('ved_data.json')) return true;
            if (p.includes('referrer_dict.json')) return true;
            if (p.includes('tco_links.json')) return true;
            return false;
        });
        vi.mocked(fs.readFileSync).mockImplementation((p) => {
            if (!p) return '{}';
            const pathStr = String(p);
            if (pathStr.includes('tco_links.json')) throw new Error('Read error');
            if (pathStr.includes('referrer_dict.json')) return '{}';
            if (pathStr.includes('ved_data.json')) return '[]';
            return '{}';
        });

        const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
        vi.resetModules();

        await import('../../utils/urlReferrer.js');

        const output = warnSpy.mock.calls.map((c) => c.join(' ')).join(' ');
        expect(output).toContain('t.co Dictionary not loaded');
        warnSpy.mockRestore();
    });
});
