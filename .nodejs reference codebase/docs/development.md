# Developer Guide

This guide covers internal development workflows for the Auto-AI framework.

## Development Setup

### Prerequisites

| Tool    | Version | Notes                |
| ------- | ------- | -------------------- |
| Node.js | 18+     | LTS recommended      |
| pnpm    | 8+      | Package manager      |
| Git     | 2.30+   | Version control      |
| Bun     | 1.0+    | For faster test runs |

### Initial Setup

```bash
# Clone and install
git clone https://github.com/kardelitaitu/auto-ai.git
cd auto-ai
pnpm install

# Copy environment template
copy .env-example .env
```

### Running Tests

```bash
# All tests (slow)
pnpm run test:all

# Fast unit tests (Bun - recommended)
pnpm run test:bun:unit

# With coverage
pnpm run test:bun:coverage

# Watch mode
pnpm run test:bun:watch
```

### Running Lint

```bash
# Check for issues
pnpm run lint

# Auto-fix
pnpm run lint:fix
```

## Git Workflow

### Branch Naming

- `feature/description` - New features
- `fix/description` - Bug fixes
- `refactor/description` - Code improvements
- `docs/description` - Documentation

### Committing

```bash
# Commit with auto-generated message
pnpm commit

# Commit with custom message
pnpm commit "Add new Twitter intent"

# Commit and push
pnpm commit "Add feature" --push
```

### Code Style

- Use ESLint for code quality
- Use Prettier for formatting (`pnpm run format`)
- Add JSDoc comments for new exports
- Keep functions under 50 lines when possible

## Project Structure

```
auto-ai/
├── api/                    # Core API
│   ├── core/              # Context, orchestrator, session
│   ├── interactions/      # Click, type, scroll, etc.
│   ├── behaviors/         # Humanization, timing
│   ├── agent/             # AI agent stack
│   └── utils/             # Helpers
├── tasks/                 # Runnable automation tasks
├── connectors/            # Browser adapters
├── config/                # Configuration files
├── docs/                  # Documentation
└── scripts/               # Build/run scripts
```

## Common Tasks

### Adding a New API Method

1. Add function to appropriate module in `api/interactions/` or `api/behaviors/`
2. Export in `api/index.js` with JSDoc `@example` blocks
3. Add tests in `api/tests/unit/`
4. Run `pnpm run lint:fix`

### Adding a New Task

1. Create file in `tasks/` directory
2. Export default async function
3. Add to task registry if needed

### Running a Task

```bash
# Page view
node main.js pageview=example.com

# Twitter follow
node main.js twitterFollow=https://twitter.com/user
```

## Debugging

### VS Code

Add to `.vscode/launch.json`:

```json
{
    "configurations": [
        {
            "type": "node",
            "request": "launch",
            "name": "Debug Main",
            "program": "${workspaceFolder}/main.js",
            "args": ["pageview=example.com"]
        }
    ]
}
```

### Browser Debugging

```bash
# Inspect mode
node --inspect-brk main.js
```

## Performance

### Benchmarks

```bash
# Run benchmarks
pnpm vitest run api/tests/benchmarks
```

### Profiling

```bash
# CPU profile
node --prof main.js

# Memory snapshot
node --inspect main.js
```

## Troubleshooting

### Tests failing after pull

```bash
# Clear node_modules and reinstall
rm -rf node_modules
pnpm install

# Clear vitest cache
rm -rf node_modules/.vite
```

### Port already in use

```bash
# Kill process on port
netstat -ano | findstr :3000
taskkill /PID <pid> /F
```

## Resources

- [API Reference](api.md)
- [Architecture](architecture.md)
- [Configuration](configuration.md)
- [AGENTS.md](../AGENTS.md)
