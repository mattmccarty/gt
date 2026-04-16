# Error Reporting in gt

## Overview

The `gt` tool uses Rust's standard error handling libraries to provide clear, actionable error messages:

- **`thiserror`** - For defining custom error types with structured fields
- **`anyhow`** - For application-level error handling with context chaining

## Error Categories

Each error type in `src/error.rs` has:
- Clear error message explaining what went wrong
- Exit code category (1-11) for scripting
- Optional suggestion for how to fix it

### Exit Codes

| Code | Category | Examples |
|------|----------|----------|
| 1 | General I/O errors | File system access, home directory |
| 2 | Configuration errors | Invalid TOML, missing config |
| 3 | Identity errors | Not found, already exists, validation |
| 4 | Repository errors | Not a git repo, missing remote |
| 5 | SSH errors | Key generation, config parsing |
| 6 | Git errors | Git commands, config parsing |
| 7 | URL errors | Unrecognized format, unknown provider |
| 8 | Strategy errors | Validation failed, migration issues |
| 9 | Permission errors | Insecure permissions, backup failed |
| 10 | User interaction | Cancelled, missing input |
| 11 | External tools | Tool not found, execution failed |

## SSH Config Parse Warnings

The SSH config parser (`src/io/ssh_config.rs`) includes a warning system for malformed configurations:

### ParseWarning Structure

```rust
pub struct ParseWarning {
    pub line_number: usize,   // 1-indexed line number
    pub directive: String,    // The problematic directive
    pub message: String,      // Detailed explanation
}
```

### Detection

The parser detects:
- Orphaned host-specific directives (HostName, IdentityFile, User, etc. without a Host block)
- Suspicious global directives
- Malformed entries

### Example Output

```
⚠️  SSH Config Warnings:
  Line 6: HostName - Host-specific directive 'HostName' found outside of any Host block.
          This is likely a corrupted SSH config. Expected format:
          Host <hostname>
              HostName github.com

To fix: Review ~/.ssh/config and ensure all host-specific
directives are under a 'Host' block.
```

## Using Warnings in Commands

Commands that parse SSH config should check for warnings:

```rust
let ssh_config = SshConfig::load(&path)?;

if ssh_config.has_warnings() {
    for warning in ssh_config.get_warnings() {
        eprintln!(
            "  Line {}: {} - {}",
            warning.line_number,
            warning.directive,
            warning.message
        );
    }
}
```

## Best Practices

1. **Always provide context** - Use `Error` variants with descriptive fields
2. **Suggest solutions** - Include fix instructions in error messages
3. **Don't fail silently** - Report warnings even if operation can continue
4. **Include line numbers** - Help users locate issues in config files
5. **Use appropriate exit codes** - Enable scripting and automation

## Testing Error Reporting

Tests should verify:
- Error messages are clear and accurate
- Line numbers are correct (1-indexed)
- Suggestions are helpful
- Exit codes are appropriate

Example:
```rust
let result = parse_corrupt_config();
assert!(result.is_ok(), "Should parse but with warnings");

let config = result.unwrap();
assert!(config.has_warnings());

let warnings = config.get_warnings();
assert!(warnings[0].message.contains("outside of any Host block"));
assert_eq!(warnings[0].line_number, 6);
```

## Future Enhancements

Potential improvements:
- Warning levels (info, warning, error)
- Suggestion auto-fix capabilities
- Colored output for better visibility
- Detailed error codes for each variant
- JSON error output for tool integration
