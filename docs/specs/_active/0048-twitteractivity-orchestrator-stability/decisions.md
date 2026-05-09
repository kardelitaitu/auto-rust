## Decisions

| Option | Pros | Cons | Decision |
|---|---|---|---|
| Keep `twitteractivity.rs` thin and test the contract | Small risk, easy to maintain, matches current architecture | Requires strong helper coverage | Chosen |
| Move more engagement logic back into `twitteractivity.rs` | Fewer indirections in one file | Harder to test, larger regression surface, less scalable | Rejected |

### Rationale

- The current task file already works best as an orchestrator.
- Reliability improves more by pinning the contract than by expanding the file.
- Easy-to-use maintenance comes from stable boundaries and targeted tests.
