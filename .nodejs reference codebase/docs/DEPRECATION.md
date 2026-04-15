# Deprecation Policy

Policy for deprecating features, methods, and configuration in Auto-AI.

---

## Deprecation Levels

### Level 1: Soft Deprecation

**Usage:** Documentation only

```javascript
/**
 * @deprecated Use api.click() instead
 */
async function legacyClick(selector) {
    // Implementation
}
```

**When to use:**
- Minor API improvements
- Non-critical methods
- Early warning

---

### Level 2: Runtime Warning

**Usage:** Console warning on use

```javascript
async function legacyClick(selector) {
    console.warn('legacyClick is deprecated. Use api.click() instead.');
    // Implementation
}
```

**When to use:**
- Methods used in development
- Features with clear replacements
- 6+ months before removal

---

### Level 3: Logged Warning

**Usage:** Proper logging with context

```javascript
import { createLogger } from '../core/logger.js';
const logger = createLogger('deprecation');

async function legacyClick(selector) {
    logger.warn('DEPRECATED: legacyClick() called. Use api.click() instead. Will be removed in v2.0.0');
    // Implementation
}
```

**When to use:**
- Production code
- Frequently used methods
- 3+ months before removal

---

### Level 4: Error After Deadline

**Usage:** Throws error after removal version

```javascript
async function legacyClick(selector) {
    const currentVersion = getVersion();
    const removalVersion = '2.0.0';
    
    if (isVersionGte(currentVersion, removalVersion)) {
        throw new Error(
            `legacyClick was removed in v${removalVersion}. Use api.click() instead.`
        );
    }
    
    logger.warn('DEPRECATED: legacyClick() - will be removed in v2.0.0');
    // Implementation
}
```

**When to use:**
- Final warning before removal
- Critical migrations only

---

## Deprecation Annotation

For standardized deprecation warnings:

```javascript
function deprecated(oldName, newName, removalVersion) {
    const warned = new Set();
    
    return function(...args) {
        const key = `${oldName}-${newName}`;
        if (!warned.has(key)) {
            console.warn(
                `⚠️  DEPRECATED: ${oldName}() is deprecated and will be removed in v${removalVersion}. ` +
                `Use ${newName}() instead.`
            );
            warned.add(key);
        }
        return this[oldName](...args);
    };
}

// Usage
MyClass.prototype.legacyClick = deprecated('legacyClick', 'click', '2.0.0');
```

---

## Deprecation Timeline

### Example: Deprecating `api.legacyClick()`

| Version | Date | Action |
|---------|------|--------|
| 1.3.0 | 2026-04-01 | Add `@deprecated` tag, add `api.click()` |
| 1.3.0 | 2026-04-01 | Add runtime warning |
| 1.4.0 | 2026-05-01 | Enhance warning with removal version |
| 1.5.0 | 2026-06-01 | Final warning release |
| 2.0.0 | 2026-07-01 | Remove `api.legacyClick()` |

**Minimum timeline:** 3 minor versions or 1 major version

---

## Communication

### Documentation Updates

When deprecating:

1. Update JSDoc with `@deprecated` tag
2. Add deprecation notice to method docs
3. Update migration guide
4. Add to CHANGELOG.md

### CHANGELOG Entry

```markdown
## [1.3.0] - 2026-04-01

### Deprecated

- `api.legacyClick()` - Use `api.click()` instead. Will be removed in v2.0.0
- `config.humanize` - Use `config.humanization.enabled` instead

### Added

- `api.click()` - New click method with better recovery
```

### Migration Guide

Create `docs/migration/v2.md` for major version changes.

---

## Configuration Deprecation

### Deprecating Config Options

```json
{
  "humanize": true,
  "_deprecated_humanize": "Use humanization.enabled instead. Removed in v2.0.0"
}
```

### Backward Compatibility

```javascript
function loadConfig() {
    const config = loadJson();
    
    // Handle deprecated option
    if (config.humanize !== undefined) {
        console.warn('config.humanize is deprecated. Use config.humanization.enabled');
        config.humanization = config.humanization || {};
        config.humanization.enabled = config.humanize;
    }
    
    return config;
}
```

---

## Browser Support Deprecation

### Announcing End of Support

```markdown
## Browser Support Changes

Support for the following browsers will end in v1.5.0:

- LegacyBrowser v1.x
- OldAntiDetect v2.x

**Migration:**
- Upgrade to LegacyBrowser v2.x
- Switch to ModernBrowser

**Reason:** These browsers no longer receive security updates.
```

---

## Enforcement

### Pre-commit Hook

```javascript
// scripts/check-deprecations.js
const deprecated = findDeprecatedUsage(code);
if (deprecated) {
    console.error(`Error: Using deprecated ${deprecated.name}. Use ${deprecated.replacement} instead.`);
    process.exit(1);
}
```

### CI Check

```yaml
# .github/workflows/quality.yml
- name: Check deprecations
  run: node scripts/check-deprecations.js
```

---

## Related Documentation

- [VERSIONING.md](VERSIONING.md) - API versioning policy
- [CHANGELOG.md](../CHANGELOG.md) - Version history
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guidelines
