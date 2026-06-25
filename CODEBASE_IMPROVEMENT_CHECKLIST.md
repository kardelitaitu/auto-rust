# Codebase Improvement & Scaling Checklist

This document details the checklist and architectural steps required to improve the `auto-rust` codebase, specifically targeting stability, anti-ban protection, and resource management when scaling the system to run up to 500+ browser sessions concurrently.

---

## 🚀 1. Concurrency & Shared Resource Lock Improvement
With 500+ concurrent browser sessions, shared files and states are critical points of contention.

- [ ] **Database Segmentation / Migration:**
  - Replace the monolithic JSON database (`twitter_activity_v2.json`) with an SQLite database or segment database files per browser profile to prevent file-locking contention.
  - Implement a connection pool (e.g., `sqlx` with SQLite) if moving to a centralized DB.
- [ ] **Configurable Staggering:**
  - Expose the task staggering delay (currently in `execute_task_on_session`) as a `.env` variable (`TASK_STAGGER_DELAY_MS`) to allow fine-tuning under different network environments.
- [ ] **Non-Blocking Write Queues:**
  - Instead of direct filesystem writing with raw locking in the tasks, introduce an async channel-based writer thread that processes persistent state updates sequentially.

---

## 🤖 2. LLM Scaling & Load Balancing
500 concurrent sessions making API calls to a single local Ollama server will cause timeouts and memory exhaustion.

- [ ] **Centralized LLM Router & Pool:**
  - Implement a centralized Ollama/LLM router that distributes inference tasks across multiple local GPUs/servers.
- [ ] **Local-to-Cloud Fallback Chain:**
  - Configure a cascading fallback chain: if local Ollama queues are full or timeout, fallback automatically to cloud endpoints (OpenRouter/Nvidia API) to keep browser tasks moving.
- [x] **Inference Rate Limiting:**
  - [x] Implement token bucket rate limiting on the LLM client wrapper to stagger requests and avoid throttling.

---

## 🛡️ 3. Anti-Detection & Account Protection (Anti-Ban)
evading platform detection algorithms requires high entropy and consistent digital identities.

- [ ] **Residential Proxy Mapping:**
  - Map each browser session to a dedicated, clean residential proxy.
  - Implement proxy health checks before starting any automation tasks; skip execution if the proxy IP is flagged or down.
- [x] **Linguistic Entropy (LLM Output Variation):**
  - [x] Expose and pass LLM temperature configurations dynamically from environment variables (`LLM_TEMPERATURE`, `OLLAMA_TEMPERATURE`, `OPENROUTER_TEMPERATURE`, `NVIDIA_TEMPERATURE`) to prevent hardcoded 0.7 values.
  - [x] Implement inline `<think>...</think>` tag stripping and structured `reasoning_content` extraction to prevent raw reasoning monologues from leaking into posts.
  - [x] Implement `presence_penalty` / `frequency_penalty` controls to prevent repeating words across multiple accounts.
  - [x] Implement **System Prompt Rotation**: Maintain a pool of different personas (e.g. casual, professional, concise, expressive) and assign them deterministically to different browser profiles based on their session ID.
- [ ] **Human Behavior Randomization:**
  - Randomize typing speeds (words per minute) and introduce realistic typos with automatic corrections.
  - Implement human-like micro-hesitations (e.g., hovering over a button for 0.5s–1.5s before clicking it).
  - Implement Bezier curve mouse movements with randomized acceleration/deceleration profiles.

---

## 🖥️ 4. Resource & Headless Process Management
Running 500 Chrome/Brave sessions concurrently consumes massive memory and CPU.

- [ ] **Chrome Arguments Optimization:**
  - Launch browsers with resource-saving flags (e.g., `--disable-dev-shm-usage`, `--js-flags="--max-opt-helper-threads=1"`, `--disable-gpu`, `--blink-settings=imagesEnabled=false` to skip loading images).
- [ ] **Orphaned Process Cleanup:**
  - Write a background watchdog process (or integrate inside orchestrator `health.rs`) that periodically detects and kills orphaned or zombie Chrome/driver processes.
- [ ] **Memory Monitoring & Throttling:**
  - Dynamically adjust `MAX_GLOBAL_CONCURRENCY` based on the host system's active CPU and memory usage to prevent system freezes.

---

## 📊 5. Observability & Centralized Monitoring
Monitoring failure rates across hundreds of threads is impossible via console logs alone.

- [ ] **Structured Log Aggregation:**
  - Configure structured logging (`tracing` with JSON formatting) and push logs to a centralized collector (e.g., Loki or Elasticsearch).
- [ ] **Telemetry Dashboard:**
  - Export success/failure metrics, rate-limiting errors (HTTP 429), and run durations to Prometheus/Grafana.
- [ ] **Automated Alerting:**
  - Set up alerts for high failure rates (e.g. if >10% of sessions encounter scroll shifts or layout shifts within a short window).

---

## 🛠️ 6. Developer Hygiene & Test Suite Integrity
Ensuring the codebase is clean, accessible, and fast to test for developers.

- [x] **Default Test Suite Build Resolution:**
  - Gate the pipeline integration test file (`tests/bacon_pipeline_integration.rs`) with `#![cfg(feature = "bacon")]` so that standard `cargo test` runs without feature flags build cleanly without compilation errors.
