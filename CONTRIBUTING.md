# Contributing to gt

Thanks for your interest in `gt`. This document covers everything you need to know to contribute cleanly: how to set up your environment, how branching works, how to write commits and PRs, and the house rules for the repo.

---

## Table of Contents

- [Before You Start](#before-you-start)
- [Development Setup](#development-setup)
- [Branching](#branching)
- [Commits](#commits)
- [Pull Requests](#pull-requests)
- [Code Style](#code-style)
- [Testing](#testing)
- [Documentation](#documentation)
- [House Rules](#house-rules)
- [Reporting Bugs](#reporting-bugs)
- [Proposing Features](#proposing-features)

---

## Before You Start

1. Open an issue first for anything non-trivial. A quick discussion saves everyone time when a PR gets too big or heads in the wrong direction.
2. Search existing issues before filing a new one. If you find a related issue, comment there instead of opening a duplicate.
3. Keep changes focused. One PR, one concern.

---

## Development Setup

### Prerequisites

- Rust 1.75.0 or newer
- Git 2.30 or newer
- Linux, macOS, or Windows (all three are supported targets)

### Clone and build

```bash
git clone git@github.com:mattmccarty/gt.git
cd gt
cargo build
./target/debug/gt --version
```

### Run tests

```bash
cargo test
```

See [docs/008-development.md](docs/008-development.md) for the full developer guide, including integration test layout and manual testing workflows.

---

## Branching

Branching, merging, and release conventions are documented in full in [docs/013-branching.md](docs/013-branching.md). The short version:

- All work happens on topic branches off `main`: `feat/*`, `fix/*`, `chore/*`, `docs/*`.
- `main` is always releasable. No direct commits.
- Merges to `main` are squashed.
- Releases are tagged `vMAJOR.MINOR.PATCH` from `main`.

---

## Commits

Full detail in [docs/013-branching.md](docs/013-branching.md#commit-messages). The essentials:

- Subject line in imperative mood, under 72 characters, capitalized, no trailing period.
- Body (optional) explains *why*, not *what*. Wrap at 72.
- Reference issues in trailers or the body: `Fixes #42`.
- **No AI or agent attribution.** No `Co-Authored-By: Claude …`, no "Generated with …" footers. Commits must read as human-authored.

Example:

```
Add passthrough for unknown git config subcommands

Unknown subcommands under `gt config` now forward to `git config`,
preserving exit codes and stderr. Explicit gt subcommands continue to
be handled internally.

Fixes #12
```

---

## Pull Requests

### Before opening a PR

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` is clean
- [ ] `cargo fmt --check` passes
- [ ] Docs updated if behavior changed
- [ ] `CHANGELOG.md` entry added under `## [Unreleased]` for user-visible changes (see [docs/014-releases.md](docs/014-releases.md#changelog-workflow))

### PR description

```markdown
## Summary
<1–3 bullets: what this change does and why>

## Changes
<bulleted list of concrete changes>

## Testing
<what you ran, what passed, how to reproduce>

## Related
Fixes #123
```

### Size

PRs over ~400 lines of non-generated diff are hard to review. Split large work into a sequence of dependent PRs — link them in each description so the reviewer can follow the chain.

### Review

Expect at least one round of review. Address feedback by pushing additional commits — do not force-push or rewrite history during review. The squash happens on merge, not during review.

---

## Code Style

### Rust

- Follow the default `rustfmt` configuration. Run `cargo fmt` before committing.
- Keep `cargo clippy -- -D warnings` clean. Address lints rather than suppressing them unless there is a good reason, documented in a comment.
- Prefer `?` for error propagation. Use the error types in `src/error.rs` rather than ad-hoc `String` errors.
- Return `Result<Output>` from command functions. `Output` is the shared formatter in `src/cli/output.rs`.
- Accept a `&Context` in command functions for logging, dry-run support, and config access.

### DRY and reuse

Before adding new utility code, check whether something similar already exists:

- Path handling: `src/core/path.rs`
- Validation: `src/util/`
- Config I/O: `src/io/toml_config.rs`
- SSH/git config I/O: `src/io/`
- Strategy implementations: `src/strategy/`

Three near-duplicate lines is fine. A fourth is a signal to extract. Don't invent an abstraction until you have at least three concrete callers.

### Comments

Write comments only when the *why* is non-obvious: a hidden invariant, a workaround for a specific bug, a decision that would surprise a reader. Do not write comments that restate what the code does — well-named identifiers cover that.

Do not reference the current task, PR number, or agent that wrote the code. If context belongs anywhere, it belongs in the PR description or commit message.

---

## Testing

### What to test

- Unit tests for pure logic: parsing, validation, strategy decisions.
- Integration tests in `tests/` for full workflows: adding an identity with multiple strategies, deleting a specific strategy variant, migrating from legacy config.
- Manual verification for cross-platform behavior that isn't easily automated (SSH agent, platform path resolution).

### What not to mock

Do not mock the filesystem, SSH config, or git config in integration tests. The value of integration tests is catching real interactions; mocks hide them. Use temp directories (`tempfile` crate) instead.

### Running

```bash
cargo test                    # all tests
cargo test test_name          # specific test
cargo test -- --nocapture     # show stdout/stderr
RUST_LOG=debug cargo test     # verbose logging
```

---

## Documentation

### File naming

Docs in `docs/` follow the `00X-topic.md` numbering convention. If you add a new doc, take the next unused number and update `docs/README.md`.

### When to update docs

- User-visible CLI behavior changes → update `docs/003-cli-reference.md`.
- Config format changes → update `docs/004-configuration.md`.
- New strategy or change to existing strategy → update `docs/002-strategies.md`.
- Anything security-relevant → update `docs/005-security.md`.

### Style

- Concise and example-driven. Show commands, show config, show output.
- No marketing voice. No "simply", "just", "easily" — if it were simple, the doc wouldn't need to exist.
- Tables beat paragraphs for reference material.
- Mermaid diagrams are fine, but only when a diagram genuinely helps. Don't diagram what a short list can convey.

---

## House Rules

### No AI or agent attribution

This rule applies everywhere the project surface is public or semi-public: issue titles and bodies, PR titles and descriptions, PR review comments, commit messages, code comments, documentation.

- No `Co-Authored-By: Claude …` trailers.
- No "Generated with Claude Code" / "AI-assisted" footers.
- No "I used an agent to …" in PR descriptions.
- No AI/LLM/Claude/agent references in code comments or docs.

Everything the project shows the world should read as human-authored.

### No secrets in the repo

Never commit:
- `.env` files, `.pem`, `.key`, or any key material
- Tokens, API keys, or credentials in any form
- Real user emails or names in test fixtures (use `test@example.com`)

If you accidentally commit a secret, rotate it immediately and file an issue. Do not rely on git history rewriting to contain the leak.

### No destructive git operations without confirmation

- Do not force-push to `main` (it's branch-protected; this is defensive regardless).
- Do not `git reset --hard` on a shared branch.
- Do not delete branches other people may be working on.

---

## Reporting Bugs

File an issue with:

- **What you ran** — exact command and any relevant config state.
- **What you expected** — the behavior you thought you'd see.
- **What you got** — actual output, error messages, exit codes.
- **Environment** — OS, `gt --version`, `git --version`, shell.
- **Minimal reproduction** — the smallest sequence of commands that shows the bug.

Use the `type:bug` label.

---

## Proposing Features

For anything larger than a small enhancement, open an issue before a PR. Describe:

- **Problem** — what's painful or missing today.
- **Proposed solution** — the rough shape of what you want to add.
- **Alternatives** — what else you considered and why this is better.
- **Impact** — breaking change? config migration needed? cross-platform implications?

Use the `type:feature` label. A maintainer will respond with feedback or a green light before you invest in a PR.

---

## See Also

- [docs/013-branching.md](docs/013-branching.md) — full branching, merging, and release conventions
- [docs/014-releases.md](docs/014-releases.md) — release process, CHANGELOG workflow, release-notes template
- [CHANGELOG.md](CHANGELOG.md) — project changelog
- [docs/008-development.md](docs/008-development.md) — developer setup and testing guide
- [docs/001-architecture.md](docs/001-architecture.md) — system architecture
- [docs/README.md](docs/README.md) — documentation index
