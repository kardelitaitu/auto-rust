# Plan#

## What Is the Solution#

Automate documentation generation and create architecture overview.

### Step 1: Add CI Documentation Step
Add `cargo doc --all-features` to `check.ps1` or separate `docs.ps1`.

### Step 2: Create Architecture Document
Create `docs/ARCHITECTURE.md` with:
- Module hierarchy diagram
- Data flow overview
- Key design decisions
- Extension points

### Step 3: Generate API Reference
Use `cargo doc --all-features` output + custom CSS for better readability.

## Internal API Outline#

Implementation details to be defined during active development.

## Decisions#

1. **Doc-only CI step**: Don't block builds on doc warnings.
2. **Keep it simple**: Start with basic architecture doc.
3. **Automate**: Generate from code as much as possible.

## Expected Outcome#

After implementation:
- `docs/ARCHITECTURE.md` created
- CI generates docs automatically
- API reference available via `target/doc/`
