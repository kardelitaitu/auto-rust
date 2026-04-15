/**
 * Twitter Stats Sidebar component
 */
import React from 'react';

export function TwitterStats({ twitterActions }) {
    const stats = [
        { label: 'Like', value: twitterActions.likes || 0 },
        { label: 'Retweet', value: twitterActions.retweets || 0 },
        { label: 'Reply', value: twitterActions.replies || 0 },
        { label: 'Quote', value: twitterActions.quotes || 0 },
        { label: 'Follow', value: twitterActions.follows || 0 },
        { label: 'Mark', value: twitterActions.bookmarks || 0 },
    ];

    return (
        <div
            className="glass"
            style={{ padding: '12px', flex: 1, display: 'flex', flexDirection: 'column' }}
        >
            <div
                style={{
                    fontSize: '9px',
                    color: 'var(--text-dim)',
                    marginBottom: '6px',
                    textTransform: 'uppercase',
                }}
            >
                Twitter
            </div>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '6px' }}>
                {stats.map(({ label, value }) => (
                    <div key={label} style={{ textAlign: 'center' }}>
                        <div style={{ fontSize: '16px', fontWeight: '500', color: '#f97316' }}>
                            {value}
                        </div>
                        <div style={{ fontSize: '10px', color: 'var(--text-dim)' }}>{label}</div>
                    </div>
                ))}
            </div>
        </div>
    );
}

export default TwitterStats;
