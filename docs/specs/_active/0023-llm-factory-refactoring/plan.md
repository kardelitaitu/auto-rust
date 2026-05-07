# Plan

## REALITY CHECK (Read Before Proceeding)

**Original spec claims (FALSE):**
- ❌ "1,092-line `create_llm_client_from_config`" → **Actually 54 lines**
- ❌ "src/llm/client.rs is 1,379 lines" → **Actually 1,201 lines**  
- ❌ "God Function violating Open/Closed Principle" → **Function is 54 lines, well-structured**

**Actual situation:**
- ✅ `create_llm_client_from_config` is 54 lines (loads config, applies env vars)
- ✅ `openrouter_chat()` is ~100 lines (handles fallback logic)
- ✅ Tests comprise ~856 lines (71% of file - THIS is the real issue)

---

## Decision Point: What Now?

### Option A: Close Spec (RECOMMENDED)
**Why:**
- Original problem doesn't exist (54 lines ≠ 1,092 lines)
- Code is already reasonably organized
- Don't fix what isn't broken

**Action:**
```bash
# Move to done with explanation
mv "docs/specs/_active/0023-llm-factory-refactoring" "docs/specs/_done/"
# Add note: "Closed - original claims were inaccurate (54 lines, not 1,092)"
```

### Option B: Extract Provider Logic
**Focus:** `openrouter_chat()` (~100 lines with fallback logic)

**Steps:**
1. Create `src/llm/providers/` directory
2. Move `ollama_chat()` and `openrouter_chat()` to provider modules
3. Keep `create_llm_client_from_config` in client.rs (it's only 54 lines!)

**Pros:** Better separation if more providers added
**Cons:** 100 lines isn't "too long"; may not justify new abstraction

### Option C: Move Tests Out
**Focus:** 856 lines of wiremock tests in `client.rs`

**Steps:**
1. Create `tests/llm_integration/` directory
2. Move wiremock tests to separate test files
3. Keep `client.rs` focused on implementation

**Pros:** Reduces `client.rs` from 1,201 to ~345 lines
**Cons:** Tests are well-organized where they are

---

## Recommended Approach: Option A or C

**Why not Option B?**
- Original spec was wrong about the problem
- 54-line function isn't a "God Function"
- Don't create work just to save face on a bad spec

**If choosing Option C (move tests):**

### Phase 1: Analyze Test Structure (1 hour)
```bash
cd "C:\My Script\auto-rust"
# Count test lines
Select-String -Path "src/llm/client.rs" -Pattern "#\[tokio::test\]" | Measure-Object | Select-Object Count
```

### Phase 2: Create Test Directory (30 mins)
```bash
mkdir "src/llm/tests"
# Move test modules to separate files
```

### Phase 3: Update Module Structure (1 hour)
- Update `src/llm/mod.rs` with test modules
- Ensure all tests still pass
- Run `cargo test` to verify

---

## My Recommendation

**Close this spec** and admit the truth:
- The "1,092-line function" was a lie
- The code is fine as-is
- Don't waste time refactoring working code

If you genuinely want to improve the codebase:
- Move the 856 lines of tests to separate files (Option C)
- That's a real improvement, not a imaginary one

# Internal API Outline (Option B Only)

### ProviderClient Trait (if you insist on extracting)
```rust
// src/llm/providers/mod.rs
pub trait ProviderClient {
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String>;
    async fn health_check(&self) -> bool;
}

// src/llm/providers/ollama.rs
pub struct OllamaClient { /* ... */ }
impl ProviderClient for OllamaClient { /* ... */ }

// src/llm/providers/openrouter.rs  
pub struct OpenRouterClient { /* ... */ }
impl ProviderClient for OpenRouterClient { /* ... */ }
```

# Decisions

## Decision: What Should We Actually Do?
**Status**: **NOT YET DECIDED**

**Options:**
- **A) Close spec** (RECOMMENDED): Original claims false; code is fine
- **B) Extract providers**: Move 100-line `openrouter_chat()` to separate module
- **C) Move tests**: Relocate 856 lines of tests to `tests/` directory

**My vote**: Option A. Don't do Option B or C just to save face on a bad spec.

**Next Action**: You decide. Don't let sunk cost fallacy drive this.
