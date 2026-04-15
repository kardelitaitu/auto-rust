/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { getPage, withPage, getEvents } from './context.js';
import { setAutoBanners, setMuteAudio } from './context-state.js';
import { setPersona, getPersona, getPersonaName } from '../behaviors/persona.js';
import { apply as applyPatch, check as patchCheck } from '../utils/patch.js';
import { injectSensors } from '../utils/sensors.js';
import { startFidgeting } from '../interactions/cursor.js';
import { loadBuiltinPlugins, getPluginManager } from './plugins/index.js';
import { withErrorHook } from './hooks.js';
import { fingerprintManager } from '../utils/fingerprint.js';

import { applyHumanizationPatch } from '../utils/browserPatch.js';

const patchedPages = new WeakSet();
const hookedContexts = new WeakSet();

function safeEmitError(error, context) {
    try {
        const events = getEvents();
        events.emitSafe('on:error', { context, error });
    } catch {
        // Fallback if context/events not available
        console.error(`[Init Error] ${context}:`, error);
    }
}

function installContextHooks(page, options) {
    try {
        const context = page?.context?.();
        if (!context || hookedContexts.has(context)) {
            return;
        }

        hookedContexts.add(context);

        context.on('page', (newPage) => {
            initPage(newPage, options).catch((e) => safeEmitError(e, 'auto-init-page'));
        });

        if (typeof page.on === 'function') {
            page.on('popup', (popupPage) => {
                initPage(popupPage, options).catch((e) => safeEmitError(e, 'auto-init-popup'));
            });
        }
    } catch (e) {
        safeEmitError(e, 'installContextHooks');
    }
}

export async function initPage(page, options = {}) {
    if (!page) return;

    // Process initialization within the safety of the page's async local storage context
    return withPage(page, async () => {
        return withErrorHook('initPage', async () => {
            const {
                persona,
                personaOverrides,
                patch = true,
                humanizationPatch = true,
                autoInitNewPages = true,
                colorScheme,
                lite = false,
                blockNotifications = false,
                blockDialogs = false,
                autoBanners = false,
                muteAudio = false,
                logger,
            } = options;

            // Preserve autoBanners and mute state in context
            setAutoBanners(autoBanners);
            setMuteAudio(muteAudio);

            if (colorScheme) {
                try {
                    await page.emulateMedia({ colorScheme });
                } catch (e) {
                    safeEmitError(e, 'emulateMedia');
                }
            }

            if (lite) {
                try {
                    const blocklist = [
                        'google-analytics.com',
                        'googletagmanager.com',
                        'facebook.net',
                        'doubleclick.net',
                        'amazon-adsystem.com',
                        'adnxs.com',
                        'quantserve.com',
                        'scorecardresearch.com',
                        'crashlytics.com',
                        'hotjar.com',
                    ];

                    await page.route('**/*', (route) => {
                        const request = route.request();
                        const type = request.resourceType();
                        const url = request.url();

                        // Block images, media, fonts, and stylesheets to save max RAM/bandwidth
                        if (
                            [
                                'image',
                                'media',
                                'font',
                                'stylesheet',
                                'texttrack',
                                'manifest',
                            ].includes(type)
                        ) {
                            return route.abort().catch((e) => safeEmitError(e, 'lite-route-abort'));
                        }

                        // Aggressive script blocking for tracking/ads
                        if (type === 'script' || type === 'xhr' || type === 'fetch') {
                            if (blocklist.some((domain) => url.includes(domain))) {
                                return route
                                    .abort()
                                    .catch((e) => safeEmitError(e, 'lite-script-abort'));
                            }
                        }

                        route.continue().catch((e) => safeEmitError(e, 'lite-route-continue'));
                    });
                    if (logger)
                        logger.info(
                            'Ultra-Lite mode enabled: media, styles, and tracking scripts blocked'
                        );
                } catch (e) {
                    safeEmitError(e, 'lite-mode-route');
                }
            }

            if (blockNotifications) {
                try {
                    const context = page.context();
                    await context
                        .grantPermissions([], {
                            origin: page.url().split('/').slice(0, 3).join('/'),
                        })
                        .catch((e) => safeEmitError(e, 'grantPermissions'));
                } catch (e) {
                    safeEmitError(e, 'blockNotifications');
                }
            }

            if (blockDialogs) {
                page.on('dialog', async (dialog) => {
                    if (logger)
                        logger.debug(
                            `[Dialog] Automatically dismissing ${dialog.type()}: ${dialog.message()}`
                        );
                    await dialog.dismiss().catch((e) => safeEmitError(e, 'dialogDismiss'));
                });
            }

            if (muteAudio) {
                try {
                    await page.setMuted(true).catch((e) => safeEmitError(e, 'setMuted'));
                    if (logger) logger.info('Audio muted');
                } catch (e) {
                    safeEmitError(e, 'muteAudio');
                }
            }

            // Load plugins for this context
            loadBuiltinPlugins();
            try {
                getPluginManager().evaluateUrl(page.url());
            } catch (_e) {
                /* ignore */
            }

            if (persona) {
                setPersona(persona, personaOverrides || {});
            } else if (personaOverrides && Object.keys(personaOverrides).length > 0) {
                setPersona(getPersonaName(), personaOverrides);
            }

            if (!patch && !humanizationPatch) {
                return;
            }

            if (patchedPages.has(page)) {
                return;
            }

            if (autoInitNewPages) {
                installContextHooks(page, options);
            }

            if (patch) {
                let fp = options.fingerprint;
                if (!fp) {
                    try {
                        const ua = await page.evaluate(() => navigator.userAgent).catch(() => null);
                        fp = fingerprintManager.matchUserAgent(ua);
                    } catch (_e) {
                        fp = fingerprintManager.getRandom();
                    }
                }
                await applyPatch(fp);
            }

            if (humanizationPatch) {
                await applyHumanizationPatch(page, logger);
                await injectSensors();
                startFidgeting();
            }

            patchedPages.add(page);
        });
    });
}

/**
 * Clear any lite-mode resource blocking on the page.
 * @param {import('playwright').Page} [page] - Optional page instance, otherwise uses currently bound page
 */
export async function clearLiteMode(page) {
    const p = page || getPage();
    try {
        await p.unroute('**/*');
    } catch (e) {
        safeEmitError(e, 'clearLiteMode');
    }
}

export async function diagnosePage(page) {
    const p = page || getPage();

    return withPage(p, async () => {
        let patchStatus = null;
        try {
            patchStatus = await patchCheck();
        } catch (e) {
            safeEmitError(e, 'patchCheck');
        }

        let url = null;
        try {
            url = typeof p?.url === 'function' ? p.url() : null;
        } catch (e) {
            safeEmitError(e, 'diagnosePage-url');
        }

        return {
            url,
            personaName: getPersonaName(),
            persona: getPersona(),
            patch: patchStatus,
            patched: !!(p && patchedPages.has(p)),
        };
    });
}
