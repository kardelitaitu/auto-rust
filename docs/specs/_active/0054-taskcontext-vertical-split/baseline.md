# Baseline

## What I Find

### task_context.rs is 5,607 lines (7.5% of all source)

The file lives at `src/runtime/task_context.rs` and already has 5 submodule files under `src/runtime/task_context/` (`click_learning.rs`, `interaction.rs`, `interaction_pipeline.rs`, `query.rs`, `types.rs`). Despite these extractions, the main file remains enormous.

### Method count by domain (89 public methods total)

| Domain | Methods | ~Lines | Extraction |
|--------|---------|--------|------------|
| Struct + constructors | 2 | 80 | Keep in main |
| Field accessors | 7 | 100 | Keep in main |
| Navigation | 3 | 150 | → `navigation.rs` |
| Permission/connected | 2 | 80 | → `navigation.rs` |
| Screenshot | 2 | 100 | → `navigation.rs` |
| Cookies | 5 | 600 | → `cookies.rs` |
| Clipboard | 5 | 200 | → `clipboard.rs` |
| Data files | 9 | 400 | → `data_files.rs` |
| HTTP/Download | 3 | 200 | → `http.rs` |
| Session/Browser export | 6 | 550 | → `session_io.rs` |
| Style/Rect/Viewport | 5 | 200 | → `style.rs` |
| Click (incl. variants) | 10 | 900 | Keep in main |
| Hover/Mouse | 4 | 150 | Keep in main |
| Focus/Keyboard/Type | 7 | 300 | Keep in main |
| Wait | 3 | 150 | Keep in main |
| Scroll | 8 | 250 | Keep in main |
| Pause | 3 | 100 | Keep in main |
| Element queries | 7 | 100 | Keep in main |
| Browser context | 3 | 120 | Keep in main |
| Copy/cut/paste | 3 | 50 | Keep in main |
| Tests | ~30 | ~500 | Keep in main |
| Other | 3 | ~150 | Keep in main |

### Extracting 6 domains removes ~2,100 lines

The 6 proposed submodules remove ~37% of the file:

| New file | Lines removed |
|----------|--------------|
| `cookies.rs` | ~600 |
| `session_io.rs` | ~550 |
| `data_files.rs` | ~400 |
| `navigation.rs` | ~330 |
| `http.rs` | ~200 |
| `clipboard.rs` | ~200 |
| `style.rs` | ~200 |
| **Total** | **~2,100** |

### Each method accesses self.page / self.policy / self.behavior_runtime

Since submodule files are children of `task_context.rs`'s module, they can access private fields without any visibility changes. The existing submodules (`click_learning.rs` etc.) already use this pattern.

## What I Claim

Extracting 7 domain-focused submodule files from `task_context.rs` reduces it from ~5,600 to ~3,500 lines — a 37% reduction. All extractions are pure moves with no behavioral changes. The existing module pattern (private field access from submodules) already proven by `click_learning.rs`, `interaction.rs`, etc.

## What Is the Proof

- Line counts from `Select-String` mapping of all public methods
- Private field access pattern verified: `click_learning.rs` accesses `self.page`, `self.metrics` directly
- Each domain group is self-contained (methods only reference `self`, no cross-coupling between domains)
