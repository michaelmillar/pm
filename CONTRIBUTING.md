# Contributing to pm

## Building

```
cargo build --release
```

Requires Rust 2024 edition (1.85+).

For the web dashboard:

```
cd web && npm install && npm run build
```

## Testing

```
cargo test
```

190 tests cover the CLI, store, scoring, research fallback, DOD parsing, roadmap, standards, and duplicate detection.

## Style

- No comments in code. Write readable code instead.
- UK spelling in user-facing text.
- `cargo fmt` and `cargo clippy` must pass cleanly.
- `npm run build` in `web/` must produce zero errors.

## Opening an issue

Describe what you expected, what happened, and how to reproduce it. Include your Rust version (`rustc --version`) and OS.

## Pull requests

- One logical change per PR.
- Include a test if the change affects scoring, store, or CLI behaviour.
- Run `cargo test` and `cargo clippy` before pushing.
