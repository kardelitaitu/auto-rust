# Architecture Diagrams

Visual diagrams of Auto-AI system architecture and data flows.

---

## System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Auto-AI Framework                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐                │
│  │   main.js    │     │ agent-main.js│     │   api/       │                │
│  │  (CLI Entry) │     │ (Game Agent) │     │  index.js    │                │
│  └──────┬───────┘     └──────┬───────┘     └──────┬───────┘                │
│         │                    │                    │                         │
│         └────────────────────┼────────────────────┘                         │
│                              │                                              │
│                     ┌────────▼────────┐                                     │
│                     │  Orchestrator   │                                     │
│                     │  (Task Queue)   │                                     │
│                     └────────┬────────┘                                     │
│                              │                                              │
│         ┌────────────────────┼────────────────────┐                         │
│         │                    │                    │                         │
│  ┌──────▼───────┐   ┌───────▼────────┐  ┌───────▼────────┐                 │
│  │Session Manager│   │  Discovery     │  │  Agent System  │                 │
│  │  (Lifecycle)  │   │  (CDP Ports)   │  │  (LLM + AI)    │                 │
│  └──────┬───────┘   └────────────────┘  └────────────────┘                 │
│         │                                                                    │
│    ┌────▼─────┐  ┌────▼─────┐  ┌────▼─────┐                                │
│    │Browser 1 │  │Browser 2 │  │Browser N │                                │
│    │  (CDP)   │  │  (CDP)   │  │  (CDP)   │                                │
│    └──────────┘  └──────────┘  └──────────┘                                │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## API Module Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        api/index.js                              │
│                     (Unified Export)                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │   core/     │  │interactions/│  │ behaviors/  │              │
│  │  Context    │  │  Actions    │  │  Persona    │              │
│  │  Session    │  │  Cursor     │  │  Idle       │              │
│  │  Orchestr.  │  │  Navigation │  │  Attention  │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │  agent/     │  │   utils/    │  │  actions/   │              │
│  │  GameRunner │  │  GhostCursor│  │  Twitter    │              │
│  │  LLMClient  │  │  Fingerprint│  │  High-level │              │
│  │  ActionEng. │  │  Math/Timing│  │  Helpers    │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Context Isolation Flow

```
┌─────────────────────────────────────────────────────────────┐
│                  AsyncLocalStorage Context                   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────────┐
        │     api.withPage(page, async () => {  │
        │         // Context is active here     │
        │         await api.click('.btn');      │
        │     });                               │
        └───────────────────────────────────────┘
                            │
            ┌───────────────┴───────────────┐
            │                               │
            ▼                               ▼
    ┌───────────────┐               ┌───────────────┐
    │  Session A    │               │  Session B    │
    │  (Page 1)     │               │  (Page 2)     │
    │  - Cursor     │               │  - Cursor     │
    │  - Persona    │               │  - Persona    │
    │  - State      │               │  - State      │
    └───────────────┘               └───────────────┘
            │                               │
            └───────────────┬───────────────┘
                            │
                            ▼
                    (No cross-contamination)
```

---

## Agent Perception-Action Loop

```
┌─────────────────────────────────────────────────────────────────────┐
│                      GameRunner.run(goal)                            │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
        ┌─────────────────────────────────────────┐
        │   1. PERCEIVE (agent.see())             │
        │   - Capture screenshot                   │
        │   - Extract AXTree (DOM structure)       │
        │   - Compress context for LLM             │
        └────────────────────┬────────────────────┘
                             │
                             ▼
        ┌─────────────────────────────────────────┐
        │   2. REASON (llmClient)                 │
        │   - Send to LLM with system prompt       │
        │   - Parse JSON response                  │
        │   - Validate action                      │
        └────────────────────┬────────────────────┘
                             │
                             ▼
        ┌─────────────────────────────────────────┐
        │   3. ACT (actionEngine)                 │
        │   - Execute with GhostCursor             │
        │   - Human-like timing                    │
        │   - Retry on failure                     │
        └────────────────────┬────────────────────┘
                             │
                             ▼
        ┌─────────────────────────────────────────┐
        │   4. VERIFY (visualDiff)                │
        │   - Compare before/after state           │
        │   - Check DOM changes                    │
        │   - Rollback if failed                   │
        └────────────────────┬────────────────────┘
                             │
                             ▼
                    (Repeat until goal achieved)
```

---

## LLM Request Routing

