# Decisions

## Decision Log

- **Scope scoping to Modals**: Decided to prioritize anything inside `[role="dialog"]` if it contains an article. This is because Twitter's modal implementation is consistent in using the dialog role for thread overlays.
- **Combined Spec**: Decided to combine context validation and selector priority into one spec because they both address the same root problem (modal invisibility to automation).

## Open Questions

- Should we check for `aria-modal="true"` specifically? (Currently sticking to `role="dialog"` as it's more common).
- How to handle multiple nested dialogs? (Twitter usually only has one main thread modal).
