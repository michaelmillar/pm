# pm — Agent Instructions

## After Any Code Change

Always reinstall the binary so the live `pm` command reflects the changes:

```bash
cargo install --path .
```

This is required at the end of every implementation session. Add it as the final step before finishing a development branch.

## Binary Location

Installed to `~/.cargo/bin/pm` via `cargo install`. Symlinked/on PATH at `~/.local/bin/pm`.

## Test Command

```bash
cargo test -p pm
```
