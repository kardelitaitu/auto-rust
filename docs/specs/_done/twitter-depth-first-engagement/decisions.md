# Decisions

## Decision Log

- **Likes Only for Replies**: Decided to restrict reply engagement to "Like" only. Automated replies to replies are much higher risk for bot detection and negative community sentiment if the template doesn't fit the context perfectly.
- **Top-Level Only**: We will only scan the top-level replies visible on the initial dive (plus a small scroll). Navigating deeper into nested threads is technically complex and increases the risk of the bot getting lost.

## Open Questions

- Should we make `max_replies_per_thread` a user-configurable setting in `default.toml`? (Starting with a hardcoded range of 1-3 for now).
