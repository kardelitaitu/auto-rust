last audited 2026-06-27 by antigravity

# Introduce async channel-based write queue for Twitter persistence

## Baseline
Currently, `TwitterPersistenceState` in `src/utils/twitter/twitteractivity_persistence.rs` relies on physical filesystem locks (`.json.lock` files) and synchronous reads/writes (`std::fs::read_to_string`, `std::fs::write`, `std::fs::rename`) on the active async task threads to coordinate updates. This blocks the Tokio scheduler workers and leads to potential timeouts under concurrency.

## Implementation Steps
1. Define a global/static async write queue in `src/utils/twitter/twitteractivity_persistence.rs`.
   - We will use `tokio::sync::mpsc` for sending commands.
   - Define a command enum:
     ```rust
     pub enum PersistenceCommand {
         Update {
             profile_name: String,
             record_session_end: bool,
             actions: Vec<String>,
             // A oneshot channel to notify the caller when the write has been flushed to disk
             reply_tx: Option<tokio::sync::oneshot::Sender<Result<(), String>>>,
         }
     }
     ```
   - Provide a lazy initialization function/global using `std::sync::OnceLock` or `once_cell` to retrieve the sender.
   - Lazily spawn the background writer loop on the first command, which:
     - Continuously reads from the receiver.
     - Performs the state load, mutation, and save operations using `spawn_blocking` to avoid blocking Tokio worker threads.
     - Notifies the caller (if a reply channel is provided).
2. Modify `update_async` or introduce a new queue-backed `update_queue_async` method:
   - Instead of checking physical locks and writing directly, it sends an `Update` command to the channel.
   - It awaits the oneshot reply channel to ensure the write has successfully completed before returning.
3. Clean up the physical `.lock` file code from `twitteractivity_persistence.rs` as it is no longer required when writes are serialized by the single consumer.

## API Changes
No public breaking API changes. Internal methods in `twitteractivity_persistence.rs` will utilize the new async queue.

## Validation
- Add unit tests verifying that commands queued to different profile names are processed correctly.
- Run `cargo test utils::twitter::twitteractivity_persistence` to verify.

## Design Decisions and Risks
- **Oneshot confirmation**: To maintain correctness and ensure that subsequent tasks read the updated state, `update_async` will await confirmation from the writer thread before returning. This guarantees sequential consistency while still utilizing non-blocking asynchronous channels and offloading file I/O to a background pool.
- **Confidence Level**: High.
