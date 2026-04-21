# CLAUDE.md - AI Assistant Project Guide

This document provides context, patterns, and conventions for AI assistants working on the `gt` (Git Identity Manager) project across sessions.

---

## Project Overview

**gt** is a cross-platform Rust CLI tool for managing multiple Git identities with SSH keys, conditional configs, and URL rewriting.

### Key Concept: Multi-Strategy Identities

The core innovation is that **identities support multiple concurrent strategies**:
- An identity can have SSH + Conditional + URL strategies **simultaneously**
- Each strategy type can have multiple variants (e.g., multiple conditional directories)
- Strategies are distinguished by discriminators (directory for conditional, scope for URL)

### Core Technologies
- **Language:** Rust 1.75.0+
- **CLI Framework:** Clap v4 (derive macros)
- **Config Format:** TOML (serde)
- **Testing:** Cargo test with integration tests
- **Supported Platforms:** Linux, macOS, Windows

---

## Architecture Patterns

### Module Structure

```
src/
├── cli/          # CLI argument parsing and output formatting
├── cmd/          # Command implementations (add, delete, list, use, etc.)
├── core/         # Core domain logic (identity, provider, path utils)
├── io/           # I/O operations (SSH config, git config, TOML)
├── strategy/     # Strategy implementations (ssh_alias, conditional, url_rewrite)
└── util/         # Utilities and validation
```

### Key Data Structures

**IdentityConfig** (`src/io/toml_config.rs`):
```rust
pub struct IdentityConfig {
    pub email: String,
    pub name: String,
    pub provider: String,
    pub strategies: Vec<StrategyConfig>,  // Multi-strategy support
    pub ssh: Option<IdentitySshConfig>,
    // Legacy fields (deprecated but kept for migration)
    pub strategy: Option<String>,
    pub conditional: Option<ConditionalConfig>,
    pub url_rewrite: Option<UrlRewriteConfig>,
}
```

**StrategyConfig** (`src/io/toml_config.rs`):
```rust
pub struct StrategyConfig {
    pub strategy_type: String,        // "ssh", "conditional", "url"
    pub priority: u8,
    pub enabled: bool,
    pub use_hostname_alias: bool,     // For SSH strategy
    pub directory: Option<String>,    // For conditional (discriminator)
    pub scope: Option<String>,        // For URL (discriminator)
    pub patterns: Option<Vec<String>>,
}
```

### Strategy Discriminators

**Discriminators** uniquely identify strategy variants:
- **SSH Strategy:** No discriminator (only one per identity)
- **Conditional Strategy:** Directory path (can have multiple)
- **URL Strategy:** Scope (organization/username, can have multiple)

Example: Identity "work" can have:
- 1 SSH strategy
- Conditional for `/work/projectA/`
- Conditional for `/work/projectB/`
- URL rewrite for scope `mycompany`
- URL rewrite for scope `clientorg`

---

## Critical Implementation Details

### 1. Git Conditional Include Patterns

**MUST have trailing slash for subdirectory matching:**

```rust
// ✅ CORRECT - Matches /path/ and all subdirectories
let condition = format!("gitdir:/path/");

// ❌ WRONG - Only matches exact directory /path
let condition = format!("gitdir:/path");
```

**Code location:** `src/cmd/add.rs:318-325`, `src/cmd/delete.rs:310-318`

**Pattern to use:**
```rust
// Normalize directory path - ensure it ends with /
let mut normalized_dir = directory.trim().to_string();
if !normalized_dir.ends_with('/') {
    normalized_dir.push('/');
}
let condition = format!("gitdir:{}", normalized_dir);
```

### 2. SSH Config User Field

**ALWAYS use `User git` for Git providers (GitHub, GitLab, Bitbucket):**

```ssh
# ✅ CORRECT
Host gt-work.github.com
    HostName github.com
    User git
    IdentityFile /home/user/.ssh/id_gt_work

# ❌ WRONG
Host gt-work.github.com
    User myusername  # Never use actual username
```

**Code location:** `src/strategy/ssh_alias.rs:124`

### 3. Git Config Section Ordering

**Global `[user]` MUST come BEFORE `[includeIf]` sections:**

```ini
# ✅ CORRECT ORDER
[user]
    email = default@example.com
    name = Default Name

[includeIf "gitdir:/work/"]
    path = ~/.gitconfig.d/work

# ❌ WRONG ORDER - includeIf will be overridden
[includeIf "gitdir:/work/"]
    path = ~/.gitconfig.d/work

[user]
    email = default@example.com
```

