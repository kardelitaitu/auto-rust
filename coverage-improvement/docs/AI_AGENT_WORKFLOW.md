# AI Agent Workflow for Coverage Improvement

## 🎯 Mission

Systematically improve test coverage for a Rust project by identifying gaps, writing targeted tests, and tracking progress until coverage targets are met.

## 📋 Prerequisites

- Rust project with `Cargo.toml`
- `cargo-tarpaulin` installed (`cargo install cargo-tarpaulin`)
- PowerShell environment
- Coverage improvement package dropped into project

## 🔄 Step-by-Step Workflow

### Phase 1: Assessment & Setup

1. **Verify Project Structure**
   ```powershell
   # Check for Cargo.toml
   Test-Path "Cargo.toml"
   
   # Verify coverage-improvement package exists
   Test-Path "coverage-improvement\README.md"
   ```

2. **Initial Coverage Baseline**
   ```powershell
   cd coverage-improvement
   .\scripts\coverage.ps1 -Loop
   ```

3. **Review Current State**
   - Check HTML report: `scripts\target\reports\coverage\html\index.html`
   - Review improvement items: `scripts\coverage_improvement.json`
   - Note current coverage percentage

### Phase 2: Gap Analysis

1. **Load Improvement Items**
   ```powershell
   $items = Get-Content "scripts\coverage_improvement.json" | ConvertFrom-Json
   ```

2. **Prioritize by Impact**
   - **P1Critical**: Large gaps, important functions
   - **P2Important**: Standard gaps, individual functions
   - **P3Lower**: Manual items, low priority

3. **Create Work Plan**
   ```powershell
   # Group by priority
   $critical = $items | Where-Object { $_.priority -eq "P1Critical" }
   $important = $items | Where-Object { $_.priority -eq "P2Important" }
   $lower = $items | Where-Object { $_.priority -eq "P3Lower" }
   ```

### Phase 3: Test Implementation

#### For Each Improvement Item:

1. **Analyze the Gap**
   - File: `src/file.rs`
   - Function: `function_name`
   - Lines: `[10, 11, 12]`
   - Coverage: `0%`

2. **Locate Source Code**
   ```powershell
   # Find the uncovered function/lines
   Get-Content "src/file.rs" | Select-Object -Skip 9 -First 5
   ```

3. **Write Targeted Tests**
   - Create or modify test files
   - Focus on the specific uncovered code paths
   - Ensure tests actually execute the uncovered lines
   - Follow existing test patterns and conventions

4. **Test Implementation Guidelines**
   - Use descriptive test names
   - Test both success and failure cases
   - Mock external dependencies when needed
   - Follow Rust testing best practices
   - Ensure tests are deterministic and fast

5. **Example Test Structure**
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[test]
       fn test_uncovered_function_edge_case() {
           // Test the specific uncovered code path
           let result = function_name(test_input);
           assert_eq!(expected_result, result);
       }
   }
   ```

### Phase 4: Verification & Tracking

1. **Run Tests**
   ```powershell
   cargo test
   ```

2. **Verify Coverage Improvement**
   ```powershell
   .\scripts\coverage.ps1 -Loop
   ```

3. **Mark Completed Items**
   ```powershell
   .\scripts\coverage_improvement_manager.ps1 Complete 'ITEM_ID'
   ```

4. **Update Progress**
   ```powershell
   .\scripts\coverage_improvement_manager.ps1 Stats
   ```

### Phase 5: Iteration

1. **Assess Remaining Gaps**
   - Check if coverage percentage improved
   - Review remaining improvement items
   - Identify any new gaps created

2. **Repeat Process**
   - Continue with next priority items
   - Regenerate coverage reports
   - Track cumulative progress

3. **Completion Criteria**
   - Target coverage percentage achieved (>90%)
   - All P1Critical items completed
   - Most P2Important items completed
   - Tests are stable and meaningful

## 🎯 Decision Points

### When to Focus on P1Critical Items
- Large coverage gaps (>10 lines)
- Core functionality functions
- Security-related code
- Error handling paths

### When to Skip or Defer Items
- Deprecated code (marked for removal)
- External dependencies not easily testable
- Platform-specific code unavailable in test environment
- Extremely complex edge cases with low business value

### When to Modify Test Strategy
- Coverage increases but tests don't verify functionality
- Tests are flaky or non-deterministic
- Test execution time becomes excessive
- Mocking requirements become too complex

## 📊 Progress Tracking

### Daily/Session Tracking
```powershell
# Start of session
.\scripts\coverage_improvement_manager.ps1 Stats

# End of session
.\scripts\coverage_improvement_manager.ps1 Stats
```

### Metrics to Monitor
- **Coverage Percentage**: Overall project coverage
- **Items Completed**: Number of improvement items marked Done
- **Test Pass Rate**: All tests should pass
- **Test Execution Time**: Keep tests reasonably fast

### Reporting Progress
- Provide summary of items completed
- Note coverage percentage improvements
- Identify any blockers or challenges
- Suggest next focus areas

## 🚨 Common Pitfalls & Solutions

### Pitfall: Tests Don't Increase Coverage
**Cause**: Tests don't actually execute the uncovered code paths
**Solution**: 
- Verify test inputs trigger the uncovered branches
- Check conditional logic and edge cases
- Use debugging to confirm code execution

### Pitfall: Coverage Data Structure Mismatch
**Cause**: Different coverage tool output format
**Solution**:
- Examine `scripts/coverage.json` structure
- Update parsing logic in coverage improvement manager
- Add support for new coverage format

### Pitfall: Too Many Low-Impact Items
**Cause**: Coverage tool reports every single uncovered line
**Solution**:
- Focus on function-level gaps first
- Group related line items into single test cases
- Prioritize business-critical code paths

### Pitfall: Tests Become Too Complex
**Cause**: Trying to test everything at once
**Solution**:
- Break down complex scenarios
- Use property-based testing for edge cases
- Focus on the most important code paths

## 🎯 Success Indicators

### Quantitative
- Coverage percentage > 90%
- All P1Critical items completed
- Test execution time < 5 minutes
- 100% test pass rate

### Qualitative
- Tests are meaningful and verify actual functionality
- Code is more robust with better error handling
- Team confidence in code changes increases
- Documentation is updated with test coverage notes

## 🔄 Continuous Improvement

### After Initial Coverage Goals Met
1. **Maintain Coverage**: Ensure new code includes tests
2. **Improve Test Quality**: Refactor tests for better maintainability
3. **Add Integration Tests**: Test component interactions
4. **Performance Testing**: Add benchmarks for critical paths

### Long-term Strategy
- Set up coverage gates in CI/CD
- Regular coverage reviews in team meetings
- Automated coverage trend monitoring
- Test-driven development for new features

---

**This workflow provides a systematic approach for AI agents to improve test coverage while maintaining quality and tracking progress effectively.**
