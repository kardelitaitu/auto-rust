/**
 * Auto-AI Dashboard - Main Application Component
 * Refactored to use custom hooks and smaller components
 */
import React, { useState, useEffect, useCallback } from 'react';

// Hooks
import { useSocketConnection, safeGet } from './hooks/useSocketConnection';
import { useMetricsHistory } from './hooks/useMetricsHistory';
import { useElectronAPI } from './hooks/useElectronAPI';

// Layout & Components
import DashboardLayout from './components/layout/DashboardLayout';
import DashboardHeader from './components/common/DashboardHeader';
import ClearConfirmModal from './components/common/ClearConfirmModal';
import MetricsPanel from './components/metrics/MetricsPanel';
import TwitterStats from './components/metrics/TwitterStats';
import TabContent, { TabHeader } from './components/common/TabContent';

// Health Tab
import HealthTab from './components/health/HealthTab';
import './components/health/health.css';

function App() {
    // Custom hooks
    const { data, status, emit } = useSocketConnection();
    const { cpuHistory, ramHistory, updateHistory } = useMetricsHistory();
    const { isAlwaysOnTop, isCompact, toggleAlwaysOnTop, toggleCompact } = useElectronAPI();

    // Local state
    const [activeTab, setActiveTab] = useState('fleet');
    const [showClearConfirm, setShowClearConfirm] = useState(false);

    // Update history when new metrics arrive
    useEffect(() => {
        if (data?.system) {
            updateHistory(data);
        }
    }, [data, updateHistory]);

    // Derived data
    const system = data.system || {};
    const cumulative = data.cumulative || {};
    const twitterActions = safeGet(data, 'metrics.twitter.actions', {});
    const apiMetrics = safeGet(data, 'metrics.api', {});
    const queueLength = safeGet(data, 'queue.queueLength', 0);
    const activeTasksCount = safeGet(data, 'queue.activeTaskCount', 0);
    const totalActiveWork = queueLength + activeTasksCount;
    const sessions = data.sessions || [];

    // Handlers
    const handleClearConfirm = useCallback(() => {
        emit('clear-history');
        setShowClearConfirm(false);
    }, [emit]);

    return (
        <>
            <DashboardLayout
                header={
                    <DashboardHeader
                        status={status}
                        isAlwaysOnTop={isAlwaysOnTop}
                        isCompact={isCompact}
                        onToggleAlwaysOnTop={toggleAlwaysOnTop}
                        onToggleCompact={toggleCompact}
                        onClearClick={() => setShowClearConfirm(true)}
                    />
                }
            >
                {/* Top Level Vital Metrics */}
                <MetricsPanel
                    system={system}
                    cumulative={cumulative}
                    apiMetrics={apiMetrics}
                    totalActiveWork={totalActiveWork}
                    cpuHistory={cpuHistory}
                    ramHistory={ramHistory}
                    isCompact={isCompact}
                />

                {/* Main Content Area */}
                {!isCompact ? (
                    <section
                        style={{
                            display: 'flex',
                            gap: '16px',
                            flex: 1,
                            minHeight: 0,
                            overflow: 'hidden',
                        }}
                    >
                        {/* Left: Tabbed Content Area */}
                        <div
                            style={{
                                flex: 1,
                                display: 'flex',
                                flexDirection: 'column',
                                minWidth: 0,
                                overflow: 'hidden',
                            }}
                        >
                            <TabHeader
                                activeTab={activeTab}
                                setActiveTab={setActiveTab}
                                sessions={sessions}
                                recentTasks={data.recentTasks}
                                healthData={data.health}
                            />
                            <div
                                className="glass"
                                style={{
                                    flex: 1,
                                    padding: '16px',
                                    overflowY: 'auto',
                                    minHeight: '200px',
                                }}
                            >
                                <TabContent
                                    activeTab={activeTab}
                                    sessions={sessions}
                                    recentTasks={data.recentTasks}
                                    data={data}
                                />
                            </div>
                        </div>

                        {/* Right: Mission Control Stats */}
                        <div
                            style={{
                                width: '180px',
                                flexShrink: 0,
                                display: 'flex',
                                flexDirection: 'column',
                            }}
                        >
                            <div
                                style={{
                                    display: 'flex',
                                    flexDirection: 'column',
                                    gap: '10px',
                                    flex: 1,
                                }}
                            >
                                <TwitterStats twitterActions={twitterActions} />
                            </div>
                        </div>
                    </section>
                ) : (
                    /* Compact Mode */
                    <section
                        style={{
                            gridColumn: '1 / -1',
                            display: 'flex',
                            flexDirection: 'column',
                            gap: '12px',
                        }}
                    >
                        <div
                            style={{
                                display: 'grid',
                                gridTemplateColumns: 'repeat(auto-fill, minmax(150px, 1fr))',
                                gap: '12px',
                            }}
                        >
                            {sessions.map((session) => (
                                <div key={session.id} className="glass" style={{ padding: '12px' }}>
                                    <div style={{ fontSize: '12px', fontWeight: '500' }}>
                                        {session.profile || session.id}
                                    </div>
                                    <div style={{ fontSize: '10px', color: 'var(--text-dim)' }}>
                                        {session.status}
                                    </div>
                                </div>
                            ))}
                            {sessions.length === 0 && (
                                <div
                                    className="glass"
                                    style={{
                                        padding: '20px',
                                        textAlign: 'center',
                                        color: 'var(--text-dim)',
                                    }}
                                >
                                    No active sessions.
                                </div>
                            )}
                        </div>
                    </section>
                )}
            </DashboardLayout>

            <ClearConfirmModal
                isOpen={showClearConfirm}
                onConfirm={handleClearConfirm}
                onCancel={() => setShowClearConfirm(false)}
            />
        </>
    );
}

export default App;
