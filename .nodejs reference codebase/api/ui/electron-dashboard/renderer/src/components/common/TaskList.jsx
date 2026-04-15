import React from 'react';

const formatTimeAgo = (timestamp) => {
    if (!timestamp) return '';
    const seconds = Math.floor((Date.now() - timestamp) / 1000);

    if (seconds < 60) return `${seconds}s ago`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}min ${seconds % 60}s ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ${minutes % 60}min ${seconds % 60}s ago`;
    return new Date(timestamp).toLocaleDateString();
};

const TaskList = React.memo(({ tasks = [] }) => {
    // Use a ref to track the last update time for efficient re-renders
    const lastUpdateRef = React.useRef(Date.now());
    const [, forceUpdate] = React.useReducer((x) => x + 1, 0);

    React.useEffect(() => {
        // Only update if visible (optimization)
        const interval = setInterval(() => {
            lastUpdateRef.current = Date.now();
            forceUpdate();
        }, 30000);
        return () => clearInterval(interval);
    }, []);

    if (tasks.length === 0) {
        return (
            <div
                style={{
                    padding: '20px',
                    textAlign: 'center',
                    color: 'var(--text-dim)',
                    fontSize: '13px',
                }}
            >
                No recent tasks recorded
            </div>
        );
    }

    return (
        <div
            style={{
                display: 'flex',
                flexDirection: 'column',
                gap: '4px',
                overflowY: 'auto',
                overflowX: 'hidden',
                maxHeight: '100%',
            }}
        >
            {tasks
                .slice()
                .reverse()
                .map((task, index) => {
                    const session = task.sessionId || task.session || '-';
                    const name = task.taskName || task.name || task.command || '-';
                    const isSuccess =
                        task.success === true ||
                        task.status === 'completed' ||
                        task.status === 'OK';
                    const timeAgo = formatTimeAgo(task.addedAt || task.timestamp || task.updatedAt);

                    return (
                        <div
                            key={task.id || index}
                            style={{
                                fontSize: '11px',
                                color: 'var(--text-secondary)',
                                display: 'flex',
                                flexDirection: 'row',
                                alignItems: 'center',
                                gap: '4px',
                                fontFamily: 'monospace',
                                padding: '4px 0',
                                borderBottom: '1px solid hsla(0, 0%, 100%, 0.05)',
                                whiteSpace: 'nowrap',
                                width: '100%',
                                overflow: 'hidden',
                            }}
                        >
                            <span
                                style={{
                                    color: 'var(--text-dim)',
                                    minWidth: '70px',
                                    flexShrink: 0,
                                }}
                            >
                                {session}
                            </span>
                            <span style={{ flexShrink: 0 }}>-</span>
                            <span
                                style={{
                                    color: 'var(--text-primary)',
                                    flex: 1,
                                    overflow: 'hidden',
                                    textOverflow: 'ellipsis',
                                    textDecoration: isSuccess ? 'none' : 'line-through',
                                }}
                            >
                                {name}
                            </span>
                            <span style={{ flexShrink: 0 }}>-</span>
                            <span
                                style={{
                                    color: isSuccess
                                        ? 'var(--accent-success)'
                                        : 'var(--accent-error)',
                                    fontWeight: '700',
                                    minWidth: '50px',
                                    flexShrink: 0,
                                }}
                            >
                                {isSuccess ? 'OK' : 'FAILED'}
                            </span>
                            {timeAgo && (
                                <>
                                    <span style={{ flexShrink: 0 }}>-</span>
                                    <span
                                        style={{
                                            color: 'var(--text-dim)',
                                            fontSize: '10px',
                                            flexShrink: 0,
                                        }}
                                    >
                                        {timeAgo}
                                    </span>
                                </>
                            )}
                        </div>
                    );
                })}
        </div>
    );
});

export default TaskList;
