# Implementation Plan: Electron Update & Security Testing

## Overview

This plan covers three key improvements to the electron-dashboard:

1. Update Electron to latest stable version for security patches
2. Add unit tests for security-critical functions
3. Add integration tests for WebSocket communication

---

## Task 1: Update Electron to Latest Stable Version

### Current State

- **Current Version:** `^28.0.0` (released Dec 2023)
- **Latest Stable:** `33.x` (as of March 2026)
- **Security Risk:** Multiple CVEs patched in versions 29-33

### Implementation Steps

#### Step 1.1: Research Breaking Changes

- [ ] Review Electron breaking changes from v28 → v33
- [ ] Check Node.js compatibility (current: >=16.0.0)
- [ ] Review electron-builder compatibility with Electron 33

#### Step 1.2: Update package.json

- [ ] Update `electron` dependency to `^33.0.0`
- [ ] Update `electron-builder` to latest compatible version
- [ ] Update Node.js engine requirement if needed

#### Step 1.3: Update Code for Breaking Changes

- [ ] Review and update `main.js` for API changes
- [ ] Review and update `preload.mjs` for contextBridge changes
- [ ] Update any deprecated Electron APIs

#### Step 1.4: Testing

- [ ] Run existing tests to verify no regressions
- [ ] Manual testing of Electron app startup
- [ ] Test window creation and IPC communication
- [ ] Test system tray and notifications

#### Step 1.5: Build Verification

- [ ] Run `npm run build:renderer`
- [ ] Run `npm run dist` to verify packaging
- [ ] Test built executable on clean system

### Risk Assessment

| Risk                 | Impact | Mitigation                            |
| -------------------- | ------ | ------------------------------------- |
| Breaking API changes | High   | Thorough testing, incremental updates |
| Build failures       | Medium | Update electron-builder first         |
| Renderer issues      | Low    | Test React app in dev mode            |

---

## Task 2: Add Unit Tests for Security-Critical Functions

### Functions to Test

#### 2.1: Authentication Functions (`dashboard.js`)

**`isAuthenticated(data)`**

- [ ] Test: Returns `true` when `AUTH_ENABLED` is `false`
- [ ] Test: Returns `false` when `AUTH_ENABLED` is `true` and `AUTH_TOKEN` is empty
- [ ] Test: Returns `false` when `AUTH_ENABLED` is `true` and token doesn't match
- [ ] Test: Returns `true` when `AUTH_ENABLED` is `true` and token matches
- [ ] Test: Handles `data.token` correctly
- [ ] Test: Handles `data.authToken` correctly

**`withAuth(eventName, handler)`**

- [ ] Test: Calls handler when authenticated
- [ ] Test: Does not call handler when not authenticated
- [ ] Test: Strips auth fields before passing to handler
- [ ] Test: Logs warning on unauthorized attempt

**`requireAuth(req, res, next)` (HTTP middleware)**

- [ ] Test: Calls `next()` when `AUTH_ENABLED` is `false`
- [ ] Test: Returns 401 when token is missing
- [ ] Test: Returns 401 when token doesn't match
- [ ] Test: Calls `next()` when token matches
- [ ] Test: Checks `x-auth-token` header
- [ ] Test: Checks `token` query parameter

#### 2.2: Validation Functions (`dashboard.js`)

**`validateTask(task)`**

- [ ] Test: Returns `null` for null/undefined/non-object
- [ ] Test: Returns `null` for empty object
- [ ] Test: Returns validated task with valid fields only
- [ ] Test: Ignores invalid fields

**`validateSession(session)`**

- [ ] Test: Returns `null` for invalid input
- [ ] Test: Returns validated session with valid fields only

**`validatePayload(payload)`**

- [ ] Test: Returns `null` for invalid input
- [ ] Test: Validates nested sessions array
- [ ] Test: Validates nested recentTasks array
- [ ] Test: Validates nested metrics object
- [ ] Test: Filters non-string errors

#### 2.3: Rate Limiting (`dashboard.js`)

**Rate limit middleware**

- [ ] Test: Allows requests under limit
- [ ] Test: Blocks requests over limit
- [ ] Test: Resets after window expires
- [ ] Test: Cleanup interval removes old entries

#### 2.4: Sanitization Functions (`dashboard.js`)

**`sanitizeLogString(str)`**

- [ ] Test: Removes control characters
- [ ] Test: Truncates to 1000 chars
- [ ] Test: Returns non-strings unchanged

**`sanitizeObject(obj)`**

- [ ] Test: Recursively sanitizes strings
- [ ] Test: Handles null and primitives
- [ ] Test: Preserves object structure

### Test File Structure

```
tests/
├── unit/
│   ├── validation.test.js          # Existing - update if needed
│   ├── authentication.test.js      # NEW - auth functions
│   ├── rate-limiting.test.js       # NEW - rate limiting
│   └── sanitization.test.js        # NEW - sanitization functions
```

### Implementation Steps

#### Step 2.1: Create Authentication Tests

- [ ] Create `tests/unit/authentication.test.js`
- [ ] Mock `AUTH_ENABLED` and `AUTH_TOKEN` constants
- [ ] Implement test cases for `isAuthenticated`
- [ ] Implement test cases for `withAuth`
- [ ] Implement test cases for `requireAuth`

