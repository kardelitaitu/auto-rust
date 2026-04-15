/**
 * Tab Content component - handles Status/History/Health tabs
 */
import React from 'react';
import SessionItem from '../sessions/SessionItem';
import TaskList from './TaskList';
import HealthTab from '../health/HealthTab';

export function TabContent({ activeTab, sessions, recentTasks, data }) {
    if (activeTab === 'fleet') {
        return (
            <div
                style={{
                    display: 'grid',
                    gridTemplateColumns: 'repeat(3, 1fr)',
                    gap: '4px',
                }}
            >
                {sessions.map((session) => (
                    <SessionItem key={session.id} session={session} />
                ))}
                {sessions.length === 0 && (
                    <div
                        style={{
                            padding: '20px',
                            textAlign: 'center',
                            color: 'var(--text-dim)',
                            gridColumn: 'span 3',
                        }}
                    >
                        No active sessions.
                    </div>
                )}
            </div>
        );
    }

    if (activeTab === 'health') {
        return <HealthTab data={data} />;
    }

    return <TaskList tasks={recentTasks || []} />;
}

export function TabHeader({ activeTab, setActiveTab, sessions, recentTasks, healthData }) {
    const healthStatus = healthData?.overall || 'unknown';
    const healthIcon = {
        healthy: '🟢',
        degraded: '🟡',
        unhealthy: '🔴',
        unknown: '⚪'
    }[healthStatus] || '⚪';

    return (
        <div
            style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                marginBottom: '12px',
                flexShrink: 0,
            }}
        >
            <div style={{ display: 'flex', gap: '4px' }}>
                <button
                    onClick={() => setActiveTab('fleet')}
                    style={tabButtonStyle(activeTab === 'fleet')}
                >
                    Status
                </button>
                <button
                    onClick={() => setActiveTab('health')}
                    style={tabButtonStyle(activeTab === 'health')}
                >
                    {healthIcon} Health
                </button>
                <button
                    onClick={() => setActiveTab('history')}
                    style={tabButtonStyle(activeTab === 'history')}
                >
                    History
                </button>
            </div>
            <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>
                {activeTab === 'fleet'
                    ? `${sessions.length} Browser(s) Discovered`
                    : activeTab === 'health'
                    ? `Health: ${healthStatus}`
                    : `${(recentTasks || []).length} Events`}
            </span>
        </div>
    );
}

function tabButtonStyle(isActive) {
    return {
        padding: '8px 16px',
        fontSize: '11px',
        fontWeight: '600',
        textTransform: 'uppercase',
        letterSpacing: '1px',
        background: isActive ? 'var(--accent-primary)' : 'transparent',
        color: isActive ? '#000' : 'var(--text-secondary)',
        border: '1px solid var(--glass-border)',
        borderRadius: '4px 4px 0 0',
        cursor: 'pointer',
        transition: 'all 0.1s',
    };
}

export default TabContent;