### 4. Git Conditional Include Limitation

**Important:** Git conditional includes with `gitdir:` only work **INSIDE** git repositories.

**Impact on cloning:**
```bash
# ❌ Won't use conditional config (not in a repo yet)
cd ~/work
git clone git@github.com:user/repo.git

# ✅ Must use SSH hostname alias for initial clone
git clone git@gt-work.github.com:user/repo.git

# ✅ OR use URL rewrite strategy (works everywhere)
git clone git@github.com:company/repo.git  # Auto-rewritten
```

**Solution:** For identities that need to clone repos, recommend:
- SSH hostname alias strategy
- URL rewrite strategy (preferred for organizations)

### 5. Legacy Strategy Migration

**Always migrate legacy config on read:**

```rust
// In any function that reads IdentityConfig
identity_config.migrate_legacy_strategies();
```

**Code location:** Multiple files including `src/cmd/delete.rs:44,149`, `src/cmd/add.rs:192`

---

## Common Workflows

### Adding a New Identity

**SSH strategy (basic):**
```bash
gt config id add work --email work@company.com --provider github
```

**Conditional strategy (directory-based):**
```bash
gt config id add work --strategy conditional --directory ~/work/
```

**URL rewrite strategy (organization-based):**
```bash
gt config id add work --strategy url --scope mycompany
```

**Multiple strategies on same identity:**
```bash
# First call creates identity with SSH
gt config id add work --email work@company.com --provider github

# Second call adds conditional to existing identity
gt config id add work --strategy conditional --directory ~/work/

# Third call adds URL rewrite to same identity
gt config id add work --strategy url --scope mycompany
```

### Deleting Strategy Variants

**Delete specific strategy:**
```bash
# Delete conditional for specific directory
gt config id delete work --strategy conditional --directory ~/work/

# Delete URL rewrite for specific scope
gt config id delete work --strategy url --scope mycompany
```

**Delete entire identity:**
```bash
# Deletes all strategies and optionally the SSH key
gt config id delete work
```

### Debugging Identity Issues

**Check current identity:**
```bash
gt config id status   # Shows current identity / detailed information
gt config id list     # All identities with strategies
```

**Check Git config:**
```bash
# Verify conditional includes are set up
git config --global --get-regexp includeIf

# Check effective config in a directory
cd ~/work/project
git config user.email
git config core.sshCommand
```

**Check SSH config:**
```bash
# Verify SSH hosts are configured
cat ~/.ssh/config | grep -A 5 "Host gt-"

# Test SSH connection
ssh -T git@gt-work.github.com
```

---

## Testing Patterns

### Unit Tests
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### Integration Tests

Located in `tests/` directory. Test full workflows:
- Adding identities with multiple strategies
- Deleting specific strategy variants
- Migration from legacy config format

### Manual Testing

**Test conditional includes:**
```bash
# Create test directories
mkdir -p /tmp/test-work
cd /tmp/test-work
git init
git config user.email  # Should show work email
```

**Test SSH strategy:**
```bash
# Clone using hostname alias
git clone git@gt-work.github.com:user/repo.git
```

**Test URL rewrite:**
```bash
# Clone using original URL (should rewrite)
git clone git@github.com:mycompany/repo.git
git config --local remote.origin.url  # Verify rewritten
```

---

## Documentation Standards

### File Naming Convention

All documentation in `docs/` follows `00X-topic.md` format:
- `001-architecture.md` - System architecture
- `002-strategies.md` - Strategy details
- `003-cli-reference.md` - CLI commands
- etc.

### Documentation Structure

**Progressive guide approach:**
1. **Getting Started** (001-003) - New users
2. **Configuration & Usage** (004-007) - Daily use
3. **Advanced Topics** (008-010) - Deep dives
4. **Troubleshooting** (011-012) - Common issues

### README Requirements

- Main `README.md` should be concise and generic
- Link to comprehensive docs in `docs/`
- Use horizontal rules (`---`) to separate sections
- Include table of contents for docs
- Each directory should have a README

---

## DRY and Pattern Reuse

Before writing new utility code, check whether an equivalent already exists. The codebase has clear seams — respect them.

### Where to look first

