import React from 'react';

const SessionItem = React.memo(({ session }) => {
    const statusClass =
        session.status === 'online'
            ? 'online'
            : session.status === 'offline'
              ? 'offline'
              : 'unknown';

    const currentTask = session.taskName || session.currentTask || session.processing || null;

    return (
        <div
            className={`session-box ${statusClass}`}
            style={{
                padding: '8px 10px',
                minHeight: 'auto',
                display: 'flex',
                flexDirection: 'row',
                alignItems: 'center',
                gap: '8px',
                fontSize: '11px',
            }}
        >
            {/* Status Indicator */}
            <div
                style={{
                    width: '8px',
                    height: '8px',
                    borderRadius: '50%',
                    background:
                        statusClass === 'online'
                            ? 'var(--accent-success)'
                            : statusClass === 'offline'
                              ? 'var(--accent-error)'
                              : 'var(--text-dim)',
                    flexShrink: 0,
                }}
            />

            {/* Session Name */}
            <div
                style={{
                    flex: '1',
                    minWidth: '0',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                    fontWeight: '400',
                    color: 'var(--text-primary)',
                    fontSize: '11px',
                }}
            >
                {session.name || 'Unknown'}
            </div>

            {/* Browser Type Badge */}
            {session.browserType && (
                <div
                    style={{
                        fontSize: '9px',
                        padding: '2px 5px',
                        background: 'rgba(255,255,255,0.05)',
                        borderRadius: '3px',
                        color: 'var(--text-dim)',
                        textTransform: 'uppercase',
                        flexShrink: 0,
                    }}
                >
                    {session.browserType}
                </div>
            )}

            {/* Workers */}
            <div
                style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '3px',
                    flexShrink: 0,
                    color: 'var(--text-secondary)',
                    fontSize: '10px',
                }}
            >
                <span
                    style={{
                        color:
                            session.activeWorkers > 0 ? 'var(--accent-primary)' : 'var(--text-dim)',
                        fontWeight: '400',
                    }}
                >
                    {session.activeWorkers || 0}
                </span>
                <span style={{ color: 'var(--text-dim)' }}>/</span>
                <span>{session.totalWorkers || 0}</span>
            </div>

            {/* Current Task */}
            <div
                style={{
                    maxWidth: '180px',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                    flexShrink: 0,
                    color: currentTask ? '#f97316' : 'var(--text-dim)',
                    fontSize: '12px',
                    fontWeight: '400',
                }}
            >
                {currentTask || '-'}
            </div>
        </div>
    );
});

export default SessionItem;
