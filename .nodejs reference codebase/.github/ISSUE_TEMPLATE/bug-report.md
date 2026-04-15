---
name: 🐛 Bug Report
description: Report a bug or unexpected behavior
title: "[Bug]: "
labels: ["bug", "triage"]
assignees: []
---

## Bug Description

<!-- A clear and concise description of what the bug is. -->

## Reproduction Steps

<!-- Steps to reproduce the behavior: -->

1. 
2. 
3. 
4. 

## Expected Behavior

<!-- A clear and concise description of what you expected to happen. -->

## Actual Behavior

<!-- What actually happened? Include screenshots if applicable. -->

## Environment

<!-- Please complete the following information: -->

- **OS**: [e.g., Windows 11, macOS 14, Ubuntu 22.04]
- **Node.js Version**: [e.g., 18.17.0]
- **pnpm Version**: [e.g., 10.33.0]
- **Auto-AI Version**: [e.g., 1.2.0]
- **Browser**: [e.g., ixBrowser, Chrome 120, Brave]

## Configuration

<!-- Relevant configuration from .env and config/settings.json (remove sensitive values) -->

```env
# .env (remove API keys!)
LOCAL_LLM_ENDPOINT=
OPENROUTER_API_KEY=***
NODE_ENV=
```

```json
// config/settings.json (relevant sections)
{
  "llm": { ... },
  "humanization": { ... }
}
```

## Logs

<!-- Paste relevant logs below. Use `LOG_LEVEL=debug` for detailed logs. -->

```
Paste logs here
```

## Additional Context

<!-- Add any other context about the problem here. -->

## Possible Solution

<!-- Optional: If you have suggestions on how to fix the issue. -->

## Checklist

- [ ] I have searched existing issues for duplicates
- [ ] I have included steps to reproduce
- [ ] I have included environment details
- [ ] I have included logs (if applicable)
- [ ] I have removed sensitive information from logs
