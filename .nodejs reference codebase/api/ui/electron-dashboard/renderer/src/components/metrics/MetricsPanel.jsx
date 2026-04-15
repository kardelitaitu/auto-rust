/**
 * Metrics Panel - displays CPU, Memory, Queue, and API metrics
 */
import React from 'react';
import MetricCard from './MetricCard';
import { safeGet } from '../../hooks/useSocketConnection';

const Icons = {
    CPU: '⚡',
    RAM: '🧠',
    Tasks: '📋',
    API: '🌐',
    Uptime: '⏱️',
    Queue: '📥',
};

const getAdaptiveColor = (value) => {
    if (value < 40) return 'var(--accent-success)';
    if (value < 80) return 'var(--accent-warning)';
    return 'var(--accent-error)';
};

export function MetricsPanel({
    system,
    cumulative,
    apiMetrics,
    totalActiveWork,
    cpuHistory,
    ramHistory,
    isCompact,
}) {
    const cpuUsage = safeGet(system, 'cpu.usage', 0);
    const ramPercent = safeGet(system, 'memory.percent', 0);

    return (
        <section
            style={{
                display: 'grid',
                gridTemplateColumns: isCompact ? 'repeat(2, 1fr)' : 'repeat(4, 1fr)',
                gap: '8px',
                paddingBottom: '12px',
                flexShrink: 0,
            }}
        >
            {/* CPU */}
            <div style={{ display: 'flex', minHeight: '140px' }}>
                <MetricCard
                    title="CPU"
                    value={cpuUsage}
                    unit="%"
                    icon={Icons.CPU}
                    color={getAdaptiveColor(cpuUsage)}
                    history={!isCompact ? cpuHistory : null}
                    maxValue={100}
                />
            </div>

            {/* Memory */}
            <div style={{ display: 'flex', minHeight: '140px' }}>
                <MetricCard
                    title="Memory"
                    value={`${safeGet(system, 'memory.used', 0).toFixed(1)}/${safeGet(system, 'memory.total', 0).toFixed(0)}`}
                    unit="GB"
                    icon={Icons.RAM}
                    color={getAdaptiveColor(ramPercent)}
                    history={!isCompact ? ramHistory : null}
                    maxValue={100}
                />
            </div>

            {/* Queue & Tasks */}
            {!isCompact && (
                <div
                    style={{ display: 'flex', flexDirection: 'column', gap: '8px', height: '100%' }}
                >
                    <div style={{ flex: 1 }}>
                        <MetricCard
                            title="Active Queue"
                            value={totalActiveWork}
                            icon={Icons.Queue}
                            color="var(--accent-warning)"
                        />
                    </div>
                    <div style={{ flex: 1 }}>
                        <MetricCard
                            title="Completed Tasks"
                            value={cumulative.completedTasks || 0}
                            icon={Icons.Tasks}
                            color="var(--accent-success)"
                        />
                    </div>
                </div>
            )}

            {/* API Health & Uptime */}
            {!isCompact && (
                <div
                    style={{ display: 'flex', flexDirection: 'column', gap: '8px', height: '100%' }}
                >
                    <div style={{ flex: 1 }}>
                        <MetricCard
                            title="API Health"
                            value={`${apiMetrics.successRate || 100}%`}
                            icon={Icons.API}
                            color={
                                apiMetrics.successRate >= 90
                                    ? 'var(--accent-success)'
                                    : apiMetrics.successRate >= 70
                                      ? 'var(--accent-warning)'
                                      : 'var(--accent-error)'
                            }
                        />
                    </div>
                    <div style={{ flex: 1 }}>
                        <MetricCard
                            title="Uptime"
                            value={Math.floor(cumulative.sessionUptimeMs / 1000 / 60) || 0}
                            unit="min"
                            icon={Icons.Uptime}
                            color="var(--accent-primary)"
                        />
                    </div>
                </div>
            )}
        </section>
    );
}

export default MetricsPanel;
