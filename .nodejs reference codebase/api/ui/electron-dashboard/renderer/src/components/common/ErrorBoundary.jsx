import React from 'react';

class ErrorBoundary extends React.Component {
    constructor(props) {
        super(props);
        this.state = { hasError: false, error: null, errorInfo: null };
    }

    static getDerivedStateFromError(error) {
        return { hasError: true };
    }

    componentDidCatch(error, errorInfo) {
        console.error('[Dashboard] ErrorBoundary caught an error:', error, errorInfo);
        this.setState({
            error: error,
            errorInfo: errorInfo,
        });
    }

    render() {
        if (this.state.hasError) {
            return (
                <div
                    style={{
                        padding: '40px',
                        textAlign: 'center',
                        backgroundColor: '#1a1a2e',
                        color: '#fff',
                        minHeight: '100vh',
                        display: 'flex',
                        flexDirection: 'column',
                        justifyContent: 'center',
                        alignItems: 'center',
                    }}
                >
                    <h1 style={{ color: '#f97316', marginBottom: '16px' }}>⚠️ Dashboard Error</h1>
                    <p style={{ color: '#94a3b8', marginBottom: '24px', maxWidth: '500px' }}>
                        Something went wrong while rendering the dashboard. Please try refreshing
                        the page.
                    </p>
                    <button
                        onClick={() => window.location.reload()}
                        style={{
                            padding: '10px 24px',
                            backgroundColor: '#f97316',
                            color: '#fff',
                            border: 'none',
                            borderRadius: '6px',
                            cursor: 'pointer',
                            fontSize: '14px',
                            fontWeight: '600',
                        }}
                    >
                        Refresh Dashboard
                    </button>
                    {process.env.NODE_ENV === 'development' && this.state.error && (
                        <details
                            style={{
                                marginTop: '24px',
                                textAlign: 'left',
                                backgroundColor: '#0f0f1a',
                                padding: '16px',
                                borderRadius: '8px',
                                maxWidth: '800px',
                                width: '100%',
                            }}
                        >
                            <summary style={{ cursor: 'pointer', color: '#94a3b8' }}>
                                Error Details (Dev Only)
                            </summary>
                            <pre
                                style={{
                                    color: '#ef4444',
                                    fontSize: '12px',
                                    overflow: 'auto',
                                    marginTop: '12px',
                                }}
                            >
                                {this.state.error.toString()}
                                {this.state.errorInfo?.componentStack}
                            </pre>
                        </details>
                    )}
                </div>
            );
        }

        return this.props.children;
    }
}

export default ErrorBoundary;
