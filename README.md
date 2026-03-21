<p align="centre">
  <img src="web/public/favicon.svg" alt="pm logo" width="64" height="64">
</p>

# pm

A priority-driven project manager for solo developers and small teams. Tracks your projects, scores them on impact, monetisation, and defensibility, runs competitive research via configurable LLM providers, and tells you what to work on next.

## Why

Solo developers juggle too many project ideas. Most tools track tasks within a single project but nothing helps you decide *which* project deserves your time right now. `pm` solves that with a scoring algorithm that weighs impact, monetisation potential, defensibility, readiness, and staleness to surface your highest-value work.

## Features

- **Priority scoring** across impact, monetisation, defensibility, and readiness
- **Competitive research** via configurable LLM providers (with automatic fallback)
- **Definition of Done** with automated verification and human sign-off
- **Roadmap tracking** with weighted phases and readiness percentage
- **Milestone tracking** parsed from Markdown checklists
- **Project lifecycle** from inbox to active to shipped/killed, with parking and soft-delete
- **Duplicate detection** using weighted name similarity
- **Standards scanning** across linked codebases (README, licence, gitleaks, CI, etc.)
- **Web dashboard** (Svelte 5 + Axum) with sortable tables, project detail views, and deterministic project icons
- **Pivot suggestions** when a project is competed out

## Install

```bash
cargo install --path .
```

Requires Rust 2024 edition (1.85+).

## Quick start

```bash
pm add "My Project"           # add to inbox
pm activate 1                 # move to active
pm link 1 ./my-project        # link to a codebase
pm status                     # see all active projects ranked
pm next                       # what should I work on?
pm research 1                 # run competitive research
pm throne                     # top 3 priority projects
```

## Web dashboard

```bash
cd web && npm install && npm run build && cd ..
pm serve
# open http://localhost:3000
```

## LLM provider configuration

Research, verification, and pivot commands use external LLM CLIs. Configure your preferred providers in `~/.config/pm/providers.conf`:

```ini
# Providers are tried in order (falls back to next on failure)
provider_1=claude
model_1=sonnet
provider_2=codex
```

Supported provider styles out of the box: `claude` (with `--model` and `--allowedTools` flags) and `codex` (with `exec --sandbox` flags). Any other command name is invoked as a generic CLI that receives the prompt as its sole argument.

## Environment variables

| Variable | Purpose | Default |
|---|---|---|
| `PM_DATA_DIR` | SQLite database location | `~/.local/share/pm/` |
| `PM_ROOT` | Root directory to scan for projects | `~/projects` |
| `PM_PROVIDERS_CONFIG` | Path to LLM provider config | `~/.config/pm/providers.conf` |
| `PM_PROVIDER_TIMEOUT_SECS` | Timeout for fallback providers | `45` |
| `PM_RESEARCH_CONFIG` | Research scheduling config | `~/.config/pm/research.conf` |
| `PM_STANDARDS_CONFIG` | Standards checks config | `~/.config/pm/standards.yml` |

## Testing

```bash
cargo test
```

149 tests covering the CLI, store, research fallback, DOD parsing, roadmap scoring, standards scanning, and duplicate detection.

## Licence

Apache 2.0
