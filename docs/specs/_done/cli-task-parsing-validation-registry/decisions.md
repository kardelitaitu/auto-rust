# Decisions

- Keep task-group parsing syntax unchanged.
- Move parser helpers behind a dedicated CLI parser module.
- Use the registry as the source of truth for `--help-task` output.
- Keep startup-mode dispatch in `main.rs`.
- Prefer deterministic task help output over verbose dynamic text.
