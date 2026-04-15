/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

describe('generateProfiles', () => {
    let writeFileSync;
    let consoleLog;
    let consoleError;

    beforeEach(() => {
        writeFileSync = vi.fn();
        consoleLog = vi.spyOn(console, 'log').mockImplementation(() => {});
        consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    });

    afterEach(() => {
        vi.resetModules();
        vi.clearAllMocks();
        consoleLog.mockRestore();
        consoleError.mockRestore();
    });

    it('should generate 50 profiles with required fields', async () => {
        vi.doMock('fs', () => ({
            default: { writeFileSync },
            writeFileSync,
        }));

        await import('../../utils/generateProfiles.js');

        expect(writeFileSync).toHaveBeenCalledTimes(1);
        const written = JSON.parse(writeFileSync.mock.calls[0][1]);

        expect(Array.isArray(written)).toBe(true);
        expect(written.length).toBe(50);

        const profile = written[0];
        expect(profile).toHaveProperty('id');
        expect(profile).toHaveProperty('description');
        expect(profile).toHaveProperty('timings');
        expect(profile).toHaveProperty('probabilities');
        expect(profile).toHaveProperty('inputMethods');
        expect(profile).toHaveProperty('maxLike');
        expect(profile).toHaveProperty('maxFollow');
        expect(profile).toHaveProperty('theme');
    });

    it('should include expected fields and profile types', async () => {
        vi.doMock('fs', () => ({
            default: { writeFileSync },
            writeFileSync,
        }));

        await import('../../utils/generateProfiles.js');

        const written = JSON.parse(writeFileSync.mock.calls[0][1]);
        const profile = written[0];

        expect(profile.probabilities).toHaveProperty('refresh');
        expect(profile.probabilities).toHaveProperty('profileDive');
        expect(profile.probabilities).toHaveProperty('tweetDive');
        expect(profile.probabilities).toHaveProperty('likeTweetafterDive');
        expect(profile.probabilities).toHaveProperty('bookmarkAfterDive');
        expect(profile.probabilities).toHaveProperty('followOnProfile');
        expect(profile.probabilities).toHaveProperty('tweet');
        expect(profile.probabilities).toHaveProperty('idle');

        expect(profile.timings).toHaveProperty('readingPhase');
        expect(profile.timings).toHaveProperty('scrollPause');
        expect(profile.timings).toHaveProperty('actionSpecific');
        expect(profile.timings.actionSpecific).toHaveProperty('space');
        expect(profile.timings.actionSpecific).toHaveProperty('keys');
        expect(profile.timings.actionSpecific).toHaveProperty('idle');

        const types = new Set(
            written.map((p) => {
                const match = p.id.match(/^\d+-(.+)$/);
                return match ? match[1] : '';
            })
        );

        expect(types.has('Skimmer')).toBe(true);
        expect(types.has('Balanced')).toBe(true);
        expect(types.has('DeepDiver')).toBe(true);
        expect(types.has('Lurker')).toBe(true);
        expect(types.has('DoomScroller')).toBe(true);
        expect(types.has('NewsJunkie')).toBe(true);
        expect(types.has('Stalker')).toBe(true);
    });

    it('should log error when file write fails', async () => {
        writeFileSync.mockImplementation(() => {
            throw new Error('disk full');
        });

        vi.doMock('fs', () => ({
            default: { writeFileSync },
            writeFileSync,
        }));

        await import('../../utils/generateProfiles.js');

        expect(consoleError).toHaveBeenCalled();
    });
});
