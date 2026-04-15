/**
 * Health Status Badge Component
 * Displays colored status indicator
 */
import React from 'react';

export function HealthStatus({ status, showLabel = true }) {
    const config = {
        healthy: { icon: '🟢', color: '#22c55e', label: 'Healthy' },
        degraded: { icon: '🟡', color: '#eab308', label: 'Degraded' },
        unhealthy: { icon: '🔴', color: '#ef4444', label: 'Unhealthy' },
        unknown: { icon: '⚪', color: '#6b7280', label: 'Unknown' }
    };

    const { icon, color, label } = config[status] || config.unknown;

    return (
        <span 
            className="health-status"
            style={{ 
                color,
                display: 'inline-flex',
                alignItems: 'center',
                gap: '6px',
                fontWeight: 600
            }}
        >
            <span style={{ fontSize: '1.2em' }}>{icon}</span>
            {showLabel && <span>{label}</span>}
        </span>
    );
}

/**
 * Provider Card Component
 * Displays LLM provider health
 */
export function ProviderCard({ name, data, compact = false }) {
    const status = data.status || data.state?.toLowerCase() || 'unknown';
    const failureRate = data.failureRate || 'N/A';

    if (compact) {
        return (
            <div className="health-card compact">
                <div className="health-card-header">
                    <span className="provider-name">{name}</span>
                    <HealthStatus status={status} showLabel={false} />
                </div>
                <div className="health-card-stat">
                    <span className="stat-label">Failure:</span>
                    <span className="stat-value">{failureRate}</span>
                </div>
            </div>
        );
    }

    return (
        <div className="health-card">
            <div className="health-card-header">
                <span className="provider-name">{name}</span>
                <HealthStatus status={status} />
            </div>
            <div className="health-card-body">
                <div className="health-stat">
                    <span className="stat-label">Failure Rate</span>
                    <span className="stat-value">{failureRate}</span>
                </div>
                <div className="health-stat">
                    <span className="stat-label">State</span>
                    <span className="stat-value">{data.state || 'N/A'}</span>
                </div>
            </div>
        </div>
    );
}

/**
 * Browser Card Component
 * Displays browser connection health
 */
export function BrowserCard({ name, data, compact = false }) {
    const status = data.status || data.state?.toLowerCase() || 'unknown';
    const failures = data.details?.failures || data.failures || 0;
    const successes = data.details?.successes || data.successes || 0;

    if (compact) {
        return (
            <div className="health-card compact">
                <div className="health-card-header">
                    <span className="browser-name">{name}</span>
                    <HealthStatus status={status} showLabel={false} />
                </div>
                <div className="health-card-stat">
                    <span className="stat-label">Failures:</span>
                    <span className="stat-value">{failures}</span>
                </div>
            </div>
        );
    }

    return (
        <div className="health-card">
            <div className="health-card-header">
                <span className="browser-name">{name}</span>
                <HealthStatus status={status} />
            </div>
            <div className="health-card-body">
                <div className="health-stat">
                    <span className="stat-label">Failures</span>
                    <span className="stat-value">{failures}</span>
                </div>
                <div className="health-stat">
                    <span className="stat-label">Successes</span>
                    <span className="stat-value">{successes}</span>
                </div>
            </div>
        </div>
    );
}

/**
 * System Resources Component
 */
export function SystemResources({ system }) {
    const memory = system?.memory || {};
    const usagePercent = parseFloat(memory.usagePercent) || 0;

    const getMemoryColor = (percent) => {
        if (percent > 90) return '#ef4444';
        if (percent > 70) return '#eab308';
        return '#22c55e';
    };

    return (
        <div className="system-resources">
            <div className="system-card">
                <div className="system-label">Memory Usage</div>
                <div className="system-value">{memory.usagePercent || 'N/A'}</div>
                <div className="progress-bar">
                    <div 
                        className="progress-fill"
                        style={{ 
                            width: `${Math.min(usagePercent, 100)}%`,
                            background: getMemoryColor(usagePercent)
                        }}
                    />
                </div>
            </div>
            <div className="system-card">
                <div className="system-label">Uptime</div>
                <div className="system-value">{system?.uptime || 'N/A'}</div>
            </div>
        </div>
    );
}
