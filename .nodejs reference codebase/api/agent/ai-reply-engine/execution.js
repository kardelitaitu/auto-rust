/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview AI Reply Engine - Execution Module
 * executeReply and reply method variants
 * @module utils/ai-reply-engine/execution
 */

import { api } from '../../index.js';
import { HumanInteraction } from '../../behaviors/human-interaction.js';

export async function executeReply(engine, page, replyText, _options = {}) {
    engine.logger.info(`[AIReply] Executing reply (${replyText.length} chars)...`);

    const human = new HumanInteraction(page);
    human.debugMode = true;

    const methods = [
        {
            name: 'keyboard_shortcut',
            weight: 40,
            fn: () => replyMethodA_Keyboard(engine, page, replyText, human),
        },
        {
            name: 'button_click',
            weight: 35,
            fn: () => replyMethodB_Button(engine, page, replyText, human),
        },
        {
            name: 'tab_navigation',
            weight: 15,
            fn: () => replyMethodC_Tab(engine, page, replyText, human),
        },
        {
            name: 'right_click',
            weight: 10,
            fn: () => replyMethodD_RightClick(engine, page, replyText, human),
        },
    ];

    const selected = human.selectMethod(methods);
    engine.logger.info(`[AIReply] Using method: ${selected.name}`);

    try {
        const result = await selected.fn();
        if (result && result.success === false) {
            engine.logger.warn(
                `[AIReply] Method ${selected.name} returned failure: ${result.reason || 'unknown_reason'}`
            );
            engine.logger.warn(`[AIReply] Trying fallback: button_click`);
            return await replyMethodB_Button(engine, page, replyText, human);
        }
        return result;
    } catch (error) {
        engine.logger.error(`[AIReply] Method ${selected.name} failed: ${error.message}`);
        engine.logger.warn(`[AIReply] Trying fallback: button_click`);
        return await replyMethodB_Button(engine, page, replyText, human);
    }
}

async function replyMethodA_Keyboard(engine, page, replyText, human) {
    human.logStep('KEYBOARD_SHORTCUT', 'Starting');

    const pageState = await page.evaluate(() => ({
        url: window.location.href,
        title: document.title,
        activeTag: document.activeElement?.tagName,
        activeAria: document.activeElement?.getAttribute('aria-label'),
        hasComposer: !!document.querySelector('[data-testid="tweetTextarea_0"]'),
    }));
    human.logStep(
        'PAGE_STATE',
        `URL: ${pageState.url.substring(0, 50)}... Active: ${pageState.activeTag} aria="${pageState.activeAria}"`
    );

    human.logStep('ESCAPE', 'Closing open menus');
    await page.keyboard.press('Escape');
    await api.wait(1000);

    const focusSelectors = [
        'article time',
        '[data-testid="tweetText"]',
        '[class*="tweetText"]',
        'article[role="article"] [dir="auto"]',
        'article[role="article"]',
    ];

    const focusMainTweet = async () => {
        human.logStep('FOCUS_MAIN', 'Focusing main tweet for keyboard shortcut');
        let focused = false;
        for (const selector of focusSelectors) {
            try {
                const el = page.locator(selector).first();
                if ((await el.count()) > 0) {
                    const success = await human.safeHumanClick(el, 'Main Tweet - Focus', 3, {
                        precision: 'high',
                    });
                    if (success) {
                        await api.wait(1000);
                        focused = true;
                        human.logStep('FOCUS_MAIN', `Clicked with ${selector}`);
                        break;
                    }
                }
            } catch (e) {
                human.logStep('FOCUS_MAIN_ERROR', e.message);
            }
        }
        if (!focused) {
            const viewport = page.viewportSize?.();
            const x = viewport ? Math.floor(viewport.width * 0.5) : 300;
            const y = viewport ? Math.floor(viewport.height * 0.35) : 300;
            await page.mouse.click(x, y);
            await api.wait(1000);
        }
    };

    await focusMainTweet();

    human.logStep('R_KEY', 'Opening reply composer');
    await page.keyboard.press('r');
    await api.wait(1000);

    const verify = await human.verifyComposerOpen(page);
    if (!verify.open) {
        human.logStep('VERIFY_FAILED', 'Composer did not open');
        await focusMainTweet();
        await page.keyboard.press('r');
        await api.wait(1000);
        const verify2 = await human.verifyComposerOpen(page);
        if (!verify2.open) {
            human.logStep('VERIFY_FAILED_2', 'Composer still not open after retry');
            return { success: false, reason: 'composer_not_open', method: 'keyboard_shortcut' };
        }
        return { success: true, method: 'keyboard_shortcut', selector: verify2.selector };
    }

    const composer = page.locator(verify.selector).first();
    await human.typeText(page, replyText, composer);

    const postResult = await human.postTweet(page, 'reply');

    return {
        success: postResult.success,
        reason: postResult.reason || 'posted',
        method: 'keyboard_shortcut',
        selector: verify.selector,
    };
}

