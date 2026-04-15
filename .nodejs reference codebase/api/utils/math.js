/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Math utilities for stochastic calculations.
 *
 * @module api/utils/math
 */

export const mathUtils = {
    /**
     * Box-Muller Transform — generates a number normally distributed around a mean.
     * @param {number} mean
     * @param {number} dev - Standard deviation
     * @param {number} [min]
     * @param {number} [max]
     * @returns {number}
     */
    gaussian: (mean, dev, min, max) => {
        let u = 0,
            v = 0;
        while (u === 0) u = Math.random();
        while (v === 0) v = Math.random();

        const z = Math.sqrt(-2.0 * Math.log(u)) * Math.cos(2.0 * Math.PI * v);
        let result = mean + z * dev;

        if (min !== undefined) result = Math.max(min, result);
        if (max !== undefined) result = Math.min(max, result);

        return Math.floor(result);
    },

    /**
     * Generates a random integer within [min, max] inclusive.
     * @param {number} min
     * @param {number} max
     * @returns {number}
     */
    randomInRange: (min, max) => {
        return Math.floor(Math.random() * (max - min + 1)) + min;
    },

    /**
     * Returns true if a random value is below the threshold.
     * @param {number} threshold - 0 to 1
     * @returns {boolean}
     */
    roll: (threshold) => {
        return Math.random() < threshold;
    },

    /**
     * Returns a random element from an array.
     * @param {Array} array
     * @returns {*}
     */
    sample: (array) => {
        if (!array || array.length === 0) return null;
        return array[Math.floor(Math.random() * array.length)];
    },

    /**
     * PID Controller step.
     * @param {object} state - { pos, integral, prevError }
     * @param {number} target
     * @param {object} model - { Kp, Ki, Kd }
     * @param {number} [dt=0.1]
     * @returns {number} New position
     */
    pidStep: (state, target, model, dt = 0.1) => {
        const error = target - state.pos;
        state.integral = (state.integral || 0) + error * dt;
        state.integral = Math.max(-10, Math.min(10, state.integral));
        const derivative = (error - (state.prevError || 0)) / dt;

        const output = model.Kp * error + model.Ki * state.integral + model.Kd * derivative;

        state.prevError = error;
        state.pos += output;
        return state.pos;
    },
};
