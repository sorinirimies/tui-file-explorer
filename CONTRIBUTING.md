# Contributing to tui-file-explorer

Thank you for your interest in contributing! This document explains how to get
started and what conventions this project follows.

## Development Setup

```bash
# Clone
git clone https://github.com/sorinirimies/tui-file-explorer.git
cd tui-file-explorer

# Install tooling (just + git-cliff)
just install-tools

# Verify everything works
just check-all
```

## Workflow

```bash
# Format
just fmt

# Lint
just clippy

# Test
just test

# All three together
just check-all
```

## Committing

This project uses [Conventional Commits](https://www.conventionalcommits.org/)
so that `git-cliff` can generate the changelog automatically.

| Prefix | When to use |
|---|---|
| `feat:` | New capability |
| `fix:` | Bug fix |
| `doc:` | Documentation only |
| `refactor:` | Code restructure, no behaviour change |
| `perf:` | Performance improvement |
| `chore:` | Tooling, CI, version bumps |

Example: `git commit -m "feat: add sort-order toggle key"`

## Releasing (maintainers only)

```bash
# Bump version, run all checks, commit, tag, push, and trigger CI/publish:
just release 0.2.0

# Or to push to both GitHub and Gitea:
just release-all 0.2.0
```

The GitHub Actions `release.yml` workflow runs on every `v*` tag push.
It tests, builds the release, creates a GitHub Release with changelog notes,
and publishes to crates.io automatically when `CRATES_IO_TOKEN` is set in
the repository secrets.

## Code Style

- `rustfmt` with `max_width = 100` (see `rustfmt.toml`)
- No `#[allow(clippy::*)]` without a comment explaining why
- Public items must have doc-comments
- New logic must come with tests