| Need | Look in | Key items |
|------|---------|-----------|
| Path operations | `src/core/path.rs` | `expand_tilde`, `ssh_config_path`, home dir resolution |
| Identity model | `src/core/identity.rs` | `Identity`, strategy attachment, legacy migration |
| Provider detection | `src/core/provider.rs` | Provider enums, URL parsing |
| Repo state | `src/core/repo.rs` | In-repo detection, remote inspection |
| URL handling | `src/core/url.rs` | Parse/transform git URLs |
| Config I/O | `src/io/toml_config.rs` | `IdentityConfig`, `StrategyConfig`, `save`, `load` |
| SSH config I/O | `src/io/ssh_config.rs` | Read/write `~/.ssh/config` stanzas |
| Git config I/O | `src/io/git_config.rs` | Global and includeIf manipulation |
| SSH key ops | `src/io/ssh_key.rs` | Generate, add, remove key files |
| Backups | `src/io/backup.rs` | Timestamped backup creation and restore |
| Strategy logic | `src/strategy/{ssh_alias,conditional,url_rewrite}.rs` | Apply/remove each strategy type |
| Scanners | `src/scan/` | Discover existing identities from SSH and git state |
| Validation | `src/util.rs` | Identity name, directory, scope validators |
| Errors | `src/error.rs` | `Error` enum, `Result` type alias |
| Output formatting | `src/cli/output.rs` | `Output` struct (success, dry_run, with_detail) |
| Interactive prompts | `src/cli/interactive.rs` | Dialoguer wrappers |

### When to extract vs. inline

- **Three near-duplicate blocks is fine.** Copy, don't abstract yet.
- **Four or more callers of the same pattern** is the signal to extract into the nearest matching module above.
- **Don't invent a trait** until you have two concrete implementations that both need to satisfy the same API.
- **Don't create a new module** to hold one function. Find the existing module it belongs in.

### Concrete DRY reminders for this repo

1. **Always call `IdentityConfig::migrate_legacy_strategies()` after loading config.** Every command that reads config must call this. Callers live in `src/cmd/add.rs`, `src/cmd/delete.rs`, etc. When adding a new command that reads config, follow the same pattern — do not re-implement migration logic.
2. **Normalize directory paths with a trailing slash before writing `gitdir:` conditions.** The helper pattern (trim + append `/`) exists in `src/cmd/add.rs` and `src/cmd/delete.rs`. If a third command needs the same logic, extract it into `src/core/path.rs` rather than copying a third time.
3. **SSH `User` field is always `git` for Git providers.** Enforced in `src/strategy/ssh_alias.rs`. Never add a new code path that writes a different value.
4. **Git config section ordering matters**: `[user]` must come before any `[includeIf]`. `src/io/git_config.rs` handles this — route all global git config writes through it.
5. **Commands return `Result<Output>` and accept `&Context`.** Every command in `src/cmd/` follows this shape. Do not return `String` or `()` from a new command.
6. **Use `ctx.dry_run` and `ctx.force` consistently.** The `Context` struct already carries these flags. Check them at the top of any command that mutates state; emit `Output::dry_run(...)` when `dry_run` is set.

### Anti-patterns to avoid

- Building your own path resolver instead of using `src/core/path.rs`.
- Reading or writing `~/.ssh/config` directly instead of going through `src/io/ssh_config.rs`.
- Catching errors with `unwrap()` or `expect()` in command paths. Propagate via `?` and let `Error` carry the context.
- Adding a new error variant without considering whether an existing variant covers the case.
- Writing a new CLI output formatter instead of adding a method to `Output`.
- Duplicating validation logic that exists in `src/util.rs`.

---

## Code Conventions

### Error Handling

Use custom error types from `src/error.rs`:
```rust
use crate::error::{Error, Result};

fn my_function() -> Result<Output> {
    // Use ? operator for error propagation
    let config = load_config()?;

    // Use specific error types
    return Err(Error::IdentityNotFound {
        name: "work".to_string(),
    });
}
```

### Output Formatting

Use `Output` struct from `src/cli/output.rs`:
```rust
use crate::cli::output::Output;

fn execute() -> Result<Output> {
    Ok(Output::success("Operation completed")
        .with_detail("identity", "work")
        .with_detail("strategy", "conditional"))
}
```

### Context Usage

Commands receive `Context` for configuration and logging:
```rust
pub fn execute(opts: &AddOpts, ctx: &Context) -> Result<Output> {
    ctx.info("Starting operation...");
    ctx.debug(&format!("Debug info: {}", value));

    if ctx.dry_run {
        return Ok(Output::dry_run("Would perform operation"));
    }

    if ctx.force {
        // Skip confirmations
    }
}
```