```
┌─────────────────────────────────────────────────────────────┐
│                    IntentClassifier                         │
│              (Analyze task complexity)                       │
└─────────────────────┬───────────────────────────────────────┘
                      │
          ┌───────────┴───────────┐
          │                       │
          ▼                       ▼
   ┌──────────────┐       ┌──────────────┐
   │ Simple Task  │       │ Complex Task │
   │ (confidence  │       │ (confidence  │
   │  >0.7)       │       │  <0.7)       │
   └──────┬───────┘       └──────┬───────┘
          │                     │
          ▼                     ▼
   ┌──────────────┐       ┌──────────────┐
   │ LocalClient  │       │ CloudClient  │
   │  (Ollama)    │       │ (OpenRouter) │
   │              │       │              │
   │ Fast & Free  │       │ Powerful     │
   │ Simple tasks │       │ Complex tasks│
   └──────┬───────┘       └──────┬───────┘
          │                     │
          ▼                     ▼
   ┌──────────────┐       ┌──────────────┐
   │ VisionInterp │       │AgentConnector│
   │ parseResponse│       │  reasoning   │
   └──────────────┘       └──────────────┘
```

---

## Browser Discovery Flow

```
┌─────────────────────────────────────────────────────────────┐
│                  Orchestrator.addTask()                      │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
        ┌─────────────────────────────────────┐
        │   discovery.js                      │
        │   - Scan configured ports            │
        │   - Query CDP endpoints              │
        │   - Return browser list              │
        └─────────────────────┬───────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
      ┌───────────┐   ┌───────────┐   ┌───────────┐
      │ixBrowser  │   │  Chrome   │   │  Dolphin  │
      │Port 53200 │   │Port 9222  │   │Port 5050  │
      └─────┬─────┘   └─────┬─────┘   └─────┬─────┘
            │               │               │
            └───────────────┼───────────────┘
                            │
                            ▼
                ┌───────────────────────┐
                │  sessionManager       │
                │  - Create worker      │
                │  - Isolate context    │
                │  - Execute task       │
                └───────────────────────┘
```

---

## Task Execution Flow

```
┌─────────────────────────────────────────────────────────────┐
│              main.js: node main.js task=url                  │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
        ┌─────────────────────────────────────┐
        │   Parse task name & payload         │
        └─────────────────────┬───────────────┘
                              │
                              ▼
        ┌─────────────────────────────────────┐
        │   Orchestrator.processTasks()       │
        │   - Queue task                       │
        │   - Dispatch to workers              │
        └─────────────────────┬───────────────┘
                              │
                              ▼
        ┌─────────────────────────────────────┐
        │   sessionManager.createWorker()     │
        │   - New browser session              │
        │   - Isolated context                 │
        └─────────────────────┬───────────────┘
                              │
                              ▼
        ┌─────────────────────────────────────┐
        │   Dynamic import: tasks/{name}.js   │
        └─────────────────────┬───────────────┘
                              │
                              ▼
        ┌─────────────────────────────────────┐
        │   Execute: task(page, payload)      │
        │   - api.withPage()                   │
        │   - api.init()                       │
        │   - Task logic                       │
        └─────────────────────┬───────────────┘
                              │
                              ▼
        ┌─────────────────────────────────────┐
        │   Cleanup & Report                  │
        └─────────────────────────────────────┘
```

---

## Humanization Stack

```
┌─────────────────────────────────────────────────────────────┐
│                    Persona System                            │
│          (16 unique interaction profiles)                    │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
        ┌─────────────────────────────────────┐
        │   PID Muscle Model                  │
        │   - Per-session acceleration        │
        │   - Unique movement signature        │
        └─────────────────────┬───────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
      ┌───────────┐   ┌───────────┐   ┌───────────┐
      │  Mouse    │   │ Keyboard  │   │  Scroll   │
      │  Physics  │   │  Dynamics │   │  Patterns │
      │           │   │           │   │           │
      │ Fitts's   │   │ Variable  │   │ Natural   │
      │ Law       │   │ Delays    │   │ Rhythm    │
      │ Bezier    │   │ Punctuation│  │ Reading   │
      │ Curves    │   │ Pauses    │   │ Flow      │
      └───────────┘   └───────────┘   └───────────┘
                              │
                              ▼
        ┌─────────────────────────────────────┐
        │   Idle Behavior                     │
        │   - Micro-fidgeting                 │
        │   - Periodic movement               │
        │   - Presence simulation              │
        └─────────────────────────────────────┘
```

---

## Related Documentation

- [`.agents/API-ARCHITECTURE.md`](../.agents/API-ARCHITECTURE.md) - Detailed API patterns
- [`.agents/PROJECT-STRUCTURE.md`](../.agents/PROJECT-STRUCTURE.md) - Directory structure
- [`docs/api.md`](api.md) - API reference
- [`docs/architecture.md`](architecture.md) - Architecture overview
