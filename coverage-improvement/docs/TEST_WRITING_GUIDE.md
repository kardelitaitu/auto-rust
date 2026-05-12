# Test Writing Guide for Coverage Improvement

## 🎯 Purpose

This guide provides best practices for writing effective tests that increase coverage while maintaining quality and maintainability.

## 🏗️ Rust Testing Fundamentals

### Test Structure
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_function_name_scenario() {
        // Arrange: Set up test data and state
        let input = create_test_input();
        
        // Act: Execute the function being tested
        let result = function_under_test(input);
        
        // Assert: Verify the expected outcome
        assert_eq!(expected_result, result);
    }
}
```

### Test File Organization
```
src/
├── lib.rs              # Main module
├── utils.rs            # Utility functions
└── my_module.rs        # Feature module

tests/
├── integration_tests.rs    # Integration tests
├── common/
│   └── test_utils.rs        # Shared test utilities
└── my_module_tests.rs       # Module-specific tests
```

## 🎯 Coverage-Driven Test Strategies

### Strategy 1: Function-Level Testing
Focus on testing entire functions rather than individual lines.

```rust
// Instead of testing each line separately
#[test]
fn test_process_data_complete_flow() {
    // Test all paths through the function
    let result = process_data(valid_input);
    assert!(result.is_ok());
    
    let result = process_data(invalid_input);
    assert!(result.is_err());
}
```

### Strategy 2: Edge Case Coverage
Identify and test boundary conditions and error cases.

```rust
#[test]
fn test_format_output_edge_cases() {
    // Test empty input
    assert_eq!(format_output(""), "default");
    
    // Test maximum length
    let long_input = "a".repeat(1000);
    assert!(format_output(&long_input).len() <= 100);
    
    // Test special characters
    assert_eq!(format_output("hello\nworld"), "hello world");
}
```

### Strategy 3: Error Path Testing
Ensure all error handling code is executed.

```rust
#[test]
fn test_error_handling_paths() {
    // Test each error condition
    assert_eq!(parse_config("invalid"), Err(ConfigError::InvalidFormat));
    assert_eq!(parse_config(""), Err(ConfigError::EmptyInput));
    assert_eq!(parse_config("missing_key"), Err(ConfigError::MissingRequired));
}
```

## 🔧 Test Implementation Patterns

### Pattern 1: Data-Driven Tests
Use test parameterization for multiple scenarios.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_calculate_discount_various_cases() {
        let test_cases = vec![
            (100.0, 0.1, 90.0),
            (100.0, 0.2, 80.0),
            (100.0, 0.5, 50.0),
            (0.0, 0.1, 0.0),
        ];
        
        for (price, discount, expected) in test_cases {
            let result = calculate_discount(price, discount);
            assert_eq!(result, expected);
        }
    }
}
```

### Pattern 2: Mock External Dependencies
Use test doubles for external services.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    
    mock! {
        ExternalService {}
        fn fetch_data(&self, id: &str) -> Result<String, ServiceError>;
    }
    
    #[test]
    fn test_process_with_mock_service() {
        let mut mock_service = MockExternalService::new();
        mock_service
            .expect_fetch_data()
            .with(eq("test123"))
            .return_const(Ok("test data".to_string()));
        
        let result = process_with_service(&mock_service, "test123");
        assert!(result.is_ok());
    }
}
```

### Pattern 3: Property-Based Testing
Use quickcheck for randomized testing.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{QuickCheck, TestResult};
    
    #[test]
    fn test_sort_properties() {
        fn prop(xs: Vec<i32>) -> TestResult {
            let mut sorted = xs.clone();
            sort_vector(&mut sorted);
            
            // Property: sorted vector is ordered
            let is_ordered = sorted.windows(2).all(|w| w[0] <= w[1]);
            
            // Property: sorted vector contains same elements
            let mut original_sorted = xs.clone();
            original_sorted.sort();
            let same_elements = sorted == original_sorted;
            
            TestResult::from_bool(is_ordered && same_elements)
        }
        
        QuickCheck::new().quicktest(prop as fn(Vec<i32>) -> TestResult);
    }
}
```

## 🎯 Coverage Gap Specific Strategies

### Uncovered Error Handling
```rust
#[test]
fn test_uncovered_error_paths() {
    // Force error conditions
    let result = function_with_error_handling(None);
    assert!(matches!(result, Err(Error::InvalidInput)));
    
    let result = function_with_error_handling(Some(invalid_value));
    assert!(matches!(result, Err(Error::ValidationError)));
}
```

### Uncovered Conditional Branches
```rust
#[test]
fn test_conditional_branches() {
    // Test each branch condition
    assert_eq!(conditional_function(true), "branch1");
    assert_eq!(conditional_function(false), "branch2");
    assert_eq!(conditional_function(None), "default");
}
```