### Validation

Always validate user input:
```rust
use crate::util::validate_identity_name;

validate_identity_name(&opts.name)?;
```

---

## File Locations

### Configuration Files

**Linux/macOS:**
- gt config: `~/.config/gt/config.toml`
- SSH config: `~/.ssh/config`
- Global git config: `~/.gitconfig`
- Conditional includes: `~/.gitconfig.d/<identity>`

**Windows:**
- gt config: `%APPDATA%\gt\config.toml`
- SSH config: `%USERPROFILE%\.ssh\config`
- Global git config: `%USERPROFILE%\.gitconfig`
- Conditional includes: `%USERPROFILE%\.gitconfig.d\<identity>`

### SSH Keys

Default location: `~/.ssh/id_gt_<identity>`

### Backup Files

SSH config backups: `~/.ssh/config.backup.<timestamp>`

---

## Common Issues and Solutions

### Issue: Clone fails with "Repository not found"

**Symptoms:** `git clone git@github.com:user/repo.git` fails

**Causes:**
1. SSH config has wrong `User` field (should be `git`, not username)
2. Using wrong SSH key
3. SSH host alias not in SSH config

**Solutions:**
```bash
# Check SSH config
cat ~/.ssh/config | grep -A 5 "Host gt-work"

# Verify User field is "git"
# Test SSH connection
ssh -T git@gt-work.github.com

# Use hostname alias for cloning
git clone git@gt-work.github.com:user/repo.git
```

### Issue: Conditional includes not working

**Symptoms:** Git config not applying in subdirectories

**Causes:**
1. Missing trailing slash on `gitdir:` pattern
2. `[user]` section comes after `[includeIf]` sections
3. Not inside a git repository

**Solutions:**
```bash
# Check includeIf patterns
git config --global --get-regexp includeIf

# Verify patterns have trailing slashes
# gitdir:/path/ ✅  gitdir:/path ❌

# Check section ordering in ~/.gitconfig
# [user] should come BEFORE [includeIf]

# Test inside a git repository
cd /path/to/repo
git config user.email
```

### Issue: Multiple strategy variants conflict

**Symptoms:** Wrong identity applied, or unexpected behavior

**Causes:**
1. Strategy priorities not set correctly
2. Multiple conditional strategies with overlapping directories
3. Discriminators not unique

**Solutions:**
```bash
# Check identity configuration
gt config id status <identity>

# Review strategy priorities in config.toml
# Higher priority wins (default: ssh=100, conditional=50, url=25)

# Ensure discriminators are unique per strategy type
```

---

## Build and Release

### Development Build
```bash
cargo build
./target/debug/gt --version
```

### Release Build
```bash
cargo build --release
./target/release/gt --version
```

### Linting and Formatting
```bash
# Run clippy
cargo clippy -- -D warnings

# Format code
cargo fmt

# Check formatting
cargo fmt --check
```

### Running with Debug Logging
```bash
RUST_LOG=debug cargo run -- id list
RUST_LOG=trace cargo run -- id add work --email work@example.com
```

---

## Cross-Platform Considerations

### Path Handling

Always use `crate::core::path` utilities:
```rust
use crate::core::path;

// Expand tilde in paths
let expanded = path::expand_tilde(&path)?;

// Get SSH config path (cross-platform)
let ssh_config = path::ssh_config_path()?;

// Get home directory
let home = dirs::home_dir().unwrap();
```

### Line Endings

Git config files use platform-specific line endings:
- Unix: `\n`
- Windows: `\r\n`

Use `writeln!` macro which handles this automatically.

### SSH Config Format

SSH config is sensitive to:
- Indentation (use 4 spaces)
- Key order (Host first, then directives)
- Comments (use `#`)

---

## Migration and Compatibility

### Legacy Config Format

**Old format (single strategy):**
```toml
[identities.work]
email = "work@company.com"
name = "Work Name"
provider = "github"
strategy = "ssh"  # Single strategy field
```

**New format (multi-strategy):**
```toml
[identities.work]
email = "work@company.com"
name = "Work Name"
provider = "github"

[[identities.work.strategies]]
strategy_type = "ssh"
priority = 100
enabled = true
use_hostname_alias = true

[[identities.work.strategies]]
strategy_type = "conditional"
directory = "/work/"
priority = 50
enabled = true
```

