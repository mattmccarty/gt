# Release Process

This document defines how releases are prepared, cut, and announced for `gt`. Branching, merging, and tagging mechanics live in [013-branching.md](013-branching.md). This document focuses on what happens *around* the tag: the CHANGELOG, the GitHub Release body, version bumps, and the checklist a maintainer follows.

---

## Table of Contents

- [Principles](#principles)
- [Artifacts](#artifacts)
- [CHANGELOG Workflow](#changelog-workflow)
- [Version Bump](#version-bump)
- [Release Checklist](#release-checklist)
- [Release Notes Template](#release-notes-template)
- [Tooling: cargo-release](#tooling-cargo-release)
- [Post-Release](#post-release)

---

## Principles

1. **Two artifacts, one source of truth.** The CHANGELOG is the permanent record. The GitHub Release body is a short narrative for humans. The Release body links to the CHANGELOG entry; never duplicates it.
2. **Every user-visible PR updates the CHANGELOG.** In the same PR. Not after, not separately. If a PR lands without a CHANGELOG entry and it should have had one, the next PR opens a follow-up entry citing the commit SHA.
3. **Releases are cheap.** The process should be short enough that cutting a patch release is easy. Anything ceremonial should be automated or removed.
4. **No release without a clean `main`.** `main` must build, test, and lint clean at the commit being tagged. If it doesn't, fix `main` first.

---

## Artifacts

A release produces the following, in order:

1. A commit on `main` that bumps `Cargo.toml` to the new version and moves the CHANGELOG `[Unreleased]` entries into a dated version section.
2. A signed annotated git tag `vMAJOR.MINOR.PATCH` pointing at that commit.
3. A GitHub Release attached to the tag, with a body written from the [Release Notes Template](#release-notes-template).

Optional follow-ups (separate tickets, not required for a release):

- Pre-built binaries attached to the GitHub Release.
- crates.io publish.
- Distribution-channel updates (Homebrew tap, AUR, Scoop, etc.).

---

## CHANGELOG Workflow

### File format

`CHANGELOG.md` at the repo root. [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format. Categories:

- `Added` — new features.
- `Changed` — changes to existing behavior.
- `Deprecated` — features marked for removal.
- `Removed` — features removed in this release.
- `Fixed` — bug fixes.
- `Security` — security-relevant fixes.

Every release has a `## [MAJOR.MINOR.PATCH] - YYYY-MM-DD` header. At the top of the file, an `## [Unreleased]` section accumulates entries as PRs merge.

### Per-PR workflow

In the same PR that changes user-visible behavior:

1. Open `CHANGELOG.md`.
2. Under `## [Unreleased]`, add a bullet in the right category. One bullet per user-visible change. Bullet describes the change in one sentence, written for a user, not a developer.
3. Don't reference the PR number, issue number, or author. The git log is the attribution record.

**What counts as user-visible?**

- CLI surface changes: new subcommand, new flag, renamed flag, behavior change of an existing flag.
- Config format changes (TOML keys added, removed, renamed).
- File-location changes (backup paths, SSH config path resolution).
- Error message changes that users script against.
- Performance regressions or improvements that exceed an order of magnitude.

**What does not require a CHANGELOG entry?**

- Internal refactors with no user-visible effect.
- Docs-only PRs (unless the doc describes a user-visible policy change, like a deprecation notice).
- Test infrastructure, CI config, dependency bumps with no behavior change.

### At release time

Move `## [Unreleased]` entries into a new `## [X.Y.Z] - YYYY-MM-DD` section. Leave the `## [Unreleased]` header in place with an empty body — the next PR will fill it.

Update the link references at the bottom of the file:

```markdown
[Unreleased]: https://github.com/mattmccarty/gt/compare/vX.Y.Z...HEAD
[X.Y.Z]: https://github.com/mattmccarty/gt/compare/vPREV...vX.Y.Z
```

---

## Version Bump

### Single source of truth

`Cargo.toml` `version = "X.Y.Z"` is the canonical version. Nothing else should hardcode the version. `--version` output is driven by `CARGO_PKG_VERSION` via clap, so it tracks automatically.

### Version choice

Follow [Semantic Versioning](https://semver.org/). Pre-1.0 rules applied to this project:

| Change type | Pre-1.0 | Post-1.0 |
|-------------|---------|----------|
| Breaking CLI, config format, or SSH/git config output | `MINOR` bump (0.x.0 → 0.(x+1).0) | `MAJOR` bump |
| New feature, backward-compatible | `MINOR` bump | `MINOR` bump |
| Bug fix, backward-compatible | `PATCH` bump | `PATCH` bump |

The first stable release (`1.0.0`) requires an explicit decision that the CLI and config format are frozen under normal semver.

### What to edit

- `Cargo.toml` `version = ...`
- `Cargo.lock` (`cargo build` regenerates it)
- `CHANGELOG.md` (move `[Unreleased]` → `[X.Y.Z] - YYYY-MM-DD`, update link refs at bottom)
- `docs/README.md` version footer line (if it lags)
- `CLAUDE.md` `**Version:**` footer line (if it lags)

---

## Release Checklist

Follow this checklist when cutting a release. If a step fails, stop and fix the underlying issue rather than skipping the step.

### 1. Pre-flight (on `main`)

```bash
git checkout main
git pull --ff-only
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

All four must pass. If anything fails, fix on a separate PR before continuing.

#### Waivers

In rare cases a pre-flight check fails on `main` for reasons unrelated to the release content. v0.2.0 shipped with three such waivers (later closed as #16, #17, #18); the pattern is worth codifying so future releases handle it consistently rather than improvising.

A waiver is acceptable only when **all** of the following hold:

1. The failure pre-exists the release content and is unrelated to it.
2. It is tracked in a dedicated GitHub issue.
3. The release PR carries a pre-flight status table that lists each check, its disposition (`pass` / `fail` / `waive`), and the ticket link for any waiver.
4. Shipping with the failure does not prevent users from building the tagged commit.

A waiver is **not** acceptable for anything introduced by the release content itself, or for failures that block building. In those cases, fix on a separate PR before continuing.

Example pre-flight status table (adapt the disposition column per release):

| Check | Result | Disposition |
|-------|--------|-------------|
| `cargo build` | pass | |
| `cargo test` | pass | |
| `cargo clippy -- -D warnings` | pass | |
| `cargo fmt --check` | pass | |

If a disposition is `waive`, that row cites the tracking issue. Waivers are the exception, not the default; if more than one check is waived in consecutive releases, stop and fix the underlying problem before cutting another release.

### 2. Prepare the release PR

```bash
git checkout -b release/vX.Y.Z
```

Edit:

- `Cargo.toml` — bump `version`.
- `CHANGELOG.md` — move `[Unreleased]` entries into `## [X.Y.Z] - YYYY-MM-DD`, update link references at the bottom of the file.
- Docs version footers if they lag.

Commit:

```
Release vX.Y.Z

<one- or two-sentence summary suitable for the GitHub Release highlights>
```

Push and open the PR with title `Release vX.Y.Z`. Wait for CI to pass.

### 3. Merge the release PR

Squash-merge. Delete the branch.

### 4. Tag `main`

```bash
git checkout main
git pull --ff-only
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

Use an annotated tag. Signed tags are encouraged when the maintainer's GPG key is configured.

### 5. Create the GitHub Release

```bash
gh release create vX.Y.Z --title "vX.Y.Z" --notes-file release-notes.md
```

The notes file is written from the [Release Notes Template](#release-notes-template). Discard the file after the release is created — it is not part of the repo.

### 6. Verify

- `gt --version` reports the new version when built from the tagged commit.
- GitHub Release links to the correct tag and the correct CHANGELOG section.
- `## [Unreleased]` in `CHANGELOG.md` is empty but present.

---

## Release Notes Template

The GitHub Release body. Keep it short. Link out to the CHANGELOG for detail.

```markdown
<1–3 sentences: what this release is about. Written for a user, not a maintainer.>

## Highlights

- **<Two- to three-word headline>**: <one-sentence description of the most visible change>
- **<Two- to three-word headline>**: <one-sentence description>
- (Up to four bullets. If you need more, the release is too big.)

## Breaking changes

- **<Short title>**: <what broke, what the user must do>. <Optional link to migration doc.>

(Omit this section entirely if there are no breaking changes.)

## Full changes

See the [CHANGELOG entry for vX.Y.Z](https://github.com/mattmccarty/gt/blob/main/CHANGELOG.md#xyz---YYYY-MM-DD).

## Install

See [README.md](https://github.com/mattmccarty/gt/blob/main/README.md#installation).
```

### CHANGELOG anchor format

The anchor under `## Full changes` is the GitHub slug of the CHANGELOG heading, not a literal template. GitHub strips brackets, drops dots in the version, and substitutes the literal `-` plus its surrounding spaces with three hyphens. Concrete examples:

| CHANGELOG heading | GitHub anchor |
|---|---|
| `## [0.2.0] - 2026-04-17` | `#020---2026-04-17` |
| `## [0.3.0] - 2026-04-21` | `#030---2026-04-21` |
| `## [1.0.0] - 2027-01-15` | `#100---2027-01-15` |

If the anchor is wrong, the link renders but scrolls to the top of the CHANGELOG rather than to the version section, which is easy to miss on review.

### What goes in Highlights vs. CHANGELOG

The CHANGELOG enumerates. Highlights narrate. A Highlight reads like "we added X because Y"; a CHANGELOG bullet reads like "X was added." If a change isn't interesting enough to justify a sentence of context, it doesn't belong in Highlights.

---

## Tooling: cargo-release

**Decision: adopt `cargo-release` once the first manual release (0.2.0) has shipped.**

Rationale:

- The manual process above has enough steps that automation pays off after one or two iterations.
- `cargo-release` handles the steps we most want automated: version bump in `Cargo.toml`, CHANGELOG `[Unreleased]` → version move with date, commit, tag, push.
- Adopting it *before* the first manual release hides breakage in the process itself. We want the first release to shake out every manual step so we know what the tool needs to do.
- After 0.2.0, codify the rules in `release.toml` and switch over. Keep this doc as the fallback and as the spec for what the tool should do.

Alternatives considered:

- **`release-plz`**: heavier, includes PR-based release automation and changelog generation from commit messages. Good for repos with high PR volume and conventional commits enforced. Reconsider after 1.0.
- **Custom script**: avoid. Unless the process drifts meaningfully from what `cargo-release` expects, the cost of maintaining a custom script outweighs its flexibility.

---

## Post-Release

1. Verify the GitHub Release is visible and linked.
2. Close any issues that the release resolves (reference the tag in the closing comment).
3. Update the project board: move closed items to `Done`, adjust `Target release` fields on remaining items to the next version.
4. If the release includes a breaking change, pin the breaking-change entry in the project's announcement surface (GitHub Discussion, README banner, or equivalent).
5. If the release process surfaced gaps in this doc or in [013-branching.md](013-branching.md), update them before starting the next release cycle.

---

## See Also

- [013-branching.md](013-branching.md) — branching, merging, and tag mechanics.
- [CONTRIBUTING.md](../CONTRIBUTING.md) — contributor-facing summary.
- [CHANGELOG.md](../CHANGELOG.md) — the record this process maintains.
- [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) — the format spec.
- [Semantic Versioning](https://semver.org/) — the versioning spec.
