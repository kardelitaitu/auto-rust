# Quality Rules

- Keep the move mechanical and small.
- Do not reimplement production logic inside benchmark files.
- Keep benchmark helpers reusable only when two or more benchmarks need them.
- Do not add benchmark code to `lib.rs`.
- Keep feature-gated code explicit in Cargo and in the spec.
- Do not delete the root `benches/` folder until the moved targets pass validation.

