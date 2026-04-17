# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Entry categories: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`.

See [docs/014-releases.md](docs/014-releases.md) for the full release process,
including how to move `Unreleased` into a version section at release time.

## [Unreleased]

## [0.2.0] - 2026-04-17

### Added

- `CHANGELOG.md` with a Keep a Changelog format and reconstructed `[0.1.0]` baseline.
- `docs/014-releases.md` covering the release-notes template, step-by-step release checklist, version-bump procedure, and the decision to adopt `cargo-release`.
- `.github/pull_request_template.md` to enforce the CHANGELOG and quality checklist on every PR.
- `gt config` now forwards unknown subcommands and flag-style invocations to `git config`, making gt a drop-in superset. Examples: `gt config get user.email`, `gt config set user.email x`, `gt config list`, `gt config edit`, `gt config --global user.name`, `gt config unset remote.origin.url`. Exit codes, stdout, and stderr from `git config` are preserved. gt-native under `gt config`: bare `gt config` shows the gt configuration summary, `gt config validate` validates gt's config, and `gt config id *` manages identities. No gt-native command shadows a `git config` subcommand.

### Changed

- **Breaking:** all identity commands moved from `gt id *` to `gt config id *`. `gt id add`, `list`, `use`, `migrate`, `key`, `status`, `delete`, `update`, `import` are now `gt config id add`, `list`, `use`, `migrate`, `key`, `status`, `delete`, `update`, `import`. Migration: prefix any invocation of `gt id <sub>` with `gt config`.
- **Breaking:** `gt fix id` moved to `gt config id fix`.
- `CONTRIBUTING.md` and `docs/013-branching.md` now link to the CHANGELOG convention and the release-notes template rather than describing them inline.
- `docs/README.md` index extended to include `014-releases.md`.
- `README.md`, `CLAUDE.md`, and the docs under `docs/` (`001`, `003`, `005`, `007`, `008`, `009`, `010`) updated to use the new `gt config id *` command paths. Pre-existing references to subcommands that never existed (`gt id init`, `gt id scan`, `gt id switch`, `gt id clone`, `gt id config`) are unchanged — out of scope here, tracked separately.

### Removed

- **Breaking:** top-level `gt id` command tree. No deprecation alias; invocations of `gt id *` now error with `unrecognized subcommand`.
- **Breaking:** top-level `gt fix` command tree. No deprecation alias; invocations of `gt fix id` now error with `unrecognized subcommand`.

### Fixed

- Passthrough commands (`gt status`, `gt add`, `gt pull`, `gt fetch`, `gt checkout`, `gt branch`, `gt merge`, `gt rebase`, `gt diff`, `gt log`, `gt stash`, `gt tag`, `gt remote`, `gt reset`, `gt commit`, `gt push`) now render git's colored output correctly. Previously gt captured stdout/stderr into buffers and replayed them, which made git disable colors (its `isatty()` check saw a pipe). Passthroughs now inherit stdio, so git sees a TTY when one is present. Exit codes from git are also propagated verbatim — `gt status` outside a repo now exits 128 (matching `git status`), not 1.

## [0.1.0] - 2026-03-21

Initial public baseline. Never tagged on GitHub; reconstructed here from the state of the repository at the initial commit so subsequent releases have a documented starting point.

### Added

#### Identity management

- Multi-strategy identity model: a single identity can combine SSH, conditional include, and URL rewrite strategies at the same time.
- SSH hostname alias strategy — writes `gt-<name>.<host>` host entries to `~/.ssh/config`.
- Git conditional include strategy — directory-scoped via `includeIf "gitdir:<path>/"` with automatic trailing-slash normalization.
- URL rewrite strategy — scope-based, transforms `git@<host>:<scope>/...` and `https://<host>/<scope>/...` URLs.
- Multiple variants per strategy type on a single identity (e.g. several conditional directories or URL scopes).
- Automatic migration from the legacy single-strategy config format on config read.

#### CLI

- `gt id` — show current identity.
- `gt id add`, `import`, `list`, `use`, `migrate`, `status`, `delete`, `update` — identity lifecycle.
- `gt id key` — SSH key subcommands: `generate`, `list`, `add`, `remove`, `activate`, `show`, `test`. Supports Ed25519 (default), RSA, and ECDSA.
- `gt config list`, `edit`, `validate` — configuration inspection.
- `gt config id`, `gt config id default [name]` — identity configuration summary and default selection.
- `gt clone <url>` — clone with automatic identity detection and URL transformation.
- `gt fix id` — repair repository URLs and local git config.
- `gt commit` — passthrough to `git commit` with shorthand date syntax (`-30s`, `15m`, `-1h`, `2d`, `-1w`, `now`) and chronological-order guardrails against HEAD's author date.
- `gt push` — passthrough to `git push` that detects future-dated commits and installs a pre-push hook to block premature pushes until the latest scheduled time. `--force`, `--cancel`, `--list` supported.
- `gt reset` — passthrough to `git reset` with special subcommands `commits` (reset to initial) and `staged` (unstage all).
- `gt status`, `add`, `pull`, `fetch`, `checkout`, `branch`, `merge`, `rebase`, `diff`, `log`, `stash`, `tag`, `remote` — passthroughs to the matching `git` subcommand.
- `gt commit list` — list commits sorted by date, annotated with scheduled-push info.

#### Configuration and output

- TOML config at `~/.config/gt/config.toml` (Unix) or `%APPDATA%\gt\config.toml` (Windows).
- Conditional include fragments written to `~/.gitconfig.d/<identity>`.
- Global flags: `--verbose`, `--quiet`, `--config`, `--output`, `--no-color`, `--dry-run`, `--force`, `--auto`, `--all`.
- Output formats: `terminal` (default), `json`, `csv`.

#### Safety

- Automatic timestamped backups of `~/.ssh/config` before every modification, with hash-based corruption detection.
- Cross-platform path handling for Linux, macOS, and Windows.
- Existing pre-push hooks are preserved and chained when the scheduled-push hook is installed.

### Known limitations

- `gt push` scheduling is a *lock*, not an *executor*: the pre-push hook prevents premature pushes but nothing fires the push when the scheduled time arrives. Tracked in [#5](https://github.com/mattmccarty/gt/issues/5).
- `gt id *` and `gt fix id` are planned to move under `gt config id *` as a breaking change in the next release. Tracked in [#4](https://github.com/mattmccarty/gt/issues/4).

[Unreleased]: https://github.com/mattmccarty/gt/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/mattmccarty/gt/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/mattmccarty/gt/releases/tag/v0.1.0
