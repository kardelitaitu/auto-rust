**AI Assistant Operating Manual for rust-orchestrator**
*Last Updated: April 28, 2026*

---

**PRIORITY ORDER: MCP tools FIRST, shell commands ONLY as fallback.**

### When to use which MCP tool:

#### filesystem MCP (Local file operations)
**USE FOR:** Reading, writing, searching, listing files in this repo
- `read_text_file` - Read file contents
- `read_multiple_files` - Read several files at once
- `list_directory` - See directory contents
- `search_files` - Find files by glob pattern (e.g., `**/*.rs`)
- `get_file_info` - File metadata (size, dates, permissions)
- `write_file` - Create/overwrite files
- `edit_file` - Make targeted edits (use `dryRun: true` to preview)
- `create_directory` - Create directories
- `move_file` - Move/rename files

**RULES:**
- ALWAYS use absolute paths: `C:\My Script\auto-rust\...`
- For discovery tasks, use `search_files` or `list_directory` FIRST
- For reading code, use `read_text_file` NOT shell `cat`/`type`
- For finding files, use `search_files` NOT shell `find`/`dir`

#### context-mode MCP (Command execution + large output handling)
**USE FOR:** Running commands that produce lots of output, indexing documentation

- `ctx_execute` - Run commands, auto-index output, search with queries
  - **PREFER this over shell** for: `git log`, `cargo build`, test runs, `npm test`, API calls
  - Use `intent` parameter to describe what you're looking for
  - Output gets indexed - use `ctx_search` to retrieve specific sections
- `ctx_execute_file` - Process a file without loading it into context
  - Use for: Large logs, data files (CSV/JSON), big source files
- `ctx_index` - Index documentation/knowledge into searchable database
  - Use for: API docs, README files, migration guides, code examples
- `ctx_search` - Search indexed content with multiple queries
  - Batch ALL questions in one call
  - Use 2-4 specific technical terms per query
- `ctx_batch_execute` - Execute multiple commands, index all output, search once
  - **PRIMARY TOOL** for complex multi-step tasks
  - Replaces 30+ execute calls + 10+ search calls
  - Provide all commands and all search queries in ONE call

**RULES:**
- Force repo cwd: `cd "C:\My Script\auto-rust" && ...`
- Use `intent` parameter to guide output filtering
- After indexing, use `search` to retrieve specific sections

#### tavily MCP (Web search and content extraction)
**USE FOR:** Web searches, extracting content from URLs, research tasks

- `tavily_search` - Search web for current information
  - Use for: News, facts, latest versions, error solutions
  - Returns snippets + source URLs
- `tavily_extract` - Extract content from specific URLs
  - Use when user provides URLs to check
  - Returns markdown or text
- `tavily_research` - Comprehensive research on a topic
  - Use for: Complex topics needing multiple sources
  - Returns detailed research report
- `tavily_skill` - Search library/API documentation
  - Use when working with specific libraries
  - Returns structured documentation chunks
- `tavily_map` - May fail with URL validation in this environment
- `tavily_crawl` - May fail with URL validation in this environment

**RULES:**
- Prefer `tavily_search` over `tavily_map/crawl` for reliability
- Use `tavily_extract` if `ctx_fetch_and_index` fails with TLS error
- Always specify `max_results` when you need specific number of results

#### memory MCP (Persistent knowledge)
**USE FOR:** Saving important facts that should persist across conversations

- `create_entities` - Save new concepts with observations
- `add_observations` - Add notes to existing entities
- `read_graph` - Read entire knowledge graph
- `search_nodes` - Search for entities by query
- `open_nodes` - Retrieve specific entities by name

**RULES:**
- ONLY use when information should persist beyond current session
- DO NOT use for temporary context or current session facts
- Keep observations concise and factual
- Use for: User preferences, project architecture decisions, common patterns

#### sequential-thinking MCP (Complex reasoning)
**USE FOR:** Breaking down complex problems into structured steps

- `sequentialthinking` - Multi-step analytical thinking
  - Use for: Architecture decisions, debugging complex issues, multi-step planning
  - Can revise previous thoughts, branch into new paths
  - Express uncertainty when present

**RULES:**
- Use for problems requiring 3+ analytical steps
- Can adjust `totalThoughts` up/down as understanding evolves
- Mark `isRevision: true` when reconsidering previous thoughts
- Set `nextThoughtNeeded: false` ONLY when truly done

### MCP Tool Selection Decision Tree:

```
Need to read/find files in repo?
  -> filesystem MCP (read_text_file, search_files, list_directory)

Need to run commands with output?
  -> context-mode MCP (ctx_execute, ctx_batch_execute)

Need web information/latest versions?
  -> tavily MCP (tavily_search, tavily_research, tavily_skill)

Need to save important facts?
  -> memory MCP (create_entities, add_observations)

Need to analyze complex problem?
  -> sequential-thinking MCP

Need external app integration (GitHub, Slack)?
  -> composio MCP (start with COMPOSIO_SEARCH_TOOLS)
```

