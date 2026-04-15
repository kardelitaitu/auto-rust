/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { createLogger } from './logger.js';

export default class HistoryCompactor {
    constructor() {
        this.targetLength = 20;
        this.logger = createLogger('HistoryCompactor');
    }

    compactHistory(actions) {
        if (!actions || actions.length === 0) {
            return {
                originalCount: 0,
                compactedCount: 0,
                compressionRatio: 1.0,
                summary: 'No actions recorded.',
            };
        }

        const originalCount = actions.length;

        if (originalCount <= this.targetLength) {
            const summary = this._generateSummary(actions);
            return {
                originalCount,
                compactedCount: originalCount,
                compressionRatio: 1.0,
                summary,
            };
        }

        const compacted = this._performCompaction(actions);
        const summary = this._generateSummary(compacted);
        const compactedCount = compacted.length;

        return {
            originalCount,
            compactedCount,
            compressionRatio: compactedCount / originalCount,
            summary,
        };
    }

    _generateSummary(actions) {
        if (!actions || actions.length === 0) {
            return 'No actions recorded.';
        }

        return actions
            .map((item, index) => {
                const successIcon = item.success ? '✓' : '✗';
                return `${index + 1}. ${successIcon} ${item.action} → ${item.target}`;
            })
            .join('\n');
    }

    _performCompaction(actions) {
        if (!actions || actions.length === 0) {
            return [];
        }

        const result = [];
        let currentGroup = null;

        for (const action of actions) {
            if (!currentGroup) {
                currentGroup = {
                    action: action.action,
                    targets: [action.target],
                    successes: action.success ? 1 : 0,
                    failures: action.success ? 0 : 1,
                    timestamps: [action.timestamp],
                };
            } else if (currentGroup.action === action.action) {
                currentGroup.targets.push(action.target);
                if (action.success) {
                    currentGroup.successes++;
                } else {
                    currentGroup.failures++;
                }
                currentGroup.timestamps.push(action.timestamp);
            } else {
                result.push(this._createGroupedEntry(currentGroup));
                currentGroup = {
                    action: action.action,
                    targets: [action.target],
                    successes: action.success ? 1 : 0,
                    failures: action.success ? 0 : 1,
                    timestamps: [action.timestamp],
                };
            }
        }

        if (currentGroup) {
            result.push(this._createGroupedEntry(currentGroup));
        }

        if (result.length > this.targetLength) {
            const prioritizedActions = [
                'click',
                'scroll',
                'navigate',
                'type',
                'goto',
                'wait',
                'select',
            ];
            const sorted = result.slice().sort((a, b) => {
                const aIdx = prioritizedActions.indexOf(a.action.replace(' (×', ''));
                const bIdx = prioritizedActions.indexOf(b.action.replace(' (×', ''));
                if (aIdx !== -1 && bIdx === -1) return -1;
                if (aIdx === -1 && bIdx !== -1) return 1;
                if (aIdx !== -1 && bIdx !== -1) return aIdx - bIdx;
                return 0;
            });
            const kept = new Set(
                sorted.slice(0, this.targetLength).map((r) => r.action + '|' + r.target)
            );
            return result
                .filter((r) => kept.has(r.action + '|' + r.target))
                .slice(0, this.targetLength);
        }

        return result;
    }

    _createGroupedEntry(group) {
        const uniqueTargets = [...new Set(group.targets)];
        const isSuccess = group.successes > group.failures;

        if (group.targets.length === 1) {
            return {
                action: group.action,
                target: group.targets[0],
                success: isSuccess,
                timestamp: group.timestamps[0],
            };
        }

        return {
            action: `${group.action} (×${group.targets.length})`,
            target:
                uniqueTargets.length > 1
                    ? `${uniqueTargets.length} different targets`
                    : group.targets[0],
            success: isSuccess,
            timestamp: group.timestamps[0],
        };
    }

    generateNarrativeSummary(actions) {
        if (!actions || actions.length === 0) {
            return 'No actions performed.';
        }

        const lines = [];
        lines.push(`Session involved ${actions.length} actions.`);

        const navigateCount = actions.filter((a) => a.action === 'navigate').length;
        const clickCount = actions.filter((a) => a.action === 'click').length;
        const typeCount = actions.filter((a) => a.action === 'type').length;

        if (navigateCount > 0) {
            lines.push(`Navigated to ${navigateCount} page(s).`);
        }
        if (clickCount > 0) {
            lines.push(`Performed ${clickCount} click(s).`);
        }
        if (typeCount > 0) {
            lines.push(`Typed into ${typeCount} field(s).`);
        }

        const failures = actions.filter((a) => !a.success);
        if (failures.length > 0) {
            lines.push(`Encountered ${failures.length} failure(s).`);
            const errorMessages = failures.filter((f) => f.error).map((f) => f.error);
            if (errorMessages.length > 0) {
                lines.push(`Errors: ${errorMessages.join(', ')}.`);
            }
        } else {
            lines.push('All actions succeeded.');
        }

        return lines.join('\n');
    }

    getStats(original, compacted) {
        const originalCount = original.length;
        const compactedCount = compacted.length;

        return {
            originalCount,
            compactedCount,
            compressionRatio: compactedCount / originalCount,
            tokenSavingsEstimate: (originalCount - compactedCount) * 10,
        };
    }
}
