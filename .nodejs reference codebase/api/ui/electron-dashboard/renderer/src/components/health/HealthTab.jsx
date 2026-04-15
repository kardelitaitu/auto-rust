/**
 * Health Tab Component
 * Displays system health monitoring dashboard
 */
import React, { useState, useEffect } from 'react';
import { HealthStatus, ProviderCard, BrowserCard, SystemResources } from './HealthComponents';

function HealthTab({ data, isCompact = false }) {
    const [healthData, setHealthData] = useState(null);
    const [lastUpdate, setLastUpdate] = useState(null);

    // Update health data when new metrics arrive
    useEffect(() => {
        if (data?.health) {
            setHealthData(data.health);
            setLastUpdate(new Date(data.health.timestamp));
        }
    }, [data]);

    if (!healthData) {
        return (
            <div className="health-tab loading">
                <div className="loading-message">
                    <span className="loading-spinner">⏳</span>
                    <span>Loading health data...</span>
                </div>
            </div>
        );
    }

    const providers = healthData.providers || healthData.circuitBreakers || {};
    const browsers = healthData.browsers || {};
    const system = healthData.system || {};

    return (
        <div className={`health-tab ${isCompact ? 'compact' : ''}`}>
            {/* Overall Status */}
            <section className="health-section">
                <div className="health-header">
                    <h2>Overall Health Status</h2>
                    {lastUpdate && (
                        <span className="last-update">
                            Updated: {lastUpdate.toLocaleTimeString()}
                        </span>
                    )}
                </div>
                <div className="overall-status-container">
                    <HealthStatus status={healthData.overall} />
                </div>
            </section>

            {/* LLM Providers */}
            {Object.keys(providers).length > 0 && (
                <section className="health-section">
                    <h2>LLM Providers</h2>
                    <div className={`health-grid ${isCompact ? 'compact' : ''}`}>
                        {Object.entries(providers).map(([name, data]) => (
                            <ProviderCard
                                key={name}
                                name={name}
                                data={data}
                                compact={isCompact}
                            />
                        ))}
                    </div>
                </section>
            )}

            {/* Browser Connections */}
            {Object.keys(browsers).length > 0 && (
                <section className="health-section">
                    <h2>Browser Connections</h2>
                    <div className={`health-grid ${isCompact ? 'compact' : ''}`}>
                        {Object.entries(browsers).map(([name, data]) => (
                            <BrowserCard
                                key={name}
                                name={name}
                                data={data}
                                compact={isCompact}
                            />
                        ))}
                    </div>
                </section>
            )}

            {/* System Resources */}
            {system && Object.keys(system).length > 0 && (
                <section className="health-section">
                    <h2>System Resources</h2>
                    <SystemResources system={system} />
                </section>
            )}

            {/* Alerts Section */}
            {healthData.recommendations && healthData.recommendations.length > 0 && (
                <section className="health-section">
                    <h2>Recommendations</h2>
                    <div className="alerts-container">
                        {healthData.recommendations.map((alert, index) => (
                            <div 
                                key={index} 
                                className={`alert ${alert.type === 'warning' ? 'warning' : 'info'}`}
                            >
                                <span className="alert-icon">
                                    {alert.type === 'warning' ? '⚠️' : 'ℹ️'}
                                </span>
                                <span className="alert-message">{alert.message}</span>
                            </div>
                        ))}
                    </div>
                </section>
            )}
        </div>
    );
}

export default HealthTab;
