/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Sensory Simulation (Ghost 3.0)
 * Injects noisy, realistic sensor data to hide automation.
 *
 * @module api/sensors
 */

import { getPage } from '../core/context.js';
import { mathUtils } from '../utils/math.js';

/**
 * Inject noisy battery and hardware sensors.
 * @returns {Promise<void>}
 */
export async function injectSensors() {
    const page = getPage();

    await page.addInitScript(
        ({ level, chargingTime, dischargingTime }) => {
            // 1. Realistic Battery Status
            if ('getBattery' in navigator) {
                const battery = {
                    charging: true,
                    level,
                    chargingTime,
                    dischargingTime,
                    addEventListener: () => {},
                };
                navigator.getBattery = () => Promise.resolve(battery);
            }

            // 2. Network Information API
            if ('connection' in navigator) {
                const connection = {
                    effectiveType: '4g',
                    rtt: 50,
                    downlink: 10,
                    saveData: false,
                    addEventListener: () => {},
                };
                Object.defineProperty(navigator, 'connection', {
                    get: () => connection,
                });
            }

            // 3. Mock DeviceOrientation and Motion
            window.DeviceOrientationEvent = window.DeviceOrientationEvent || function () {};
            window.DeviceMotionEvent = window.DeviceMotionEvent || function () {};
        },
        {
            level: mathUtils.gaussian(0.85, 0.1, 0.5, 1.0),
            chargingTime: mathUtils.randomInRange(0, 100),
            dischargingTime: Infinity,
        }
    );
}
