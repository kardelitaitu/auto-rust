## Validation

- Helper tests continue to cover normalization and filter matching.
- Regression tests cover `discover_with_filters` with empty discovery and no filters.
- Regression tests cover `discover_with_filters` with empty discovery and active filters.
- Regression tests cover pool-boundary normalization, such as `Brave_Browser` matching `brave-browser`.
- Port-range parsing tests stayed unchanged.
- `spec-lint.ps1` passed before handoff.
