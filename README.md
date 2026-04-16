# gt - Git Identity Manager

A cross-platform CLI tool for managing multiple Git identities with SSH keys, conditional configs, and URL rewriting.

---

## What is gt?

**gt** solves the problem of managing multiple Git identities (work, personal, clients) across different accounts and providers. Use different SSH keys and email addresses for different projects automatically.

### Core Features

- **Multiple Strategies**: SSH hostname aliases, Git conditional includes, or URL rewriting
- **Multi-Strategy Support**: Use SSH + Conditional + URL strategies together on one identity
- **Auto-Detection**: Scans and detects existing Git/SSH configurations
- **Backup & Restore**: Automatic SSH config backups with corruption detection
- **Cross-Platform**: Linux, macOS, and Windows support
- **SSH Key Management**: Generate, track, and activate SSH keys
- **Smart Defaults**: Works out of the box with minimal configuration

---

## Quick Start

```bash
# Add an identity with SSH strategy
gt id add work --email work@company.com --provider github

# Add conditional strategy for a directory
gt id add work --strategy conditional --directory ~/work/

# Add URL rewrite strategy for an organization
gt id add work --strategy url --scope mycompany

# Clone with automatic identity detection
git clone git@github.com:mycompany/repo.git

# Show current identity
gt id

# List all identities and their strategies
gt id list
```

---

## Installation

### From Source

Requires Rust 1.75.0 or later:

```bash
cargo install --path .
```

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/mattmccartyllc/gitid/releases).

---

## Core Commands

| Command | Description |
|---------|-------------|
| `gt id` | Show current identity status |
| `gt id add <name>` | Add new identity or strategy variant |
| `gt id list` | List all identities with strategies |
| `gt id use <identity>` | Use identity in current repository |
| `gt id delete <identity>` | Delete identity or specific strategy |
| `gt id status` | Show detailed identity information |
| `gt id key` | Manage SSH keys |

**See:** [docs/003-cli-reference.md](docs/003-cli-reference.md) for complete command documentation.

---

## Configuration

Configuration is stored in TOML format:

- **Linux/macOS**: `~/.config/gt/config.toml`
- **Windows**: `%APPDATA%\gt\config.toml`

**See:** [docs/004-configuration.md](docs/004-configuration.md) for configuration details.

---

## Supported Providers

- GitHub (github.com)
- GitLab (gitlab.com)
- Bitbucket (bitbucket.org)
- Azure DevOps (dev.azure.com)
- AWS CodeCommit
- Custom/self-hosted Git servers

**See:** [docs/002-strategies.md](docs/002-strategies.md) for provider-specific setup.

---

## Documentation

Comprehensive documentation is organized as a progressive guide in the [docs/](docs/) directory:

### Getting Started
- **[docs/README.md](docs/README.md)** - Documentation index and reading guide
- **[001-architecture.md](docs/001-architecture.md)** - System architecture and design
- **[002-strategies.md](docs/002-strategies.md)** - Identity strategies explained
- **[003-cli-reference.md](docs/003-cli-reference.md)** - Complete CLI reference

### Configuration & Usage
- **[004-configuration.md](docs/004-configuration.md)** - Configuration file reference
- **[005-security.md](docs/005-security.md)** - Security model and best practices
- **[006-cross-platform.md](docs/006-cross-platform.md)** - Platform compatibility guide
- **[007-migration.md](docs/007-migration.md)** - Migration and upgrade procedures

### Advanced Topics
- **[008-development.md](docs/008-development.md)** - Developer guide and testing
- **[009-big-picture.md](docs/009-big-picture.md)** - High-level system overview
- **[010-backup-restore.md](docs/010-backup-restore.md)** - Backup and recovery system

### Troubleshooting
- **[011-bug-reports.md](docs/011-bug-reports.md)** - Known issues and resolutions
- **[012-error-handling.md](docs/012-error-handling.md)** - Error handling and troubleshooting

---

## Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- id list

# Build release binary
cargo build --release

# Run lints
cargo clippy -- -D warnings

# Format code
cargo fmt
```

**See:** [docs/008-development.md](docs/008-development.md) for contribution guidelines.

---

## License

Apache 2.0 License - See [LICENSE](../LICENSE) for details.

Copyright (c) 2026 Matt McCarty LLC
