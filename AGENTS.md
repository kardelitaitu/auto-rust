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

## Development Tasklist (Rust vs `.nodejs-reference`)

### Phase 0 - Foundations
- [x] Create `src/result.rs` for unified task result contract: `success | failed | timeout`
- [x] Refactor `task::perform_task` and orchestrator execution path to return structured result
- [x] Add consistent error typing (`timeout`, `validation`, `navigation`, `session`)

### Phase 1 - Orchestrator Reliability
- [x] Add per-task timeout (`task_timeout_ms`) with cancellation propagation
- [x] Add group timeout hard-stop (`group_timeout_ms`) for batch execution
- [x] Add retry policy with attempt metadata (`attempt`, `max_retries`, `last_error`)
- [x] Add worker/page health checks and stale-task cleanup

### Phase 2 - Session Lifecycle
- [x] Implement full `connect_to_browser` for configured profiles (currently TODO)
- [x] Add session state tracking (`idle`, `busy`, `failed`) and failure score
- [x] Add managed page registry and guaranteed release on all code paths
- [x] Add graceful shutdown flow (cancel active tasks -> close pages -> close browsers)

### Phase 3 - Config + Validation
- [x] Add file-backed config loader (`config/*.toml`) with env override precedence
- [x] Add task payload schema validator mirroring `task-validator.js`
- [ ] Add task parser parity checks with `.nodejs-reference/api/utils/task-parser.js`
- [x] Add startup config validation and fail-fast diagnostics

### Phase 4 - API Utility Layer
- [ ] Create `src/api/client.rs` for HTTP requests with shared headers
- [ ] Add retry with jitter/backoff (`retries`, `factor`, `max_delay`)
- [ ] Add optional circuit-breaker module (feature-flagged)
- [ ] Add provider fallback strategy hooks

### Phase 5 - Observability
- [ ] Create metrics collector: task counts, durations, session stats, API stats
- [ ] Add task history ring buffer and per-task breakdown
- [ ] Export `run-summary.json` at shutdown
- [ ] Add periodic health/memory logs with threshold warnings

### Phase 6 - Utility Hardening
- [ ] Replace JS-simulated mouse/keyboard with CDP/native input when available
- [ ] Keep JS fallback path for unsupported browser contexts
- [ ] Add deterministic utility tests for navigation/scroll/timing behavior
- [ ] Add integration tests for `cookiebot` and `pageview`

### Delivery Tracks
- [ ] Fast track: complete Phases 0-2 first (production stability)
- [ ] Full parity: complete Phases 0-6 (feature parity with Node reference)
