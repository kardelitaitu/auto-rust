# Quality Rules

- **Human-like Timing**: Ensure pauses between reply engagements are randomized and realistic (1-3s).
- **Limit Respect**: Sub-engagements MUST NOT bypass the session engagement limits.
- **Safety First**: If any error occurs during reply scanning, the bot should immediately proceed to `goto_home` to avoid getting stuck in a loop.
