# Contributing Checklist

A step-by-step checklist for contributing to Auto-AI.

---

## Before You Start

- [ ] I have read [CONTRIBUTING.md](../CONTRIBUTING.md)
- [ ] I have read [AGENTS.md](../AGENTS.md)
- [ ] I have searched existing [issues](https://github.com/kardelitaitu/auto-ai/issues) for duplicates
- [ ] I have searched existing [pull requests](https://github.com/kardelitaitu/auto-ai/pulls) for related work

---

## Development Setup

- [ ] Forked the repository
- [ ] Cloned fork locally
- [ ] Installed Node.js 18+
- [ ] Installed pnpm 10+
- [ ] Ran `pnpm install`
- [ ] Copied `.env.example` to `.env`
- [ ] Verified tests pass: `pnpm run test:bun:unit`

---

## Making Changes

### Planning

- [ ] Created/linked to an issue describing the problem
- [ ] Discussed approach (for significant changes)
- [ ] Created a feature branch: `git checkout -b feat/description`

### Coding

- [ ] Followed existing code style
- [ ] Added JSDoc comments for new exports
- [ ] Kept functions focused (< 50 lines when possible)
- [ ] Used `api.*` methods instead of raw Playwright
- [ ] Added error handling
- [ ] Added logging for debugging

### Testing

- [ ] Added unit tests for new functionality
- [ ] Updated existing tests if behavior changed
- [ ] All tests pass: `pnpm run test:bun:unit`
- [ ] Integration tests pass: `pnpm run test:bun:integration`
- [ ] Coverage maintained or improved: `pnpm run test:bun:coverage`

### Code Quality

- [ ] Lint passes: `pnpm run lint`
- [ ] Format applied: `pnpm run format`
- [ ] No console.log in production code
- [ ] No TODO comments (or documented in issue)
- [ ] File size reasonable (< 500 lines)

---

## Before Submitting

### Documentation

- [ ] Updated README.md if user-facing change
- [ ] Updated API docs if API changed
- [ ] Added examples if helpful
- [ ] Updated CHANGELOG.md (unreleased section)
- [ ] Updated AGENT-JOURNAL.md

### Final Checks

- [ ] Ran quality gate: `node scripts/quality-gate.js`
- [ ] All quality checks pass
- [ ] Git working tree clean except intended changes
- [ ] Commit messages are clear and descriptive

---

## Creating Pull Request

### PR Description

- [ ] Used [PR template](../.github/PULL_REQUEST_TEMPLATE.md)
- [ ] Described what problem is solved
- [ ] Explained how changes work
- [ ] Included testing evidence (screenshots, logs)
- [ ] Linked related issues: `Fixes #123`

### PR Checklist

- [ ] Marked correct type: bug fix, feature, refactor, etc.
- [ ] Checked all applicable checklist items
- [ ] Added reviewers if appropriate
- [ ] Set labels (bug, enhancement, etc.)

---

## After Submission

### Review Process

- [ ] Responded to reviewer comments promptly
- [ ] Made requested changes (if reasonable)
- [ ] Re-ran tests after changes
- [ ] Kept PR updated with main branch: `git rebase main`

### After Merge

- [ ] Deleted feature branch
- [ ] Tested the merged changes locally
- [ ] Reported any post-merge issues

---

## Quick Reference

### Common Commands

```bash
# Run tests
pnpm run test:bun:unit

# Run lint
pnpm run lint

# Format code
pnpm run format

# Quality gate
node scripts/quality-gate.js

# Commit
pnpm commit "Add new feature"

# Update branch
git rebase main
```

### Branch Naming

- `feat/feature-name` - New features
- `fix/bug-description` - Bug fixes
- `refactor/module-name` - Code improvements
- `docs/topic` - Documentation
- `test/feature-name` - Test additions

### Commit Message Format

```
type(scope): subject

body (optional)

footer (optional)
```

**Types**: feat, fix, docs, style, refactor, test, chore

**Examples**:
```
feat(agent): add retry logic for LLM requests
fix(browser): resolve connection timeout on Windows
docs(api): add examples to click method
```

---

## Getting Help

- [Documentation](../docs/)
- [Existing Issues](https://github.com/kardelitaitu/auto-ai/issues)
- [AGENTS.md](../AGENTS.md)
- [API Cheatsheet](../API-CHEATSHEET.md)

---

*Last updated: 2026-03-31*
