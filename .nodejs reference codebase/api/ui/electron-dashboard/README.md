# Auto-AI Dashboard

An optional, non-intrusive Electron-based dashboard for monitoring Auto-AI browser sessions in real-time.

## Features

- **Real-time monitoring** of browser sessions
- **Compact design** (resizable window)
- **Individual session boxes** with status indicators
- **WebSocket communication** for live updates
- **Self-contained** in `/ui` folder
- **Optional integration** - core system continues if dashboard fails
- **Standalone mode** - run independently from orchestrator
- **Remote server support** - connect to dashboard server on different machine

## Setup

### Prerequisites

- Node.js 16+ (for Electron)
- Auto-AI framework running (optional - dashboard works standalone)

### Installation

1. Navigate to the dashboard folder:

    ```bash
    cd api/ui/electron-dashboard
    ```

2. Install dependencies:

    ```bash
    npm install
    ```

3. Start the dashboard:
    ```bash
    npm start
    ```

### Standalone Server Mode

The dashboard can run the WebSocket server embedded, or you can run it separately:

```bash
# Run server only (no Electron UI)
node start-server.js

# Run full Electron app (includes server)
npm start
```

### Configuration

Edit `config.json` in the dashboard folder:

```json
{
    "server": {
        "port": 3001,
        "host": "localhost"
    },
    "ui": {
        "defaultCompact": false,
        "defaultAlwaysOnTop": false
    },
    "connect": {
        "autoConnect": true,
        "remoteHost": "",
        "remotePort": 3001
    }
}
```

| Option                  | Description                                      | Default   |
| ----------------------- | ------------------------------------------------ | --------- |
| `server.port`           | WebSocket server port                            | 3001      |
| `server.host`           | Server host                                      | localhost |
| `ui.defaultCompact`     | Start in compact mode                            | false     |
| `ui.defaultAlwaysOnTop` | Always on top window                             | false     |
| `connect.remoteHost`    | Connect to remote server (leave empty for local) | ""        |
| `connect.remotePort`    | Remote server port                               | 3001      |

### Remote Server Connection

To connect to a dashboard server running on a different machine:

1. Set `connect.remoteHost` to the remote IP/hostname
2. Set `connect.remotePort` to the remote port
3. Leave `server.port` as is (won't be used)

Example for connecting to remote server at 192.168.1.100:3001:

```json
{
    "connect": {
        "remoteHost": "192.168.1.100",
        "remotePort": 3001
    }
}
```

## Usage

### Option 1: Full Dashboard (Server + UI)

```bash
cd api/ui/electron-dashboard
npm start
```

The Electron app will:

1. Start the WebSocket server on port 3001
2. Open the dashboard UI
3. Connect to itself via Socket.io

### Option 2: Server Only (for remote connections)

```bash
cd api/ui/electron-dashboard
node start-server.js
```

Then run the Electron app on another machine with remote config:

```json
{
    "connect": {
        "remoteHost": "192.168.1.100",
        "remotePort": 3001
    }
}
```

### Option 3: Connect to Auto-AI Orchestrator

If you want the dashboard to receive metrics from the Auto-AI orchestrator:

1. Start the dashboard server:

    ```bash
    node start-server.js
    ```

2. Ensure your orchestrator pushes metrics to the dashboard. The orchestrator will automatically detect the running dashboard server and connect to it.

## Architecture

### Non-Intrusive Design

The dashboard is completely optional and isolated:

- **Core system**: Continues running normally even if dashboard fails
- **Self-contained**: All UI code in `/ui/electron-dashboard/` folder
- **WebSocket communication**: Real-time data via Socket.io
- **Graceful degradation**: Dashboard connection is optional

### File Structure

```
api/ui/electron-dashboard/
├── dashboard.js         # WebSocket server (Express + Socket.io)
├── main.js              # Electron main process
├── preload.mjs          # IPC bridge (ES modules)
├── start-server.js      # Standalone server entry point
├── config.json          # Dashboard configuration
├── package.json         # Dependencies and scripts
├── vitest.config.js     # Test configuration
├── lib/
│   ├── history-manager.js  # Persistent history storage
│   └── logger.js           # ANSI-colored logging
├── renderer/            # React frontend
│   ├── src/
│   │   ├── App.jsx           # Main React component
│   │   ├── main.jsx          # React entry point
│   │   └── components/
│   │       ├── common/
│   │       │   ├── ErrorBoundary.jsx  # Error handling wrapper
│   │       │   └── TaskList.jsx       # Task history list
│   │       ├── layout/
│   │       │   └── DashboardLayout.jsx # Layout wrapper
│   │       ├── metrics/
│   │       │   └── MetricCard.jsx     # Metric display with sparkline
│   │       └── sessions/
│   │           └── SessionItem.jsx    # Session status display
│   └── dist/            # Built React app (production)
└── tests/               # Unit tests
```

### Communication Flow

1. **Auto-AI orchestrator** collects metrics and session data
2. **Dashboard server** (WebSocket) broadcasts data every 2 seconds
3. **Electron app** receives updates and renders session boxes
4. **User interface** displays real-time status and metrics

## Development

### Running in Development

```bash
cd ui/electron-dashboard
npm run dev
```

### Building for Production

```bash
cd ui/electron-dashboard
npm run build
```

This creates platform-specific executables in the `dist/` folder.

## Troubleshooting

### Dashboard Not Connecting

1. Check if Auto-AI is running with dashboard enabled
2. Verify port 3001 is available
3. Check firewall settings
4. Look at Auto-AI logs for dashboard errors

### Sessions Not Showing

1. Ensure browser sessions are active in Auto-AI
2. Check Auto-AI logs for session discovery
3. Verify dashboard is receiving data

### Performance Issues

1. Dashboard updates every 2 seconds (configurable)
2. Low memory footprint (single WebSocket connection)
3. No impact on core automation performance

## Security

- **Local only**: Dashboard runs on localhost
- **No external dependencies**: Self-contained Electron app
- **No data persistence**: Real-time monitoring only
- **Optional**: Can be disabled without affecting core functionality

### Authentication Note

Sensitive endpoints like `clear-history` are currently unprotected since the dashboard is designed for local use only. If exposing the dashboard to a network, consider adding authentication middleware:

```javascript
// Example: Add API key authentication
const API_KEY = process.env.DASHBOARD_API_KEY;

expressApp.use('/api', (req, res, next) => {
    const key = req.headers['x-api-key'] || req.query.apiKey;
    if (key !== API_KEY) {
        return res.status(401).json({ error: 'Unauthorized' });
    }
    next();
});
```

## License

MIT License - see LICENSE file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## Support

For issues and questions:

- Check the troubleshooting section
- Review the logs in Auto-AI
- Open an issue on the repository

---

**Note**: This dashboard is an optional enhancement for monitoring. The core Auto-AI automation continues to function normally even if the dashboard is not available or encounters errors.