### Required operating rules:
1. Always use absolute paths with filesystem MCP tools
2. For context-mode, force repo cwd: `cd "C:\My Script\auto-rust" && ...`
3. For composio: Start with `COMPOSIO_SEARCH_TOOLS`, reuse `session_id` in subsequent calls
4. For composio tools with `schemaRef`: Fetch schema via `COMPOSIO_GET_TOOL_SCHEMAS` first
5. Never invent tool arguments - stay schema-compliant
6. **ALWAYS try MCP tools before falling back to shell commands**
7. **Git commit messages must be descriptive** - use format `type: what changed (reason/impact)`. Never use generic messages like "update", "fix", "changes". Examples:
   - `docs: rewrite README with TOC (843 -> 350 lines)`
   - `feat: add twitterquote task with LLM integration`
   - `fix: handle rate limit in twitterfollow retry logic`
8. **Never push to remote without running verification commands** - always execute these before `git push`:
   - `cargo t` (runs `cargo test --all-features` via alias)
   - `cargo f` (runs `cargo fmt --all -- --check` via alias)
   - `cargo clippy`
   - Ensure all pass before pushing to remote repository

### Codebase rules
- `TaskContext` is the task-api entry point; task code should stay thin and compose shared capabilities.
- Use short task verbs in examples: `api.click(...)`, `api.pause(...)`, `api.focus(...)`, and `api.keyboard(...)` (with `api.r#type(...)` as the Rust-safe alias).
- `api.pause(base_ms)` uses a **uniform** ±20% random delay; `api.pause_with_variance(base_ms, pct)` uses the same **uniform** model with a custom spread; `api.pause_human(base_ms, pct)` uses a **Gaussian** delay for human-like variance. Do not rely on exact wall-clock pause duration when a cancel token is wired: pauses may end early on cooperative shutdown.
- `TaskContext::new` and `new_with_metrics` take a final `Option<CancellationToken>`; the orchestrator passes `Some(token)` so the pause family can wake early on group cancel. Tests and ad-hoc construction typically pass `None`.
- High-level task-api verbs already add a post-action settle pause, so tasks should not duplicate `pause` unless they need a special case.
- `api.click(selector)` is the default interaction path; it runs the selector pipeline with scroll + move + click. Use coordinate clicks only when a task explicitly needs them.
- Task groups are intentionally broadcast to every active browser session; parallel fan-out is the default execution model.
- Validation and task execution should share one payload resolver for alias handling and normalization.
- Keep task-specific parsing out of orchestrator code when a shared validation helper can own it.
- Run summaries should include active, healthy, and unhealthy session counts plus per-task/per-session breakdowns.
- If healthy sessions drop below the operational threshold, emit a warning so batch degradation is visible in logs.
- Prefer task-api verbs that stay on the API surface: `api.click(...)`, `api.click_and_wait(...)`, `api.hover(...)`, `api.double_click(...)`, `api.middle_click(...)`, `api.right_click(...)`, `api.drag(...)`, `api.focus(...)`, `api.keyboard(...)`, `api.pause(...)`, `api.pause_with_variance(...)`, `api.pause_human(...)`, `api.randomcursor()`, `api.clear(...)`, `api.select_all(...)`, `api.exists(...)`, `api.visible(...)`, `api.text(...)`, `api.html(...)`, `api.attr(...)`, `api.wait_for(...)`, `api.wait_for_visible(...)`, `api.scroll_to(...)`, `api.url()`, and `api.title()`.
- Keep shared UTF-8-safe text helpers in the internal/text utility layer instead of duplicating truncation logic in tasks.
- Keep X/Twitter selectors scoped to the target container or captured node; avoid page-wide button scans when a task can bind a single element.
- Prefer deterministic verification of the same target element that was clicked or inspected.
- Use `cookiebot` only for its own resource-blocking behavior; do not leak that policy into unrelated tasks.
- Keep task names canonical and consistent across `task/mod.rs`, `src/cli.rs`, validation, and README.
- Current supported browsers are Brave and Roxybrowser; other Chromium browsers are future connectors only.

### Twitter Utility Modules
The Twitter automation utilities are located in `src/utils/twitter/` and have comprehensive rustdoc documentation:

**Core Modules:**
- `twitteractivity_dive.rs`: Thread diving, reading, and incremental caching for LLM context
- `twitteractivity_interact.rs`: Engagement actions (like, retweet, follow, reply, bookmark)
- `twitteractivity_llm.rs`: LLM-powered reply/quote generation with context extraction and validation
- `twitteractivity_feed.rs`: Feed scrolling, candidate identification, and progress tracking
- `twitteractivity_navigation.rs`: Page navigation and login state checks

**Supporting Modules:**
- `twitteractivity_sentiment*.rs`: Sentiment analysis (emoji, context, domains, LLM)
- `twitteractivity_humanized.rs`: Human-like timing and cursor movements
- `twitteractivity_selectors.rs`: Twitter-specific DOM selectors
- `twitteractivity_decision.rs`: Engagement decision logic
- `twitteractivity_limits.rs`: Engagement limit tracking
- `twitteractivity_persona.rs`: Persona-based behavior profiles
- `twitteractivity_popup.rs`: Popup/modal handling

