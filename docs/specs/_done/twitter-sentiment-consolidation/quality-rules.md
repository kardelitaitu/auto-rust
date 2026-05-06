# quality rules

## Code Quality Standards

- **Error Handling**: All functions return appropriate error types, no panics in production code.
- **Documentation**: All public APIs documented with examples and parameter descriptions.
- **Testing**: Unit tests for all strategies and analyzer methods, integration tests for consumer usage.
- **Performance**: No unnecessary allocations, efficient string operations.
- **Thread Safety**: Async methods properly handle concurrent access.
- **Type Safety**: Strong typing with no unsafe code.
- **Maintainability**: Clear separation of concerns, no tight coupling.

## Validation Rules

- **Compilation**: Must compile without warnings on stable Rust.
- **Test Coverage**: All public functions have corresponding tests.
- **API Stability**: Changes don't break existing consumer code.
- **Performance**: Analysis operations complete within reasonable time limits.