async function replyMethodB_Button(engine, page, replyText, human) {
    human.logStep('BUTTON_CLICK', 'Starting reply via button');

    human.logStep('FIND_REPLY_BTN', 'Locating reply button');
    const replyBtnSelectors = [
        '[data-testid="reply"]',
        'button[aria-label*="Reply"]',
        'button:has-text("Reply")',
    ];

    let replyBtn = null;
    let btnSelector = '';

    for (const sel of replyBtnSelectors) {
        try {
            const btn = page.locator(sel).first();
            if ((await btn.count()) > 0) {
                replyBtn = btn;
                btnSelector = sel;
                break;
            }
        } catch (_e) {
            // Continue
        }
    }

    if (!replyBtn) {
        human.logStep('FIND_REPLY_BTN_FAIL', 'Reply button not found');
        return { success: false, reason: 'reply_button_not_found', method: 'button_click' };
    }

    human.logStep('CLICK_REPLY_BTN', `Clicking reply button: ${btnSelector}`);
    await human.safeHumanClick(replyBtn, 'Reply Button', 3);

    await api.wait(1000);

    const verify = await human.verifyComposerOpen(page);
    if (!verify.open) {
        human.logStep('VERIFY_FAILED', 'Composer did not open after button click');
        return { success: false, reason: 'composer_not_open', method: 'button_click' };
    }

    const composer = page.locator(verify.selector).first();
    await human.typeText(page, replyText, composer);

    const postResult = await human.postTweet(page, 'reply');

    return {
        success: postResult.success,
        reason: postResult.reason || 'posted',
        method: 'button_click',
    };
}

async function replyMethodC_Tab(engine, page, replyText, human) {
    human.logStep('TAB_NAVIGATION', 'Starting reply via tab navigation');

    human.logStep('TAB_PRESS', 'Pressing Tab to focus tweet');
    for (let i = 0; i < 8; i++) {
        await page.keyboard.press('Tab');
        await api.wait(1000);
    }

    await api.wait(1000);

    human.logStep('ENTER_PRESS', 'Pressing Enter to open reply');
    await page.keyboard.press('Enter');
    await api.wait(1000);

    const verify = await human.verifyComposerOpen(page);
    if (!verify.open) {
        human.logStep('VERIFY_FAILED', 'Composer did not open via tab');
        return { success: false, reason: 'composer_not_open', method: 'tab_navigation' };
    }

    const composer = page.locator(verify.selector).first();
    await human.typeText(page, replyText, composer);

    const postResult = await human.postTweet(page, 'reply');

    return {
        success: postResult.success,
        reason: postResult.reason || 'posted',
        method: 'tab_navigation',
    };
}

async function replyMethodD_RightClick(engine, page, replyText, human) {
    human.logStep('RIGHT_CLICK', 'Starting reply via right-click menu');

    const tweetSelectors = ['article[role="article"]', '[data-testid="tweet"]', 'article'];

    let tweetEl = null;
    for (const sel of tweetSelectors) {
        try {
            const el = page.locator(sel).first();
            if ((await el.count()) > 0) {
                tweetEl = el;
                break;
            }
        } catch (_e) {
            // Continue
        }
    }

    if (!tweetEl) {
        human.logStep('FIND_TWEET_FAIL', 'Could not find tweet element');
        return { success: false, reason: 'tweet_not_found', method: 'right_click' };
    }

    human.logStep('RIGHT_CLICK_TWEET', 'Right-clicking tweet');
    await tweetEl.click({ button: 'right' });
    await api.wait(1000);

    const replyOptionSelectors = [
        '[role="menuitem"]:has-text("Reply")',
        '[data-testid="reply"]',
        'button:has-text("Reply")',
    ];

    let replyOption = null;
    for (const sel of replyOptionSelectors) {
        try {
            const opt = page.locator(sel).first();
            if ((await opt.count()) > 0) {
                replyOption = opt;
                break;
            }
        } catch (_e) {
            // Continue
        }
    }

    if (!replyOption) {
        human.logStep('REPLY_OPTION_NOT_FOUND', 'Reply option not in context menu');
        await page.keyboard.press('Escape');
        await api.wait(1000);
    } else {
        human.logStep('CLICK_REPLY_OPTION', 'Clicking Reply in context menu');
        await replyOption.click();
        await api.wait(1000);
    }

    const verify = await human.verifyComposerOpen(page);
    if (!verify.open) {
        return { success: false, reason: 'composer_not_open', method: 'right_click' };
    }

    const composer = page.locator(verify.selector).first();
    await human.typeText(page, replyText, composer);

    const postResult = await human.postTweet(page, 'reply');

    return {
        success: postResult.success,
        reason: postResult.reason || 'posted',
        method: 'right_click',
    };
}
