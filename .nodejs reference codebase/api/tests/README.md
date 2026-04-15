# Test Suite Organization

This directory contains the test suite for the Auto-AI project, organized using the `@tests` alias pattern.

## Directory Structure

```
tests/
├── README.md                              # This file
├── index.js                               # Main entry point with all exports
│
├── unit/                                  # Unit tests (fast, isolated tests)
│   ├── README.md                          # Unit tests documentation
│   ├── index.js                           # Unit tests index
│   ├── ai-twitter-activity.test.js        # AI Twitter Activity task tests
│   ├── async-queue.test.js                # Async queue functionality
│   ├── engagement-limits.test.js          # Engagement limit logic
│   ├── human-interaction.test.js          # Human interaction utilities
│   ├── test-smart-prob.js                 # Smart probability redistribution
│   ├── test-action-config.js              # Action configuration tests
│   ├── test-actions.js                    # Actions tests
│   ├── test-simple-dive.js                # Simple dive tests
│   ├── test-human-methods.js              # Human methods tests
│   ├── test-modular-methods.js            # Modular methods tests
│   └── test-reply-method.js               # Reply method tests
│
├── integration/                           # Integration tests (component interaction)
│   ├── README.md                          # Integration tests documentation
│   ├── index.js                           # Integration tests index
│   ├── agent-connector-health.test.js     # Agent connector health monitoring
│   ├── circuit-breaker.test.js            # Circuit breaker pattern
│   ├── request-queue.test.js              # Request queue functionality
│   ├── test-core-modules.js               # Core module integration tests
│   ├── test-dedupe.js                     # API deduplication tests
│   ├── test-ai-reply-engine.js            # AI reply engine tests
│   ├── test-cloud-prompt-fix.js           # Cloud prompt fix tests
│   ├── test-cloud-client-multi.js         # Cloud client multi tests
│   ├── test-multi-api.js                  # Multi API tests
│   └── test-cloud-api.js                  # Cloud API tests
│
└── edge-cases/                            # Edge case and boundary tests
    ├── README.md                          # Edge cases documentation
    ├── index.js                           # Edge cases index
    ├── phase1-3-validation.js             # Phase 1-3 validation tests
    ├── test-multiline-tweet.js            # Multiline tweet handling
    ├── test-models.js                     # Model-specific tests
    └── test-dive-lock.js                  # Dive lock edge cases
```

## Using the @tests Alias

Import test utilities using the `@tests` alias:

```javascript
// Import from unit tests
import { describe, it, expect } from 'vitest';

// Import shared test utilities
import { mockPage, mockAgent } from '@tests/unit/mocks.js';

// Import from edge cases
import { testEdgeCases } from '@tests/edge-cases/utils.js';
```

## Running Tests

### All Tests

```bash
npm test
```

### Unit Tests Only

```bash
pnpm run test:unit
# or
npx vitest run tests/unit
```

### Integration Tests Only

```bash
pnpm run test:integration
# or
npx vitest run tests/integration
```

### Edge Case Tests Only

```bash
npx vitest run tests/edge-cases
```

### Watch Mode

```bash
pnpm run test:watch
```

## Test Categories

### Unit Tests

- **Purpose**: Test individual functions/methods in isolation
- **Speed**: Fast (< 100ms per test)
- **Characteristics**:
    - No external dependencies (database, network, etc.)
    - Mock all external services
    - Test one specific behavior

### Integration Tests

- **Purpose**: Test component interactions
- **Speed**: Medium (100ms - 1s per test)
- **Characteristics**:
    - May use real configurations
    - Test how components work together
    - Verify API contracts

### Edge Case Tests

- **Purpose**: Test boundary conditions and error paths
- **Speed**: Varies
- **Characteristics**:
    - Test invalid inputs
    - Test timeout scenarios
    - Test recovery mechanisms

## Best Practices

1. **Naming**: Test files should end with `.test.js`
2. **Isolation**: Each test should be independent
3. **Descriptive**: Use descriptive test names
4. **Coverage**: Aim for meaningful coverage, not 100%
5. **Speed**: Unit tests should be fast (< 100ms)
6. **Mocking**: Mock external dependencies in unit tests

## Example Test Structure

```javascript
import { describe, it, expect, vi } from 'vitest';

describe('Module Name', () => {
    describe('Function/Feature', () => {
        it('should handle normal case', () => {
            // Arrange
            const input = 'valid-input';

            // Act
            const result = functionUnderTest(input);

            // Assert
            expect(result).toBe('expected-output');
        });

        it('should handle edge case', () => {
            // Arrange
            const input = null;

            // Act & Assert
            expect(() => functionUnderTest(input)).toThrow();
        });
    });
});
```

## CI/CD Integration

Tests run automatically on:

- Pull requests
- Commits to main branch
- Scheduled daily runs

Failed tests will block merges until fixed.
