# Auto-AI Dashboard

An optional, non-intrusive Electron-based dashboard for monitoring Auto-AI browser sessions in real-time.

## Features

- **Real-time monitoring** of browser sessions
- **Compact design** (500x400px, resizable window)
- **Individual session boxes** with status indicators
- **WebSocket communication** for live updates
- **Self-contained** in `/ui` folder
- **Optional integration** - core system continues if dashboard fails

## Setup

### Prerequisites

- Node.js 16+ (for Electron)
- Auto-AI framework running

### Installation

1. Navigate to the dashboard folder:

    ```bash
    cd ui/electron-dashboard
    ```

2. Install dependencies:

    ```bash
    npm install
    ```

3. Start the dashboard:
    ```bash
    npm start
    ```

### Configuration

Enable dashboard in your Auto-AI `config/settings.json`:

```json
{
    "ui": {
        "dashboard": {
            "enabled": true,
            "port": 3001
        }
    }
}
```

## Usage

1. Start your Auto-AI automation:

    ```bash
    npm run start
    ```

2. Start the dashboard:

    ```bash
    cd ui/electron-dashboard && npm start
    ```

3. The dashboard will connect automatically and display:
    - Online/offline status of each browser session
    - Worker utilization
    - Current tasks and progress
    - Real-time updates every 2 seconds

## Architecture

### Non-Intrusive Design

The dashboard is completely optional and isolated:

- **Core system**: Continues running normally even if dashboard fails
- **Self-contained**: All UI code in `/ui/electron-dashboard/` folder
- **WebSocket communication**: Real-time data via Socket.io
- **Graceful degradation**: Dashboard connection is optional

### File Structure

```
ui/electron-dashboard/
├── main.js              # Electron main process
├── preload.js           # IPC bridge
├── renderer/
│   ├── index.html       # Dashboard UI
│   ├── style.css        # Styling
│   └── script.js        # Frontend logic
├── package.json         # Electron dependencies
└── dashboard.js         # WebSocket server
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
