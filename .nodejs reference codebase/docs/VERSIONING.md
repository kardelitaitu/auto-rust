# API Versioning Policy

Policy for versioning the Auto-AI public API.

---

## Version Numbering

Auto-AI uses [Semantic Versioning](https://semver.org/):

```
MAJOR.MINOR.PATCH
```

- **MAJOR**: Breaking API changes
- **MINOR**: Backward-compatible features
- **PATCH**: Backward-compatible bug fixes

### Examples

- `1.2.0` → `1.2.1` - Bug fix patch
- `1.2.0` → `1.3.0` - New feature
- `1.2.0` → `2.0.0` - Breaking change

---

## What Constitutes a Breaking Change

### Major Version (Breaking)

- Removing a public API method
- Changing method signatures
- Changing return types
- Removing configuration options
- Changing default behavior significantly
- Removing support for a browser type

### Minor Version (Non-Breaking)

- Adding new API methods
- Adding new configuration options
- Adding new browser support
- Performance improvements
- Bug fixes that don't change behavior

### Patch Version (Non-Breaking)

- Bug fixes
- Performance optimizations
- Documentation updates
- Internal refactoring

---

## Deprecation Process

### Phase 1: Announce Deprecation (Minor Release)

```javascript
/**
 * @deprecated Use api.click() instead. Will be removed in v2.0.0
 */
async function legacyClick(selector) {
    // ...
}
```

- Add `@deprecated` JSDoc tag
- Add runtime deprecation warning
- Update documentation
- Set removal version

### Phase 2: Maintain (Minor Releases)

- Keep deprecated method working
- Log deprecation warnings
- Document migration path

### Phase 3: Remove (Major Release)

- Remove deprecated method
- Update migration guide
- Document in changelog

---

## Deprecation Timeline

| Change Type | Notice Period | Example |
|-------------|---------------|---------|
| Method removal | 1 major version | Deprecated in 1.x, removed in 2.0 |
| Signature change | 1 major version | Old signature works until 2.0 |
| Config option | 1 major version | Old option works until 2.0 |
| Browser support | 2 minor versions | Support ends in 1.4.0 |

---

## Migration Guides

For each major version, create a migration guide:

```markdown
# Migration Guide: v1.x → v2.0

## Breaking Changes

### api.legacyClick() removed

**Before:**
```javascript
await api.legacyClick('.btn');
```

**After:**
```javascript
await api.click('.btn');
```

### Config option renamed

**Before:**
```json
{ "humanize": true }
```

**After:**
```json
{ "humanization": { "enabled": true } }
```
```

---

## API Stability Guarantee

### Stable API

Methods marked as stable will:

- Not be removed without deprecation
- Maintain signature compatibility
- Have tests and documentation

### Experimental API

Methods marked as experimental:

- May change without deprecation
- May be removed in minor releases
- May have limited documentation

**Marking Experimental:**

```javascript
/**
 * @experimental May change in future releases
 */
async function experimentalFeature() {
    // ...
}
```

---

## Version Support

| Version | Support Status | End Date |
|---------|----------------|----------|
| 2.x     | Current        | -        |
| 1.x     | Maintenance    | 2026-12-31 |
| 0.x     | End of Life    | 2026-03-31 |

### Support Levels

**Current:**
- Bug fixes
- Security updates
- New features

**Maintenance:**
- Security updates only
- Critical bug fixes

**End of Life:**
- No updates
- Upgrade recommended

---

## Release Schedule

| Release Type | Frequency |
|--------------|-----------|
| Patch        | As needed |
| Minor        | Monthly   |
| Major        | Quarterly |

---

## Related Documentation

- [CHANGELOG.md](../CHANGELOG.md) - Version history
- [DEPRECATION.md](DEPRECATION.md) - Deprecation details
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guidelines
