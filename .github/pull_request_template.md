## Summary

<!-- 1–3 bullets: what this change does and why. -->

## Changes

<!-- Bulleted list of concrete changes. -->

## Testing

<!-- What you ran, what passed, how to reproduce. For docs-only PRs, say so. -->

## Related

<!-- Fixes #N, Refs #N. Delete this section if nothing applies. -->

---

## Pre-merge checklist

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` is clean
- [ ] `cargo fmt --check` passes
- [ ] Docs updated if behavior changed
- [ ] `CHANGELOG.md` entry added under `## [Unreleased]` for user-visible changes (see [docs/014-releases.md](../docs/014-releases.md#changelog-workflow))
- [ ] No AI or agent attribution anywhere in commits, PR body, or diff (see [CONTRIBUTING.md](../CONTRIBUTING.md#house-rules))
