/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Twitter Follow Back Task
 * @module tasks/followback.js
 *
 * usage example
 * node main.js followback
 * node main.js followback=5
 */

const DEFAULT_TASK_TIMEOUT_MS = 4 * 60 * 1000;

import { api } from '../api/index.js';
import { createLogger } from '../api/core/logger.js';
import metricsCollector from '../api/utils/metrics.js';

export default async function followbackTask(page, payload) {
    const browserInfo = payload?.browserInfo || 'unknown_profile';
    const logger = createLogger('followback.js');

    const taskTimeoutMs = payload?.taskTimeoutMs || DEFAULT_TASK_TIMEOUT_MS;
    const startTime = Date.now();

    try {
        await api.withPage(
            page,
            async () => {
                await Promise.race([
                    (async () => {
                        await api.init(page, {
                            humanizationPatch: true,
                            sensors: true,
                            logger,
                        });

                        const startupJitter = api.randomInRange(2000, 5000);
                        logger.info(`Warming up for ${startupJitter}ms...`);
                        await api.wait(startupJitter);

                        logger.info('Step 1: Navigating to x.com home...');
                        await api.goto('https://x.com/home', {
                            waitUntil: 'domcontentloaded',
                            timeout: 60000,
                        });

                        await api.waitVisible('[data-testid="primaryColumn"]', { timeout: 30000 });

                        logger.info('Step 2: Simulating reading for 1-2 minutes...');
                        const readTime = api.randomInRange(60000, 120000);
                        await api.scroll.read(null, {
                            pauses: Math.floor(readTime / 10000),
                            variableSpeed: true,
                        });

                        logger.info('Step 3: Clicking own profile...');
                        const profileLinkSelector = 'a[data-testid="AppTabBar_Profile_Link"]';

                        let profileClicked = false;
                        if (await api.exists(profileLinkSelector)) {
                            await api.click(profileLinkSelector, { precision: 'safe' });
                            profileClicked = true;
                        }

                        if (!profileClicked) {
                            // Try to extract username from profile link href
                            const profileHref = await page
                                .locator(profileLinkSelector)
                                .getAttribute('href')
                                .catch(() => null);
                            if (profileHref) {
                                logger.warn(
                                    `Profile link click failed, navigating to ${profileHref}`
                                );
                                await api.goto(`https://x.com${profileHref}`, {
                                    waitUntil: 'domcontentloaded',
                                });
                            } else {
                                throw new Error(
                                    'Could not find profile link — cannot proceed to followers'
                                );
                            }
                        }

                        await api.waitForLoadState('domcontentloaded');
                        await api.wait(api.randomInRange(2000, 4000));

                        logger.info('Step 4: Clicking followers link...');
                        const followersLinks = [
                            'a[href$="/verified_followers"]',
                            'a[href$="/followers"]',
                        ];

                        let followersClicked = false;
                        for (const selector of followersLinks) {
                            if (await api.exists(selector)) {
                                await api.click(selector, { precision: 'safe' });
                                followersClicked = true;
                                break;
                            }
                        }

                        if (!followersClicked) {
                            logger.warn('Followers link not found, attempting direct navigation');
                            const profileUrl = page.url();
                            const usernameMatch = profileUrl.match(/x\.com\/([^/?]+)/);
                            if (usernameMatch) {
                                await api.goto(`https://x.com/${usernameMatch[1]}/followers`, {
                                    waitUntil: 'domcontentloaded',
                                });
                            }
                        }

                        await api.waitForLoadState('domcontentloaded');
                        await api.wait(api.randomInRange(2000, 3000));

                        logger.info('Step 5: Clicking Followers tab...');
                        const followersTabSelector = 'a[role="tab"][href$="/followers"]';
                        await api.click(followersTabSelector, { precision: 'safe' });
                        logger.info('Navigated to Followers Tab');

                        await api.wait(api.randomInRange(1500, 2500));

                        logger.info('Step 6: Simulating human reading scroll on followers list...');
                        await api.scroll.read(null, {
                            pauses: api.randomInRange(5, 10),
                            scrollAmount: api.randomInRange(200, 600),
                        });

                        // Scroll back to top before clicking buttons
                        logger.info('Scrolling back to top...');
                        await api.scroll.toTop();
                        await api.wait(api.randomInRange(1000, 2000));

                        // Smart count: categorize follow buttons after scrolling
                        // CRITICAL: exclude -unfollow buttons explicitly ("unfollow" ends with "follow")
                        const followOnlySelector =
                            '[data-testid$="-follow"]:not([data-testid$="-unfollow"])';
                        const allButtons = page.locator(followOnlySelector);
                        const totalButtons = await allButtons.count();
                        let followBackBtns = 0,
                            genericFollowBtns = 0,
                            alreadyFollowing = 0;

                        for (let j = 0; j < totalButtons; j++) {
                            const btn = allButtons.nth(j);
                            const btnText = ((await btn.textContent()) || '').toLowerCase();
                            const btnAria = (
                                (await btn.getAttribute('aria-label')) || ''
                            ).toLowerCase();

                            if (
                                btnText.includes('following') ||
                                btnText.includes('pending') ||
                                btnAria.includes('following') ||
                                btnAria.includes('unfollow')
                            ) {
                                alreadyFollowing++;
                            } else if (
                                btnText.trim() === 'follow back' ||
                                btnAria.includes('follow back')
                            ) {
                                followBackBtns++;
                            } else if (
                                btnText.trim() === 'follow' ||
                                btnAria.includes('follow @')
                            ) {
                                genericFollowBtns++;
                            }
                        }
                        logger.info(
                            `Button scan: ${followBackBtns} Follow back, ${genericFollowBtns} Follow, ${alreadyFollowing} Already following (${totalButtons} total)`
                        );

                        logger.info('Step 7: Clicking "Follow" or "Follow back" buttons...');

                        let followBackCount = 0;

                        // Parse requested follows from payload (supports followback=N shorthand)
                        const requestedFollows = parseInt(
                            payload?.maxFollows || payload?.value || payload?.url
                        );
                        const maxFollows =
                            !isNaN(requestedFollows) && requestedFollows > 0 ? requestedFollows : 1;

                        logger.info(`Targeting ${maxFollows} follows.`);

                        const clickedTestIds = new Set(); // Track already-clicked buttons
                        let emptyScrollRounds = 0;
                        const maxEmptyScrolls = 30; // Stop after 30 scrolls with no new eligible buttons
                        const clickLoopDeadline = Date.now() + 3 * 60 * 1000; // 3-minute budget for Following back

                        while (
                            followBackCount < maxFollows &&
                            emptyScrollRounds < maxEmptyScrolls &&
                            Date.now() < clickLoopDeadline
                        ) {
                            // Re-scan buttons each round (live locator, excludes -unfollow)
                            const buttons = page.locator(followOnlySelector);
                            const count = await buttons.count();

                            let clickedThisRound = false;

                            for (let i = 0; i < count; i++) {
                                if (followBackCount >= maxFollows) break;

                                const btn = buttons.nth(i);
                                const testId = (await btn.getAttribute('data-testid')) || '';

                                // Skip if already clicked in a previous round (or empty testId)
                                if (!testId || clickedTestIds.has(testId)) continue;

                                if (await api.visible(btn)) {
                                    const text = ((await btn.textContent()) || '').toLowerCase();
                                    const ariaLabel = (
                                        (await btn.getAttribute('aria-label')) || ''
                                    ).toLowerCase();

                                    // CRITICAL: Skip if already following or pending
                                    if (
                                        text.includes('following') ||
                                        text.includes('pending') ||
                                        ariaLabel.includes('following') ||
                                        ariaLabel.includes('unfollow')
                                    ) {
                                        logger.info(
                                            `Skipping button ${i}: already following (text="${text.trim()}", aria="${ariaLabel.trim()}")`
                                        );
                                        continue;
                                    }

                                    // ONLY target "Follow back" buttons — skip generic "Follow" (sidebar recommendations)
                                    const trimmedText = text.trim();
                                    const isFollowBack =
                                        trimmedText === 'follow back' ||
                                        ariaLabel.includes('follow back');

                                    if (!isFollowBack) continue;

                                    const usernameMatch = ariaLabel.match(/@(\w+)/);
                                    const targetUser = usernameMatch
                                        ? usernameMatch[0]
                                        : 'unknown user';

                                    logger.info(
                                        `(${followBackCount + 1}/${maxFollows}) Clicking Follow back for ${targetUser}`
                                    );

                                    // Human-like scroll to button (Golden View focus)
                                    try {
                                        await api.scroll.focus(btn);
                                    } catch (_scrollErr) {
                                        logger.warn(
                                            `scroll.focus failed for ${targetUser}, using fallback`
                                        );
                                        await btn.scrollIntoViewIfNeeded();
                                    }
                                    await api.wait(api.randomInRange(300, 600));
                                    await api.click(btn, {
                                        precision: 'safe',
                                        hoverBeforeClick: true,
                                    });

                                    // Verify via data-testid change
                                    let verified = false;
                                    clickedTestIds.add(testId);
                                    if (testId) {
                                        const unfollowTestId = testId.replace(
                                            /-follow$/,
                                            '-unfollow'
                                        );
                                        const unfollowBtn = page.locator(
                                            `[data-testid="${unfollowTestId}"]`
                                        );
                                        const verifyStart = Date.now();

                                        while (Date.now() - verifyStart < 5000) {
                                            await api.wait(500);
                                            if ((await unfollowBtn.count()) > 0) {
                                                verified = true;
                                                break;
                                            }
                                        }
                                    }

                                    if (verified) {
                                        logger.info(`🟢 Followed ${targetUser} ✅✅✅`);
                                        followBackCount++;
                                    } else {
                                        logger.warn(
                                            `🔴 Could not verify follow for ${targetUser} — button did not change to "Following"`
                                        );
                                    }

                                    await api.wait(api.randomInRange(2000, 6000));
                                    clickedThisRound = true;
                                    break; // Re-scan from top after each click (list shifts)
                                }
                            }

                            // If no eligible button was found this round, scroll down to load more
                            if (!clickedThisRound) {
                                emptyScrollRounds++;
                                const lastBtn = buttons.last();
                                if ((await lastBtn.count()) > 0) {
                                    logger.info(
                                        `Scrolling to last button to load more... (attempt ${emptyScrollRounds}/${maxEmptyScrolls})`
                                    );
                                    await api.scroll(api.randomInRange(300, 500));
                                } else {
                                    logger.info(
                                        `No buttons found, scrolling down... (attempt ${emptyScrollRounds}/${maxEmptyScrolls})`
                                    );
                                    await api.scroll(api.randomInRange(300, 500));
                                }
                                await api.wait(api.randomInRange(2000, 3000));
                            } else {
                                emptyScrollRounds = 0; // Reset on success
                            }
                        }

                        if (Date.now() >= clickLoopDeadline && followBackCount < maxFollows) {
                            logger.warn(
                                `⏱️ Click loop 2-minute budget expired (followed ${followBackCount}/${maxFollows})`
                            );
                        }

                        if (followBackCount > 0) {
                            logger.info(
                                `✅ Successfully followed ${followBackCount}/${maxFollows} users`
                            );
                            metricsCollector.recordSocialAction('follow', followBackCount);
                        } else {
                            logger.warn('No eligible follow buttons found');
                        }

                        await api.wait(api.randomInRange(3000, 8000));

                        const elapsed = ((Date.now() - startTime) / 1000).toFixed(1);
                        logger.info(`Task completed in ${elapsed}s`);
                    })(),
                    new Promise((_, reject) =>
                        setTimeout(
                            () => reject(new Error(`Task timeout (${taskTimeoutMs}ms)`)),
                            taskTimeoutMs
                        )
                    ),
                ]);
            },
            { taskName: 'followback', sessionId: browserInfo }
        );
    } catch (error) {
        if (error.message.includes('Target page, context or browser has been closed')) {
            logger.warn('Task interrupted: Browser/Page closed');
        } else {
            logger.error('Error:', error.message);
        }
    } finally {
        // Page lifecycle managed by orchestrator — do not close here
    }
}
