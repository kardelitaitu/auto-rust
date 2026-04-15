# Auto-AI Integration Test Plan

## Current State

- Integration test coverage: 8.53% statements, 6.62% branches, 9.56% functions, 8.74% lines
- Goal: Reach 90%+ coverage while exercising all critical user journeys

## Priority Areas for Test Creation

### 1. Authentication & API Integration (Highest Priority)

**Rationale**: External service boundaries are critical for framework functionality and often have complex error handling paths.

#### 1.1 LLM Provider Integration

- Test `api/utils/free-openrouter-helper.js` - OpenRouter API key handling
- Test `api/utils/local-ollama-manager.js` - Local Ollama instance management
- Test `api/core/ollama-client.js` - Ollama client communication
- Test `api/core/cloud-client.js` - Cloud API communication
- Test `api/utils/free-api-router.js` - API key rotation and failover
- Test `api/utils/api-key-timeout-tracker.js` - Rate limiting and timeout handling

#### 1.2 Configuration & Initialization

- Test `api/core/config.js` - Configuration loading and validation
- Test `api/core/init.js` - System initialization sequences
- Test `api/core/logger.js` - Logging configuration and output

### 2. Core Business Workflows (High Priority)

**Rationale**: These represent the main value proposition of the framework - browser automation orchestration.

#### 2.1 Orchestration & Task Management

- Test `api/core/orchestrator.js` - Task dispatch, worker management, timeout handling
- Test `api/core/sessionManager.js` - Worker lifecycle, health monitoring, session persistence
- Test `api/core/context.js` - AsyncLocalStorage context isolation
- Test `api/core/context-state.js` - Session-bound state management
- Test `api/core/request-queue.js` - Request queuing and prioritization
- Test `api/core/circuit-breaker.js` - Failure detection and recovery
- Test `api/core/health-monitor.js` - Worker health checking
- Test `api/core/automator.js` - Browser automation execution layer

#### 2.2 Agent & AI Functionality

- Test `api/agent/index.js` - Main agent entry point
- Test `api/agent/runner.js` - Agent execution loop
- Test `api/agent/ai-reply-engine/` - AI decision making components
- Test `api/agent/finder.js` - Element finding strategies
- Test `api/agent/observer.js` - Page change detection
- Test `api/agent/executor.js` - Action execution
- Test `api/agent/memoryInjector.js` - Context injection for AI
- Test `api/agent/tokenCounter.js` - Token usage tracking
- Test `api/agent/vision.js` - Visual processing capabilities

#### 2.3 Interactions & Behaviors

- Test `api/interactions/` - Core interaction primitives (click, type, scroll, etc.)
- Test `api/behaviors/` - Humanization patterns (mouse movement, timing, idle behavior)
- Test `api/behaviors/human-timing.js` - Human-like timing variations
- Test `api/behaviors/motor-control.js` - Mouse movement smoothing
- Test `api/behaviors/scroll-helper.js` - Scrolling behavior simulation
- Test `api/behaviors/idle.js` - Idle behavior simulation
- Test `api/behaviors/micro-interactions.js` - Small random movements
- Test `api/behaviors/persona.js` - Persona-based behavior variation

### 3. External Service Boundaries (High Priority)

**Rationale**: These are integration points with external systems that must be reliable.

#### 3.1 Browser Connectors

- Test `api/connectors/discovery/` - Browser discovery mechanisms
- Test `api/connectors/baseDiscover.js` - Base discovery interface
- Test vendor-specific connectors (ixbrowser, morelogin, local browsers, etc.)

#### 3.2 Plugin System

- Test `api/core/plugins/` - Plugin loading and execution
- Test `api/core/plugins/manager.js` - Plugin lifecycle management

#### 3.3 Vision & Image Processing

- Test `api/core/vision/` - Image storage and ROI detection
- Test `api/agent/vision.js` - Visual processing for agents

### 4. Error Handling & Resilience (Medium Priority)

**Rationale**: Robust error handling is essential for production reliability.

#### 4.1 Error Recovery

- Test `api/agent/errorRecoveryPrompt.js` - AI-guided error recovery
- Test `api/agent/selfHealingPrompt.js` - Self-healing capabilities
- Test `api/agent/retryStrategy.js` - Retry mechanisms with backoff
- Test `api/core/errors.js` - Error classification and handling

#### 4.2 Network & Failure Scenarios

- Test timeout handling across all external API calls
- Test circuit breaker activation and recovery
- Test request queue behavior under failure conditions
- Test session recovery after worker crashes

### 5. Performance & Edge Cases (Medium Priority)

**Rationale**: Performance characteristics and edge cases affect real-world usability.

#### 5.1 Concurrency & Resource Management

- Test session manager under high load
- Test request queue prioritization under load
- Test worker health monitoring performance
- Test memory usage patterns

#### 5.2 Edge Cases

- Test empty/invalid inputs to all public APIs
- Test boundary conditions in timing and retry logic
- Test race conditions in session/context management
- Test cleanup and resource disposal

## Test Implementation Approach

### 5.1 Mocking Strategy

- Use `vi.mock()` extensively for external dependencies
- Mock browser pages and CDP interfaces
- Mock HTTP requests to LLM APIs and cloud services
- Mock file system operations where appropriate
- Use existing test fixtures and mocks from `api/tests/mocks/`

### 5.2 Test Structure

- Follow existing patterns in `api/tests/integration/`
- Use `describe()` blocks for logical grouping
- Use `beforeEach()`/`afterEach()` for test isolation
- Test both success and failure paths
- Include performance assertions where relevant

### 5.3 Coverage Targets

- Aim for 90%+ statement coverage in all priority modules
- Ensure branch coverage reflects decision points
- Test all public API methods and exported functions
- Cover error handling paths thoroughly

## Immediate Next Steps (Iteration 1)

Based on the coverage analysis, the lowest coverage files that should be prioritized for initial test creation:

1. `api/agent/errorRecoveryPrompt.js` (0.011% statement coverage)
2. `api/agent/finder.js` (0.05% statement coverage)
3. `api/agent/historyManager.js` (0.020% statement coverage)
4. `api/agent/memoryInjector.js` (0.031% statement coverage)
5. `api/agent/observer.js` (0.017% statement coverage)
6. `api/agent/promptAdapter.js` (0.024% statement coverage)
7. `api/agent/responseCache.js` (0.029% statement coverage)
8. `api/agent/responseValidator.js` (0.040% statement coverage)
9. `api/agent/retryStrategy.js` (0.031% statement coverage)
10. `api/agent/selfHealingPrompt.js` (0.032% statement coverage)

These agent components represent critical AI decision-making and execution paths that are currently nearly untested.

## Success Criteria

- Integration test coverage reaches ≥90% statements
- All critical user journeys are exercised (authentication → task execution → result handling)
- Error handling paths are tested for all external service integrations
- Performance characteristics validated under load
- No regressions in existing functionality
