import React from 'react';

const Sparkline = ({ data, color, height = 120, maxValue }) => {
    if (!data || data.length < 2) return null;
    const max = maxValue || Math.max(...data, 1);
    const min = 0; // Bars start from 0 for clarity
    const range = max - min || 1;
    const width = 120; // Fixed width for the viewBox coordinate system

    const minSlots = 25;
    const totalSlots = Math.max(data.length, minSlots);
    const barGapRatio = 0.2; // 20% of space is gap
    const step = width / totalSlots;
    const barWidth = step * (1 - barGapRatio);

    // Right-align the data if it's less than minSlots
    const xOffset = (totalSlots - data.length) * step;

    return (
        <div
            style={{
                position: 'absolute',
                bottom: 0,
                left: 0,
                right: 0,
                height: `${height}px`,
                opacity: 0.3,
                pointerEvents: 'none',
            }}
        >
            <svg
                width="100%"
                height="100%"
                viewBox={`0 0 ${width} ${height}`}
                preserveAspectRatio="none"
            >
                {data.map((val, i) => {
                    const barHeight = ((val - min) / range) * height;
                    const x = xOffset + i * step;
                    const y = height - barHeight;
                    return (
                        <rect
                            key={i}
                            x={x}
                            y={y}
                            width={barWidth}
                            height={Math.max(barHeight, 1)} // Ensure at least 1px visible for low values
                            fill={color}
                            rx={barWidth / 4} // Subtle rounding
                        />
                    );
                })}
            </svg>
        </div>
    );
};

const MetricCard = ({
    title,
    value,
    unit,
    icon,
    trend,
    color = 'var(--accent-primary)',
    history,
    maxValue,
}) => {
    return (
        <div
            className="glass glass-interactive"
            style={{
                padding: '20px',
                display: 'flex',
                flexDirection: 'column',
                gap: '8px',
                position: 'relative',
                overflow: 'hidden',
                flex: 1,
                minHeight: '100px',
                boxSizing: 'border-box',
            }}
        >
            <div
                style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    position: 'relative',
                    zIndex: 1,
                }}
            >
                <span
                    style={{
                        fontSize: '12px',
                        color: 'var(--text-secondary)',
                        fontWeight: '500',
                        textTransform: 'uppercase',
                        letterSpacing: '0.5px',
                    }}
                >
                    {title}
                </span>
                {icon && <span style={{ color: color, opacity: 0.8 }}>{icon}</span>}
            </div>
            <div
                style={{
                    display: 'flex',
                    alignItems: 'baseline',
                    gap: '4px',
                    position: 'relative',
                    zIndex: 1,
                }}
            >
                <span style={{ fontSize: '24px', fontWeight: '700', color: 'var(--text-primary)' }}>
                    {value}
                </span>
                {unit && <span style={{ fontSize: '12px', color: 'var(--text-dim)' }}>{unit}</span>}
            </div>
            {trend && (
                <div
                    style={{
                        fontSize: '11px',
                        color: trend > 0 ? 'var(--accent-success)' : 'var(--accent-error)',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '4px',
                        position: 'relative',
                        zIndex: 1,
                    }}
                >
                    {trend > 0 ? '↑' : '↓'} {Math.abs(trend)}%
                </div>
            )}
            {history && <Sparkline data={history} color={color} maxValue={maxValue} />}
        </div>
    );
};

export default MetricCard;