### Migration Process

**Automatic migration** via `IdentityConfig::migrate_legacy_strategies()`:
1. Called automatically when reading config
2. Converts legacy fields to new `strategies` array
3. Clears legacy fields
4. Saves updated config

**Manual migration:**
```bash
# Simply run any gt command
gt config id list  # Triggers migration on config read
```

---

## Git Workflow

Full branching, PR, and release conventions live in [`docs/013-branching.md`](docs/013-branching.md). Contributor-facing rules live in [`CONTRIBUTING.md`](CONTRIBUTING.md). Read both before your first commit in a new session.

### Critical rule: no AI/agent attribution

Everything that lands in the repo or on GitHub must read as human-authored.

- **Do not** add `Co-Authored-By: Claude …` trailers to commits.
- **Do not** add "Generated with Claude Code" footers to PRs, issues, or comments.
- **Do not** reference AI, LLM, Claude, or agents in code comments, docs, commit messages, issues, PRs, or review comments.
- **Do not** write "I used an agent to …" in PR bodies or issue descriptions.

This rule is non-negotiable. If you are about to commit, open a PR, file an issue, or leave a review comment, strip all such references first.

### Commit style

- Imperative mood subject under 72 chars, capitalized, no trailing period.
- Body explains *why*, not *what*.
- Reference issues via trailers (`Fixes #42`).

Example:

```
Add passthrough for unknown git config subcommands

Unknown subcommands under `gt config` now forward to `git config`,
preserving exit codes and stderr. Explicit gt subcommands continue
to be handled internally.

Fixes #12
```

### Branching

- Work on `feat/*`, `fix/*`, `chore/*`, or `docs/*` branches off `main`.
- Never commit directly to `main`.
- Squash-merge PRs. Delete the branch after merge.
- Releases are tagged `vX.Y.Z` from `main`.

---

## Quick Reference Checklist

When working on this project:

- [ ] Read `docs/001-architecture.md` for system overview
- [ ] Read `CONTRIBUTING.md` and `docs/013-branching.md` before committing
- [ ] Check `docs/002-strategies.md` for strategy details
- [ ] Reuse existing modules (see "DRY and Pattern Reuse" table above)
- [ ] Always normalize directory paths with trailing slashes
- [ ] Always use `User git` in SSH config for Git providers
- [ ] Test with `cargo test` before committing
- [ ] Run `cargo clippy -- -D warnings` and `cargo fmt --check`
- [ ] Update documentation if changing behavior
- [ ] Call `migrate_legacy_strategies()` when reading config
- [ ] Use `Context` for logging and dry-run support
- [ ] Return `Result<Output>` from command functions
- [ ] Follow `00X-` naming for documentation files
- [ ] **No AI/agent attribution** in commits, PRs, issues, comments, or code

---

## Resources

- **Main README:** `README.md`
- **Contributor Guide:** `CONTRIBUTING.md`
- **Documentation Index:** `docs/README.md`
- **Architecture:** `docs/001-architecture.md`
- **CLI Reference:** `docs/003-cli-reference.md`
- **Configuration:** `docs/004-configuration.md`
- **Development Guide:** `docs/008-development.md`
- **Bug Reports:** `docs/011-bug-reports.md`
- **Error Handling:** `docs/012-error-handling.md`
- **Branching Strategy:** `docs/013-branching.md`

---

## Notes for AI Assistants

### Context Preservation

This file should be read at the start of each session to understand:
- Project architecture and patterns
- Critical implementation details (trailing slashes, SSH user field, etc.)
- Common workflows and debugging approaches
- Testing and documentation standards

### Common Patterns Learned

1. **Multi-strategy is the core value proposition** - Always consider how changes affect multiple concurrent strategies
2. **Git has subtle requirements** - Trailing slashes, section ordering, gitdir limitations
3. **SSH config is finicky** - User field, Host matching, key permissions
4. **Migration must be automatic** - Always migrate legacy configs on read
5. **Cross-platform matters** - Use path utilities, test on Windows

### When Stuck

1. Check existing code patterns in `src/cmd/` for similar operations
2. Review integration tests in `tests/` for usage examples
3. Read strategy implementations in `src/strategy/` for details
4. Consult documentation in `docs/` for design rationale
5. Test manually with `cargo run -- <command>` and `RUST_LOG=debug`

---

**Version:** 0.3.0
**Last Updated:** 2026-04-21
