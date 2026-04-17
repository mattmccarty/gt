# gt Documentation

This directory contains comprehensive documentation for the gt CLI tool.

## Table of Contents

The documentation is organized as a progressive guide, with each document building on the previous:

### Core Documentation

| # | Document | Description |
|---|----------|-------------|
| 1 | [001-architecture.md](001-architecture.md) | System architecture, module breakdown, and design patterns |
| 2 | [002-strategies.md](002-strategies.md) | Deep dive into the three identity management strategies |
| 3 | [003-cli-reference.md](003-cli-reference.md) | Complete CLI command reference with examples |
| 4 | [004-configuration.md](004-configuration.md) | Configuration file format and options |
| 5 | [005-security.md](005-security.md) | Security model, permissions, and best practices |
| 6 | [006-cross-platform.md](006-cross-platform.md) | Platform-specific considerations and compatibility |
| 7 | [007-migration.md](007-migration.md) | Migration guides and URL fixing |
| 8 | [008-development.md](008-development.md) | Developer guide, testing, and contribution guidelines |
| 9 | [009-big-picture.md](009-big-picture.md) | High-level system diagram tying everything together |
| 10 | [010-backup-restore.md](010-backup-restore.md) | Backup and restore system for SSH configs |
| 11 | [011-bug-reports.md](011-bug-reports.md) | Known issues, bug reports, and resolutions |
| 12 | [012-error-handling.md](012-error-handling.md) | Error handling, validation, and troubleshooting |
| 13 | [013-branching.md](013-branching.md) | Branching strategy, merging, and tag mechanics |
| 14 | [014-releases.md](014-releases.md) | Release process, CHANGELOG workflow, release-notes template |

## Reading Order

### For New Users
1. Start with [001-architecture.md](001-architecture.md) for system overview
2. Read [002-strategies.md](002-strategies.md) to understand identity strategies
3. Use [003-cli-reference.md](003-cli-reference.md) as a daily reference
4. Reference [004-configuration.md](004-configuration.md) when customizing behavior

### For Developers
1. All of the above, plus:
2. [008-development.md](008-development.md) for contribution guidelines
3. [009-big-picture.md](009-big-picture.md) for the complete system view
4. [011-bug-reports.md](011-bug-reports.md) for known issues and fixes
5. [012-error-handling.md](012-error-handling.md) for error handling patterns

### For Security Review
1. [005-security.md](005-security.md) for security model and threat analysis
2. [006-cross-platform.md](006-cross-platform.md) for platform-specific security considerations

### For Troubleshooting
1. [012-error-handling.md](012-error-handling.md) for common errors and solutions
2. [010-backup-restore.md](010-backup-restore.md) for recovery procedures
3. [011-bug-reports.md](011-bug-reports.md) for known issues

## Diagram Legend

Throughout the documentation, we use Mermaid diagrams with consistent styling:

```
Component Types:
- [Rectangle] = Module/Component
- ([Oval]) = Data Store
- {Diamond} = Decision Point
- [[Subroutine]] = External System

Colors (when supported):
- Blue = Core components
- Green = Strategy implementations
- Orange = I/O operations
- Gray = External systems
```

## Version

This documentation corresponds to gt version 0.1.0.