#### Step 2.2: Create Rate Limiting Tests

- [ ] Create `tests/unit/rate-limiting.test.js`
- [ ] Mock Express request/response objects
- [ ] Implement test cases for rate limit behavior
- [ ] Implement test cases for cleanup interval

#### Step 2.3: Create Sanitization Tests

- [ ] Create `tests/unit/sanitization.test.js`
- [ ] Implement test cases for `sanitizeLogString`
- [ ] Implement test cases for `sanitizeObject`

#### Step 2.4: Update Existing Tests

- [ ] Review `validation.test.js` for completeness
- [ ] Add any missing test cases

#### Step 2.5: Run Tests

- [ ] Run `npm test` to verify all tests pass
- [ ] Run `npm run test:coverage` to check coverage
- [ ] Fix any failing tests

---

## Task 3: Add Integration Tests for WebSocket Communication

### Test Scenarios

#### 3.1: Connection Management

- [ ] Test: Client can connect to WebSocket server
- [ ] Test: Client receives initial metrics on connect
- [ ] Test: Client disconnects cleanly
- [ ] Test: Server tracks connected clients correctly

#### 3.2: Metrics Broadcasting

- [ ] Test: Server broadcasts metrics on interval
- [ ] Test: Broadcast includes all required fields
- [ ] Test: Broadcast skips when no clients connected
- [ ] Test: Broadcast resumes when client reconnects

#### 3.3: Socket Events

**`push_metrics` event**

- [ ] Test: Server updates metrics when valid payload received
- [ ] Test: Server ignores invalid payload
- [ ] Test: Server requires auth when enabled

**`task-update` event**

- [ ] Test: Server merges task data correctly
- [ ] Test: Server broadcasts updated metrics after task update
- [ ] Test: Server requires auth when enabled

**`clear-history` event**

- [ ] Test: Server clears history when authorized
- [ ] Test: Server ignores when not authorized
- [ ] Test: Server broadcasts cleared metrics

**`requestUpdate` event**

- [ ] Test: Server sends metrics to requesting client

#### 3.4: Error Handling

- [ ] Test: Server handles malformed JSON gracefully
- [ ] Test: Server handles disconnected clients
- [ ] Test: Circuit breaker pauses after consecutive errors

### Test File Structure

```
tests/
└── integration/
    ├── api.test.js                 # Existing - update if needed
    ├── websocket.test.js           # NEW - WebSocket communication
    └── authentication-flow.test.js # NEW - auth flow integration
```

### Implementation Steps

#### Step 3.1: Create WebSocket Integration Tests

- [ ] Create `tests/integration/websocket.test.js`
- [ ] Set up Socket.io client for testing
- [ ] Implement connection tests
- [ ] Implement metrics broadcast tests
- [ ] Implement event handler tests

#### Step 3.2: Create Authentication Flow Tests

- [ ] Create `tests/integration/authentication-flow.test.js`
- [ ] Test auth-enabled scenario
- [ ] Test auth-disabled scenario
- [ ] Test token validation flow

#### Step 3.3: Update Existing Integration Tests

- [ ] Review `api.test.js` for completeness
- [ ] Add auth middleware tests to export endpoints

#### Step 3.4: Run Tests

- [ ] Run `npm test` to verify all tests pass
- [ ] Run `npm run test:coverage` to check coverage
- [ ] Fix any failing tests

---

## Execution Order

### Phase 1: Electron Update (High Priority)

1. Research breaking changes
2. Update dependencies
3. Fix any breaking changes
4. Test thoroughly

### Phase 2: Unit Tests (Medium Priority)

1. Create authentication tests
2. Create rate limiting tests
3. Create sanitization tests
4. Update existing tests

### Phase 3: Integration Tests (Medium Priority)

1. Create WebSocket tests
2. Create auth flow tests
3. Update existing integration tests

---

## Success Criteria

### Electron Update

- [ ] Electron version >= 33.0.0
- [ ] All existing tests pass
- [ ] App builds successfully
- [ ] Manual testing confirms functionality

### Unit Tests

- [ ] > 80% coverage for security functions
- [ ] All new tests pass
- [ ] No regressions in existing tests

### Integration Tests

- [ ] WebSocket communication fully tested
- [ ] Auth flow fully tested
- [ ] All new tests pass

---

## Rollback Plan

If Electron update causes critical issues:

1. Revert `package.json` changes
2. Run `npm install` to restore old version
3. Document specific issues for future fix

If tests cause CI/CD issues:

1. Temporarily skip failing tests
2. Fix issues iteratively
3. Re-enable tests once stable

---

## Timeline Estimate

| Task              | Effort         | Priority |
| ----------------- | -------------- | -------- |
| Electron Update   | 2-3 hours      | High     |
| Unit Tests        | 3-4 hours      | Medium   |
| Integration Tests | 3-4 hours      | Medium   |
| **Total**         | **8-11 hours** | -        |

---

## Notes

- Run `npm run lint` after each phase
- Update `AGENT-JOURNAL.md` with progress
- Commit after each successful phase
- Test on Windows (primary target platform)
