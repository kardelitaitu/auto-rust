# Baseline

- There is no repo-root helper for creating a consistent git stash checkpoint before handing work to another agent.
- Recovery after a broken handoff is manual today, which means agents have to remember the right `git stash` command and message format.
- The spec workflow docs mention handoff and handback, but they do not give a concrete checkpoint/restore path.
- `AGENTS.md` does not yet tell agents to snapshot the worktree before a risky handoff.
- The current gap is process safety, not runtime behavior.
