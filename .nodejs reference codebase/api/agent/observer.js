/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Agent Observer
 * Provides a semantic view of the page for LLMs.
 * @module api/agent/observer
 */

import { getPage } from '../core/context.js';
import { setStateAgentElementMap } from '../core/context-state.js';

/**
 * Process DOM elements to extract semantic information.
 * Extracted for testability.
 * @param {Document} doc - Document object (for testing)
 * @returns {object[]} Array of element objects
 */
export function processDomElements(doc = document) {
    const interactiveSelectors = [
        'a',
        'button',
        'input',
        'select',
        'textarea',
        '[role="button"]',
        '[role="link"]',
        '[role="checkbox"]',
        '[role="menuitem"]',
        '[role="tab"]',
        '[onclick]',
    ];

    const allElements = doc.querySelectorAll(interactiveSelectors.join(','));
    const results = [];
    let idCounter = 1;

    allElements.forEach((el) => {
        const style = doc.defaultView.getComputedStyle(el);
        if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0')
            return;

        const rect = el.getBoundingClientRect();
        if (rect.width === 0 || rect.height === 0) return;

        let label = (
            el.innerText?.trim() ||
            el.getAttribute('aria-label') ||
            el.getAttribute('placeholder') ||
            el.getAttribute('title') ||
            el.getAttribute('alt') ||
            el.value ||
            ''
        ).trim();

        if (label.length > 50) label = label.substring(0, 47) + '...';

        if (!label && !el.getAttribute('role')) {
            if (!el.getAttribute('data-testid')) return;
            label = `[${el.getAttribute('data-testid')}]`;
        }

        const role =
            el.tagName.toLowerCase() === 'a'
                ? 'link'
                : el.tagName.toLowerCase() === 'input'
                  ? el.type || 'input'
                  : el.tagName.toLowerCase();

        const id = idCounter++;
        el.setAttribute('data-agent-id', id.toString());

        results.push({
            id,
            role,
            label: label || '[no-label]',
            selector: `[data-agent-id="${id}"]`,
        });
    });

    return results;
}

/**
 * See - Generates a semantic map of interactive elements on the page.
 * Assigns temporary IDs to elements for easy interaction via api.agent.do().
 *
 * @param {object} [options]
 * @param {boolean} [options.compact=true] - If true, returns a concise string for LLM tokens
 * @returns {Promise<string|object[]>} Semantic map of the page
 * @example
 * const view = await api.agent.see();
 */
export async function see(options = {}) {
    const { compact = true } = options;
    const page = getPage();

    const elements = await page.evaluate(processDomElements);

    // Store in context state for api.agent.do()
    setStateAgentElementMap(elements);

    if (compact) {
        return elements.map((el) => `[${el.id}] ${el.role}: "${el.label}"`).join('\n');
    }

    return elements;
}

export default see;
