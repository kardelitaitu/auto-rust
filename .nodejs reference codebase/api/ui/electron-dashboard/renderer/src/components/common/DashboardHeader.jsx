/**
 * Dashboard Header component with controls
 */
import React from 'react';

const APP_VERSION = '1.0.0';

export function DashboardHeader({
    status,
    isAlwaysOnTop,
    isCompact,
    onToggleAlwaysOnTop,
    onToggleCompact,
    onClearClick,
}) {
    return (
        <div
            style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                width: '100%',
            }}
        >
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                <h1>
                    The <span style={{ color: 'var(--accent-primary)' }}>Dashboard</span>
                </h1>
                <button
                    onClick={onToggleAlwaysOnTop}
                    className="glass glass-interactive"
                    style={{
                        padding: '4px 8px',
                        fontSize: '10px',
                        color: isAlwaysOnTop ? 'var(--accent-primary)' : 'var(--text-dim)',
                        border: '1px solid var(--glass-border)',
                    }}
                >
                    PIN {isAlwaysOnTop ? 'ON' : 'OFF'}
                </button>
                <button
                    onClick={onToggleCompact}
                    className="glass glass-interactive"
                    style={{
                        padding: '4px 8px',
                        fontSize: '10px',
                        color: 'var(--text-dim)',
                        border: '1px solid var(--glass-border)',
                    }}
                >
                    {isCompact ? 'EXPAND' : 'COMPACT'}
                </button>
                <button
                    onClick={onClearClick}
                    className="glass glass-interactive"
                    style={{
                        padding: '4px 8px',
                        fontSize: '10px',
                        color: 'var(--accent-error)',
                        border: '1px solid var(--glass-border)',
                    }}
                >
                    CLEAR
                </button>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '15px' }}>
                <div
                    className="status"
                    style={{ display: 'flex', alignItems: 'center', gap: '8px' }}
                >
                    <div className={`heartbeat ${status}`} />
                    <span className={status}>{status === 'online' ? 'Connected' : status}</span>
                </div>
                {!isCompact && (
                    <div
                        style={{
                            fontSize: '11px',
                            color: 'var(--text-dim)',
                            fontFamily: 'monospace',
                        }}
                    >
                        v{APP_VERSION}
                    </div>
                )}
            </div>
        </div>
    );
}

export default DashboardHeader;
