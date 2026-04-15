# Optimization Techniques

Performance optimization guide for Auto-AI automation.

---

## Table of Contents

1. [Caching](#caching)
2. [Connection Pooling](#connection-pooling)
3. [Query Optimization](#query-optimization)
4. [Parallel Execution](#parallel-execution)
5. [Memory Management](#memory-management)
6. [Timing Optimization](#timing-optimization)
7. [Best Practices](#best-practices)

---

## Caching

### LRU Cache for Queries

Use the built-in LRU cache to avoid redundant DOM queries:

```javascript
import { queryCache, createCachedFunction } from './api/utils/lru-cache.js';

// Manual caching
async function getCachedElement(selector) {
    const cached = queryCache.get(selector);
    if (cached) return cached;
    
    const element = await page.$(selector);
    queryCache.set(selector, element);
    return element;
}

// Automatic caching with wrapper
const cachedQuery = createCachedFunction(
    async (selector) => await page.$(selector),
    { maxSize: 100, ttl: 10000 }  // 10 second TTL
);

// First call executes
const el1 = await cachedQuery('.button');

// Second call returns cached result
const el2 = await cachedQuery('.button');
```

### Content Caching

```javascript
import { contentCache } from './api/utils/lru-cache.js';

async function getCachedContent(url) {
    const cached = contentCache.get(url);
    if (cached) return cached;
    
    await api.goto(url);
    const content = await api.getText('body');
    
    contentCache.set(url, content);
    return content;
}
```

### Cache Invalidation

```javascript
// Clear cache after navigation
await api.goto('https://example.com');
queryCache.clear();  // Clear stale selectors

// Selective invalidation
queryCache.delete('.dynamic-content');
```

---

## Connection Pooling

### Using Connection Pool

```javascript
import { getConnection, releaseConnection } from './api/core/connection-pool.js';

// Get connection from pool
const conn = await getConnection();

try {
    const { browser } = conn;
    const page = await browser.newPage();
    
    await api.withPage(page, async () => {
        await api.init(page);
        await api.goto('https://example.com');
        // ... automation
    });
} finally {
    // Always release back to pool
    await releaseConnection(conn);
}
```

### Custom Pool Configuration

```javascript
import { createPool } from './api/core/connection-pool.js';

const pool = createPool({
    minSize: 2,      // Keep 2 connections ready
    maxSize: 10,     // Max 10 connections
    acquireTimeout: 30000,  // 30s timeout
    idleTimeout: 300000     // 5min idle expiry
});

// Use custom pool
const conn = await pool.getConnection();
// ... use connection
await pool.releaseConnection(conn);
```

---

## Query Optimization

### Efficient Selectors

```javascript
// ✅ Fast - ID selector
await api.click('#submit-btn');

// ✅ Fast - Class selector
await api.click('.btn-primary');

// ✅ Good - Combined
await api.click('form#login button.submit');

// ❌ Slow - Deep hierarchy
await api.click('div > form > div > button');

// ❌ Slow - Universal selector
await api.click('*[data-action="submit"]');
```

### Batch Queries

```javascript
// ❌ Slow - Sequential queries
const items = [];
for (let i = 0; i < 10; i++) {
    items.push(await api.find(`.item-${i}`));
}

// ✅ Fast - Parallel queries
const items = await Promise.all(
    Array.from({ length: 10 }, (_, i) => 
        api.find(`.item-${i}`)
    )
);
```

### Wait Strategically

```javascript
// ❌ Slow - Fixed long wait
await api.wait(5000);

// ✅ Fast - Wait for specific condition
await api.wait.forElement('.loaded');

// ✅ Fast - Wait for network
await api.wait.forNetworkIdle();
```

---

## Parallel Execution

### Multiple Sessions

```javascript
// Process multiple pages in parallel
const pages = await Promise.all(
    Array.from({ length: 5 }, () => browser.newPage())
);

await Promise.all(
    pages.map(page => 
        api.withPage(page, async () => {
            await api.init(page);
            await api.goto('https://example.com');
            // ... process
        })
    )
);
```

### Batch Processing

```javascript
// Process items in batches
async function processInBatches(items, batchSize = 10) {
    const results = [];
    
    for (let i = 0; i < items.length; i += batchSize) {
        const batch = items.slice(i, i + batchSize);
        const batchResults = await Promise.all(
            batch.map(item => processItem(item))
        );
        results.push(...batchResults);
    }
    
    return results;
}
```

---

## Memory Management

### Close Pages Promptly

```javascript
// ✅ Good - Proper cleanup
async function processPage() {
    const page = await browser.newPage();
    try {
        await api.withPage(page, async () => {
            // ... process
        });
    } finally {
        await page.close();  // Always close
    }
}

// ❌ Bad - Memory leak
async function processPage() {
    const page = await browser.newPage();
    await api.withPage(page, async () => {
        // ... process
    });
    // page never closed!
}
```

### Clear Caches Periodically

```javascript
// Clear caches every 100 operations
let operationCount = 0;

async function operation() {
    operationCount++;
    
    if (operationCount % 100 === 0) {
        queryCache.clear();
        contentCache.clear();
        
        // Force garbage collection if available
        if (global.gc) global.gc();
    }
    
    // ... operation
}
```

### Limit Concurrent Sessions

```javascript
// Use semaphore for concurrency control
import { Semaphore } from './api/utils/semaphore.js';

const sessionSemaphore = new Semaphore(5);  // Max 5 concurrent

async function createSession() {
    await sessionSemaphore.acquire();
    
    try {
        const page = await browser.newPage();
        // ... use page
        return page;
    } finally {
        sessionSemaphore.release();
    }
}
```

---

## Timing Optimization

### Reduce Humanization Delays

```javascript
// For development/testing (faster)
await api.init(page, {
    persona: 'focused',
    humanizationPatch: false  // Disable for speed
});

// For production (stealthy)
await api.init(page, {
    persona: 'casual',
    humanizationPatch: true
});
```

### Adjust Scroll Speed

```javascript
// Faster scrolling (less stealthy)
{
    "humanization": {
        "scroll": {
            "globalScrollMultiplier": 0.5  // 50% faster
        }
    }
}

// Slower scrolling (more stealthy)
{
    "humanization": {
        "scroll": {
            "globalScrollMultiplier": 2.0  // 2x slower
        }
    }
}
```

### Optimize LLM Usage

```javascript
// Use local LLM for simple tasks
{
    "llm": {
        "local": {
            "enabled": true,
            "model": "hermes3:8b"  // Fast, small model
        },
        "cloud": {
            "enabled": false  // Only for complex tasks
        }
    }
}
```

---

## Best Practices

### 1. Profile Before Optimizing

```javascript
// Measure execution time
const start = Date.now();
await automationTask();
const duration = Date.now() - start;
console.log(`Task took ${duration}ms`);
```

### 2. Use Benchmarks

```bash
# Run benchmarks
pnpm vitest run api/tests/benchmarks

# Compare with baseline
node scripts/benchmark-compare.js
```

### 3. Monitor Pool Statistics

```javascript
import { defaultPool } from './api/core/connection-pool.js';

const stats = defaultPool.stats();
console.log('Pool stats:', stats);
// { total: 5, available: 3, inUse: 2, waiting: 0 }
```

### 4. Monitor Cache Hit Rate

```javascript
import { queryCache } from './api/utils/lru-cache.js';

const stats = queryCache.stats();
console.log('Cache stats:', stats);
// { size: 50, hits: 100, misses: 20, hitRate: '83.33%' }
```

### 5. Cleanup on Error

```javascript
async function robustAutomation() {
    let page;
    try {
        page = await browser.newPage();
        await api.withPage(page, async () => {
            // ... automation
        });
    } catch (error) {
        logger.error('Automation failed', error);
        throw error;
    } finally {
        // Always cleanup
        if (page) await page.close();
        queryCache.clear();
    }
}
```

### 6. Use Connection Pool for High Volume

```javascript
// For processing many items
const pool = createPool({
    minSize: 5,
    maxSize: 20
});

// Process 100 items efficiently
const results = await Promise.all(
    Array.from({ length: 100 }, async (_, i) => {
        const conn = await pool.getConnection();
        try {
            return await processItem(i, conn.browser);
        } finally {
            await pool.releaseConnection(conn);
        }
    })
);
```

---

## Performance Targets

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Page load | < 3s | `Date.now()` timing |
| Query execution | < 100ms | Cache hit rate |
| Action execution | < 500ms | Benchmark tests |
| Memory/session | < 100MB | Process monitor |
| Connection reuse | > 80% | Pool stats |

---

## Related Documentation

- [`api/utils/lru-cache.js`](../api/utils/lru-cache.js) - LRU cache implementation
- [`api/core/connection-pool.js`](../api/core/connection-pool.js) - Connection pooling
- [docs/PERFORMANCE.md](PERFORMANCE.md) - Performance guidelines
- [docs/RECIPES.md](RECIPES.md) - Optimization patterns

---

*Last updated: 2026-03-31*
