/**
 * Clear History Confirmation Modal
 */
import React from 'react';

export function ClearConfirmModal({ isOpen, onConfirm, onCancel }) {
    if (!isOpen) return null;

    return (
        <div
            style={{
                position: 'fixed',
                top: 0,
                left: 0,
                right: 0,
                bottom: 0,
                backgroundColor: 'rgba(0, 0, 0, 0.8)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                zIndex: 1000,
            }}
        >
            <div
                className="glass"
                style={{
                    padding: '24px',
                    maxWidth: '400px',
                    textAlign: 'center',
                    border: '1px solid var(--accent-error)',
                }}
            >
                <h3 style={{ color: 'var(--accent-error)', marginBottom: '16px' }}>
                    Clear History?
                </h3>
                <p
                    style={{
                        color: 'var(--text-secondary)',
                        marginBottom: '24px',
                        fontSize: '14px',
                    }}
                >
                    This will reset all tasks, Twitter actions, and metrics. This action cannot be
                    undone.
                </p>
                <div style={{ display: 'flex', gap: '12px', justifyContent: 'center' }}>
                    <button
                        onClick={onCancel}
                        className="glass glass-interactive"
                        style={{
                            padding: '8px 16px',
                            border: '1px solid var(--glass-border)',
                            cursor: 'pointer',
                        }}
                    >
                        Cancel
                    </button>
                    <button
                        onClick={onConfirm}
                        style={{
                            padding: '8px 16px',
                            background: 'var(--accent-error)',
                            color: '#fff',
                            border: 'none',
                            borderRadius: '4px',
                            cursor: 'pointer',
                            fontWeight: '600',
                        }}
                    >
                        Clear All
                    </button>
                </div>
            </div>
        </div>
    );
}

export default ClearConfirmModal;
