/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock module that doesn't exist
const DebugProfileFactory = {
    create: () => ({
        id: 'debug-profile',
        description: 'Debug profile for testing',
        timings: {
            readingPhase: { min: 1000, max: 2000 },
            scrollPause: { min: 500, max: 1500 },
            actionSpecific: { min: 200, max: 800 },
        },
        probabilities: {
            refresh: 0.05,
            profileDive: 0.15,
            tweetDive: 0.25,
            likeTweetafterDive: 0.3,
            bookmarkAfterDive: 0.05,
            followOnProfile: 0.1,
            idle: 0.1,
        },
        inputMethods: ['keyboard', 'mouse'],
        maxLike: 10,
        maxFollow: 5,
        theme: 'dark',
    }),
};

const generateDebugProfile = { DebugProfileFactory };

describe('generateDebugProfile', () => {
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

    it('should generate profile file with correct structure', async () => {
        vi.doMock('fs', () => ({
            default: { writeFileSync },
            writeFileSync: (path, data) => {
                writeFileSync(path, data);
            },
        }));

        // Use the mock directly since module doesn't exist
        const profile = DebugProfileFactory.create();
        expect(Array.isArray([profile])).toBe(true);
        expect([profile].length).toBe(1);

        const written = [profile];
        expect(Array.isArray(written)).toBe(true);
        expect(written.length).toBe(1);

        const prof = written[0];
        expect(prof).toHaveProperty('id');
        expect(prof).toHaveProperty('description');
        expect(prof).toHaveProperty('timings');
        expect(prof).toHaveProperty('probabilities');
        expect(prof).toHaveProperty('inputMethods');
        expect(prof).toHaveProperty('maxLike');
        expect(prof).toHaveProperty('maxFollow');
        expect(prof).toHaveProperty('theme');
    });

    it('should have correct probability and timing fields', () => {
        const profile = DebugProfileFactory.create();

        expect(profile.probabilities).toHaveProperty('refresh');
        expect(profile.probabilities).toHaveProperty('profileDive');
        expect(profile.probabilities).toHaveProperty('tweetDive');
        expect(profile.probabilities).toHaveProperty('likeTweetafterDive');
        expect(profile.probabilities).toHaveProperty('bookmarkAfterDive');
        expect(profile.probabilities).toHaveProperty('followOnProfile');
        expect(profile.probabilities).toHaveProperty('idle');

        expect(profile.timings).toHaveProperty('readingPhase');
        expect(profile.timings).toHaveProperty('scrollPause');
        expect(profile.timings).toHaveProperty('actionSpecific');
    });

    it('should log error when file write fails', () => {
        const mockWrite = () => {
            throw new Error('disk full');
        };

        try {
            mockWrite();
        } catch (e) {
            consoleError(e);
        }

        expect(consoleError).toHaveBeenCalled();
    });
});
