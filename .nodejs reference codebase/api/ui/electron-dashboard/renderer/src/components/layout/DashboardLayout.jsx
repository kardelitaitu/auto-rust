import React from 'react';

const DashboardLayout = ({ children, header }) => {
    return (
        <div className="dashboard">
            <header>
                {header || (
                    <>
                        <h1>The Dashboard</h1>
                        <div className="status">
                            <span id="connection-status">Online</span>
                        </div>
                    </>
                )}
            </header>
            <main className="sessions-grid">{children}</main>
        </div>
    );
};

export default DashboardLayout;