### Uncovered Loop Edge Cases
```rust
#[test]
fn test_loop_edge_cases() {
    // Test empty collection
    assert_eq!(process_vector(&vec![]), 0);
    
    // Test single element
    assert_eq!(process_vector(&vec![1]), 1);
    
    // Test maximum capacity
    let large_vec = vec![1; 1000];
    assert!(process_vector(&large_vec) > 0);
}
```

## 📋 Test Quality Checklist

### Before Writing Tests
- [ ] Understand the function's purpose and contract
- [ ] Identify normal, edge, and error cases
- [ ] Review existing test patterns in the codebase
- [ ] Check for external dependencies that need mocking
- [ ] Plan test data and setup requirements

### During Test Implementation
- [ ] Use descriptive test names that explain what's being tested
- [ ] Follow Arrange-Act-Assert pattern
- [ ] Test behavior, not implementation details
- [ ] Use appropriate assertion methods
- [ ] Keep tests focused and independent

### After Writing Tests
- [ ] Run tests and verify they pass
- [ ] Check that coverage actually increases
- [ ] Review test clarity and maintainability
- [ ] Ensure tests run quickly
- [ ] Add documentation for complex test scenarios

## 🚨 Common Pitfalls to Avoid

### Pitfall 1: Testing Implementation Details
```rust
// Bad: Tests internal state
#[test]
fn test_internal_counter() {
    let obj = MyClass::new();
    obj.increment(); // Tests private counter
    assert_eq!(obj.get_internal_count(), 1);
}

// Good: Tests public behavior
#[test]
fn test_public_api_behavior() {
    let obj = MyClass::new();
    let result = obj.process(input);
    assert_eq!(result, expected_output);
}
```

### Pitfall 2: Fragile Tests
```rust
// Bad: Depends on exact timing or order
#[test]
fn test_timing_dependent() {
    let start = SystemTime::now();
    let result = function_with_delay();
    let elapsed = start.elapsed().unwrap();
    assert!(elapsed.as_millis() > 100); // Fragile
}

// Good: Uses deterministic behavior
#[test]
fn test_deterministic_behavior() {
    let result = function_with_controlled_delay(Duration::from_millis(100));
    assert_eq!(result, expected_result);
}
```

### Pitfall 3: Over-Mocking
```rust
// Bad: Mocks everything, tests nothing
#[test]
fn test_over_mocked() {
    let mock_service = MockService::new();
    mock_service.expect_get_data().return_const(Ok(vec![]));
    mock_service.expect_process_data().return_const(Ok(()));
    mock_service.expect_save_data().return_const(Ok(()));
    
    // Test only verifies mocks work
    assert!(process_with_mocks(&mock_service).is_ok());
}

// Good: Tests real logic with minimal mocking
#[test]
fn test_core_logic_with_minimal_mocks() {
    let mock_service = MockService::new();
    mock_service.expect_get_data().return_const(Ok(test_data()));
    
    // Test actual processing logic
    let result = process_logic(&mock_service);
    assert_eq!(result.processed_count, expected_count);
}
```

## 🔧 Advanced Testing Techniques

### Custom Test Helpers
```rust
// tests/common/test_utils.rs
pub fn create_test_user() -> User {
    User {
        id: "test123".to_string(),
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
        created_at: Utc::now(),
    }
}

pub fn assert_user_eq(actual: &User, expected: &User) {
    assert_eq!(actual.id, expected.id);
    assert_eq!(actual.name, expected.name);
    assert_eq!(actual.email, expected.email);
    // Skip timestamp comparison for tests
}
```

### Integration Test Patterns
```rust
// tests/integration_tests.rs
use my_project::*;

#[test]
fn test_complete_workflow() {
    // Test entire user journey
    let app = setup_test_app();
    let user = create_test_user(&app);
    let result = process_user_data(&app, &user);
    assert!(result.is_success());
    
    // Verify side effects
    let saved_data = get_user_data(&app, &user.id);
    assert!(saved_data.is_processed);
}
```

### Performance Testing
```rust
#[test]
fn test_performance_constraints() {
    let start = Instant::now();
    let result = expensive_operation(test_data);
    let duration = start.elapsed();
    
    assert!(result.is_valid());
    assert!(duration.as_millis() < 1000, "Operation took too long: {:?}", duration);
}
```

## 📈 Measuring Test Effectiveness

### Coverage Impact Metrics
- **Lines Added**: Number of newly covered lines
- **Functions Covered**: Number of newly covered functions
- **Branch Coverage**: Improvement in conditional coverage
- **Test Quality**: Tests that verify meaningful behavior

### Test Maintenance Metrics
- **Test Execution Time**: Keep tests fast
- **Test Flakiness**: Tests should be deterministic
- **Test Complexity**: Simple tests are easier to maintain
- **Documentation**: Tests should serve as usage examples

---

**This guide provides comprehensive strategies for writing effective tests that improve coverage while maintaining code quality and test maintainability.**
