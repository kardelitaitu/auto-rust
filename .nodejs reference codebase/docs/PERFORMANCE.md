# Performance Guidelines

Guidelines for maintaining and improving Auto-AI performance.

---

## Performance Targets

| Metric | Target | Critical Threshold |
|--------|--------|-------------------|
| Page load time | < 3s | > 10s |
| Action execution | < 500ms | > 2s |
| LLM response (local) | < 5s | > 15s |
| LLM response (cloud) | < 10s | > 30s |
| Session startup | < 2s | > 5s |
| Memory per session | < 100MB | > 500MB |

---

## Benchmarking

### Running Benchmarks

```bash
# Run all benchmarks
pnpm vitest run api/tests/benchmarks

# Compare with previous run
node scripts/benchmark-compare.js

# Save new baseline
node scripts/benchmark-compare.js --save
```

### Benchmark Categories

1. **Query Performance** - DOM query speed
2. **Action Performance** - Click, type, scroll speed
3. **Timing Performance** - Delay accuracy
4. **Memory Performance** - Session memory usage

---

## Optimization Techniques

### 1. Reduce DOM Queries

```javascript
// ❌ Slow: Multiple queries
const el = await page.$('.container');
const btn = await el.$('button');
const text = await btn.textContent();

// ✅ Fast: Single query with better selector
const text = await page.textContent('.container button');
```

### 2. Batch Operations

```javascript
// ❌ Slow: Sequential actions
for (const item of items) {
    await api.click(item);
}

// ✅ Fast: Parallel where possible
await Promise.all(items.map(item => api.click(item)));
```

### 3. Efficient Waiting

```javascript
// ❌ Slow: Fixed long wait
await api.wait(5000);

// ✅ Fast: Wait for condition
await api.wait.forElement('.loaded');
```

### 4. Memory Management

```javascript
// ❌ Memory leak: Unclosed pages
async function process() {
    const page = await browser.newPage();
    // ... process
    // page never closed!
}

// ✅ Proper cleanup
async function process() {
    const page = await browser.newPage();
    try {
        // ... process
    } finally {
        await page.close();
    }
}
```

---

## Profiling

### CPU Profiling

```bash
# Generate CPU profile
node --prof main.js pageview=example.com

# Analyze profile
node --prof-process isolate-*.log > profile.txt
```

### Memory Profiling

```bash
# Take heap snapshot
node --inspect main.js

# Connect Chrome DevTools to inspect://localhost:9229
# Take heap snapshot in Memory tab
```

### Performance Timeline

```javascript
// In your automation script
const start = performance.now();
await api.goto('https://example.com');
const loadTime = performance.now() - start;
console.log(`Load time: ${loadTime.toFixed(2)}ms`);
```

---

## Common Performance Issues

### Issue: Slow Element Finding

**Symptom**: `api.click()` takes > 2s

**Causes**:
- Complex CSS selectors
- Large DOM
- Element not yet rendered

**Fixes**:
```javascript
// Use specific selectors
await api.click('#submit-btn');  // ✅ ID selector (fastest)
await api.click('.btn.primary');  // ✅ Class + qualifier
await api.click('div > span > a'); // ❌ Deep hierarchy (slow)

// Wait for element first
await api.wait.forElement('#submit-btn');
await api.click('#submit-btn');
```

### Issue: Memory Growth

**Symptom**: Memory usage grows continuously

**Causes**:
- Unclosed pages
- Event listener leaks
- Cached data not cleared

**Fixes**:
```javascript
// Always close pages
await page.close();

// Remove listeners
page.removeAllListeners();

// Clear caches periodically
if (sessionCount % 100 === 0) {
    global.gc(); // Force GC (requires --expose-gc)
}
```

### Issue: LLM Timeout

**Symptom**: LLM requests timeout frequently

**Causes**:
- Model too large
- Complex prompts
- Network issues

**Fixes**:
```json
// config/settings.json
{
  "llm": {
    "local": {
      "model": "hermes3:8b",  // Use smaller model
      "timeout": 30000        // Increase timeout
    },
    "fallback": {
      "enabled": true,        // Enable cloud fallback
      "maxRetries": 2
    }
  }
}
```

---

## CI Performance Monitoring

The release workflow includes performance checks:

```yaml
# .github/workflows/release.yml
- name: Run benchmarks
  run: node scripts/benchmark-compare.js --save

- name: Check for regressions
  run: node scripts/benchmark-compare.js
```

---

## Performance Budget

| Resource | Budget | Alert |
|----------|--------|-------|
| Bundle size | < 5MB | > 10MB |
| Startup time | < 2s | > 5s |
| Memory/session | < 100MB | > 200MB |
| CPU usage | < 50% | > 80% |

---

## Monitoring in Production

### Key Metrics to Track

1. **Session Duration** - Average session lifetime
2. **Action Success Rate** - % of successful actions
3. **Error Rate** - Errors per 100 sessions
4. **LLM Latency** - Average LLM response time
5. **Browser Connect Rate** - % successful connections

### Logging Performance

```javascript
// Add performance logging
const start = Date.now();
await api.click('.btn');
const duration = Date.now() - start;

if (duration > 1000) {
    logger.warn(`Slow click: ${duration}ms`);
}
```

---

## Related Documentation

- [`scripts/benchmark-compare.js`](../scripts/benchmark-compare.js) - Benchmark comparison tool
- [`api/tests/benchmarks/`](../api/tests/benchmarks/) - Benchmark tests
- [docs/architecture.md](architecture.md) - System architecture
