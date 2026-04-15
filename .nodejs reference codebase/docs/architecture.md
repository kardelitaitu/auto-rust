# Architecture

This document describes the system design and core concepts of Auto-AI.

## Overview

Auto-AI uses a layered architecture with clear separation of concerns:

```
┌─────────────────────────────────────────┐
│           Entry Points                 │
│  main.js | agent-main.js | api/index.js │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│          API Layer (api/)               │
│  ┌─────────────────────────────────┐   │
│  │    Core (context, orchestrator) │   │
│  ├─────────────────────────────────┤   │
│  │    Agent (AI decision engine)   │   │
│  ├─────────────────────────────────┤   │
│  │    Interactions (click, type)   │   │
│  ├─────────────────────────────────┤   │
│  │    Behaviors (humanization)     │   │
│  └─────────────────────────────────┘   │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│       Browser Layer (connectors/)       │
│  ixBrowser | MoreLogin | Dolphin | etc. │
└─────────────────────────────────────────┘
```

## Core Components

### Context (`api/core/context.js`)

Provides session isolation using Node.js `AsyncLocalStorage`. Each browser session runs in an isolated context with its own:

- Page instance
- Configuration
- State

```javascript
api.withPage(async (page) => {
    // This runs in an isolated context
});
```

### Orchestrator (`api/core/orchestrator.js`)

Manages the lifecycle:

- Browser discovery
- Task queueing
- Dispatch and execution
- Shutdown flow

### Session Manager (`api/core/sessionManager.js`)

Handles:

- Worker health monitoring
- Persistent session state
- Connection lifecycle

### Agent (`api/agent/`)

The AI agent stack:

- **Perception** - Vision, screen capture, DOM analysis
- **Reasoning** - LLM-powered decision making
- **Action** - Execution with rollback support

## Browser Connectors

Located in `connectors/`, each adapter:

- Discovers running browser instances
- Provides connection endpoints
- Handles browser-specific quirks

Supported browsers:

- ixBrowser
- MoreLogin
- Dolphin Anty
- Undetectable
- RoxyBrowser
- Local Brave/Chrome/Edge/Vivaldi

## Humanization

The behaviors layer (`api/behaviors/`) reduces detection risk:

- **Mouse Movement** - PID-based curves, not straight lines
- **Keystroke Dynamics** - Variable timing, typos
- **Scrolling** - Random patterns, momentum
- **Idle Behavior** - Realistic pauses
- **Attention** - Tab focus simulation

## Data Flow

```
Task Input → Orchestrator → Browser Discovery
                              ↓
                         Session Manager
                              ↓
                         Context (isolate)
                              ↓
                         Agent (decide)
                              ↓
                         Interactions (act)
                              ↓
                         Behaviors (humanize)
                              ↓
                         Browser via CDP
```

## Testing

Tests are organized in `api/tests/`:

- `unit/` - Unit tests for individual modules
- `integration/` - Integration tests for flows
- `edge-cases/` - Edge case and error handling tests

Run tests:

```bash
pnpm test:unit       # Unit tests
pnpm test:integration # Integration tests
pnpm test:all        # All tests
```

## More Details

- [API Reference](api.md) - Detailed API docs
- [Agent Documentation](../.agents/) - AI agent specifics
- [Stealth Protocol](../.agents/STEALTH-PROTOCOL.md) - Anti-detection details