**Documentation:**
All functions include detailed rustdoc with Arguments, Returns, Errors, Behavior, and Selectors sections. Generate with `cargo doc --all-features`.

### Known environment caveats (verified):
- `filesystem` MCP - Full functionality working
- `context-mode` `ctx_execute` - Working correctly
- `context-mode` `ctx_fetch_and_index` - May fail with TLS error (`UNABLE_TO_GET_ISSUER_CERT_LOCALLY`)
  - **Fallback:** Use `tavily_extract` for URL content
- `memory` MCP - Entity creation/retrieval working
- `sequential-thinking` MCP - Working correctly
- `tavily_search`, `tavily_extract`, `tavily_research`, `tavily_skill` - All working
- `tavily_map`, `tavily_crawl` - May return invalid start URL errors
  - **Fallback:** Use `tavily_search` + `tavily_extract` combination

## Code Improvement Workflow

When improving the codebase, follow this systematic procedure to ensure quality and prevent regressions:

### 0. Baseline Check
- **Run cargo check** - verify clean compilation state before starting
- **Check git status** - ensure no uncommitted changes that could interfere
- **Record baseline** - note current test results if relevant
- **This provides a clean rollback point** if fixes introduce unexpected issues

### 1. Identify Problems
- Review code for bugs, performance issues, code smells, or violations of best practices
- Use grep/search to find patterns across the codebase
- Check for unused variables, dead code, duplicate logic, inconsistent error handling
- Review large functions, complex control flow, and unclear abstractions

### 2. Filter Problems
- Separate actual bugs from style preferences
- Prioritize by impact: functional bugs > performance > maintainability > style
- Discard findings that are intentional design choices (documented or justified)
- Focus on issues that have clear, measurable impact

### 3. Verify Problems Are Real
- **Deep trace the data flow** - follow variables through the code to understand actual behavior
- **Check usage patterns** - verify how functions are called and return values are used
- **Review documentation** - check if comments explain why code is written a certain way
- **Consider context** - understand the broader architecture before making changes
- **Be 100% certain** - only proceed with fixes when you have complete understanding

### 4. Prove Fixes Won't Break Codebase
- **Run cargo check** - verify compilation before any changes
- **Run cargo test** - establish baseline test results
- **Consider test coverage** - if tests don't cover the code, plan to add tests
- **Think about edge cases** - what could go wrong with the fix?
- **Review dependencies** - will the change affect other modules?

### 5. Wait for Confirmation
- **Present findings clearly** - explain the problem, the fix, and the rationale
- **Show evidence** - include code snippets, test results, and analysis
- **Ask for approval** - do not make changes until user confirms
- **Be prepared to adjust** - user may have different priorities or constraints

**Follow this order: Present → Approve → Execute**
- First present the findings and proposed fix
- Wait for explicit user approval
- Only then execute the changes

### 6. Fix Files (After Confirmation)
- **Make minimal changes** - fix only what's necessary
- **Follow existing style** - match the surrounding code conventions
- **Add comments if needed** - explain non-obvious changes
- **Use edit tools** - prefer targeted edits over wholesale rewrites

### 7. Run Verification
- **cargo check** - ensure compilation succeeds
- **cargo test** - ensure all tests pass
- **If tests fail** - either fix the implementation or patch the test (if test was wrong)
- **If no tests exist** - consider adding tests for the fixed code

### 8. Handle Test Issues
- **If implementation is wrong** - fix the implementation and re-test
- **If test is wrong** - update the test to match correct behavior
- **If no test coverage** - create a test that validates the fix
- **Document the test** - explain what it tests and why

### 9. Report, Commit, and Push
- **Summarize changes** - explain what was fixed and why
- **Use clear commit messages** - describe the change, not just "fix bug"
- **Include context** - mention the issue addressed and the impact
- **git commit** - commit with proper message
- **git push** - push to remote repository

### 9.5. Rollback Checkpoint
- **Create a git tag** or note the commit hash after successful push
- **This provides a safety net** if the fix introduces issues in production
- **Quick rollback reference** - easy to revert if problems emerge
- **Tag format example**: `git tag rollback-safe-YYYYMMDD`

### 10. Update Journal

Add entry to `JOURNAL.md` summarizing the session's accomplishments:

```markdown
## YYYY-MM-DD - Brief Description

### Accomplished This Session

#### Area of Work
- **file.rs**: What changed and why
- **Module**: Key additions or fixes

### Current Status

| Item | Status |
|------|--------|
| Build | ✅ Pass / ❌ Fail |
| Tests | ✅ X passed / ❌ Y failed |
| cargo clippy | ✅ Clean / ⚠️ Warnings |
```

**Keep it concise** - focus on what changed, not how. Cross-reference commit messages for details.

### Key Principles
- **Be conservative** - when in doubt, don't change it
- **Be thorough** - verify your understanding before proposing changes
- **Be transparent** - show your work and reasoning
- **Be patient** - wait for confirmation before acting
- **Be minimal** - change only what needs to be changed
