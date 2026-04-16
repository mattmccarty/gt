# Backup and Restore Guide

## Overview

The `gt` tool includes comprehensive backup and restore capabilities for your SSH configuration. This ensures you never lose critical SSH settings due to corruption, mistakes, or failed operations.

## Table of Contents

- [Why Backup?](#why-backup)
- [Automatic Backups](#automatic-backups)
- [Manual Backups](#manual-backups)
- [Listing Backups](#listing-backups)
- [Restoring Backups](#restoring-backups)
- [Corruption Detection](#corruption-detection)
- [Configuration](#configuration)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Why Backup?

Your SSH config (`~/.ssh/config`) is critical infrastructure:

- **Controls SSH authentication** for all git operations
- **Corruption can break all SSH access** to remote repositories
- **Manual editing is error-prone** and can introduce syntax errors
- **Recovery without backups is difficult** or impossible

The `gt` backup system provides:

✅ **Automatic backups** before any SSH config modification
✅ **Corruption detection** with detailed error messages
✅ **One-command restore** from any previous state
✅ **Validation** of backups before restore
✅ **Safe rotation** keeping the last 10 backups by default

## Automatic Backups

### How It Works

Every time `gt` modifies your SSH config (via `gt id add`, `gt id switch`, etc.), it:

1. **Creates a timestamped backup** of the current state
2. **Applies the modification**
3. **Rotates old backups** (keeps last 10 by default)

Backup naming format:
```
~/.ssh/config.YYYYMMDD_HHMMSS.bak
```

Example:
```
~/.ssh/config.20240319_143022.bak
~/.ssh/config.20240319_120015.bak
~/.ssh/config.20240318_220000.bak
```

### Disabling Auto-Backup

Auto-backup is enabled by default. To disable (not recommended):

```toml
# ~/.config/gt/config.toml
[backup]
auto_backup = false
```

Or via environment variable:
```bash
export GT_BACKUP_AUTO=false
```

## Manual Backups

### Creating a Manual Backup

```bash
# Create backup of current SSH config
gt id backup create
```

Output:
```
Created SSH config backup
  path: ~/.ssh/config.20240319_143022.bak
```

### When to Create Manual Backups

- **Before manual editing** of `~/.ssh/config`
- **Before system updates** that might affect SSH
- **Before experimenting** with new configurations
- **Creating restore points** for known-good states

### With Notes (Future Feature)

```bash
# Add a note to remember why you created this backup
gt id backup create --note "Before switching to URL rewrite strategy"
```

## Listing Backups

### Basic List

```bash
gt id backup list
```

Output:
```
Found 5 SSH config backup(s)

#  TIMESTAMP            AGE           STATUS
1  2024-03-19 14:30:22  2 hours ago   -
2  2024-03-19 12:15:01  4 hours ago   -
3  2024-03-18 22:00:00  18 hours ago  -
4  2024-03-18 15:30:00  25 hours ago  -
5  2024-03-17 10:00:00  2 days ago    -
```

### With Validation

```bash
gt id backup list --validate
```

Output:
```
Found 5 SSH config backup(s)

#  TIMESTAMP            AGE           STATUS
1  2024-03-19 14:30:22  2 hours ago   OK
2  2024-03-19 12:15:01  4 hours ago   OK
3  2024-03-18 22:00:00  18 hours ago  2 warnings
4  2024-03-18 15:30:00  25 hours ago  OK
5  2024-03-17 10:00:00  2 days ago    CORRUPT
```

Status values:
- **OK**: No issues, safe to restore
- **N warnings**: Minor issues but usable (orphaned directives, etc.)
- **CORRUPT**: Major issues, cannot be parsed

### Detailed View

```bash
gt id backup list --all
```

Output:
```
Found 5 SSH config backup(s)

#  TIMESTAMP            AGE           SIZE    HOSTS  STATUS
1  2024-03-19 14:30:22  2 hours ago   2.3 KB  8      OK
2  2024-03-19 12:15:01  4 hours ago   2.1 KB  7      OK
3  2024-03-18 22:00:00  18 hours ago  1.9 KB  6      2 warnings
4  2024-03-18 15:30:00  25 hours ago  2.0 KB  7      OK
5  2024-03-17 10:00:00  2 days ago    1.5 KB  0      CORRUPT
```

## Restoring Backups

### Automatic Restore (Recommended)

Restores the most recent valid backup:

```bash
gt id backup restore
```

Interactive prompt:
```
Restore SSH config from backup:
  Backup: 2024-03-19 14:30:22 UTC
  Hosts:  8 entries

Restore this backup? [y/N] y

SSH config restored successfully
  restored_from: 2024-03-19 14:30:22
  previous_config_backed_up_to: ~/.ssh/config.20240319_163045.bak
```

### Restore Specific Backup

By index:
```bash
gt id backup restore 2
```

By timestamp (partial match):
```bash
gt id backup restore 2024-03-19
```

By path:
```bash
gt id backup restore ~/.ssh/config.20240319_143022.bak
```

### Non-Interactive Restore

Skip confirmation prompt:
```bash
gt id backup restore --yes
```

### Force Restore

Restore even if backup has warnings:
```bash
gt id backup restore --force
```

### Skip Pre-Restore Backup

Don't backup current state before restoring (not recommended):
```bash
gt id backup restore --no-backup
```

### Dry Run

See what would happen without actually restoring:
```bash
gt id backup restore --dry-run
```

## Corruption Detection

### During Scan

When you run `gt id scan`, it checks for SSH config corruption:

```bash
gt id scan
```

Output with corruption:
```
⚠️  SSH Config Warnings:
  Line 6: HostName - Host-specific directive 'HostName' found outside
  of any Host block. This is likely a corrupted SSH config. Expected format:
  Host <hostname>
      HostName github.com

  Line 7: User - Host-specific directive 'User' found outside of any
  Host block. This is likely a corrupted SSH config.

To fix: Review ~/.ssh/config and ensure all host-specific
directives are under a 'Host' block.

A valid backup is available from 2 hours ago.
To restore: gt id backup restore
To see all backups: gt id backup list --validate

Scan complete
  ssh_warnings: 2
  ssh_entries: 0
```

### Validation Before Restore

The tool automatically validates backups before restoring:

```bash
gt id backup restore 5
```

Output if backup is corrupted:
```
Error: Backup has 4 warnings. Use --force to restore anyway.
  Line 6: HostName found outside of any Host block
  Line 7: User found outside of any Host block
  Line 8: IdentityFile found outside of any Host block
  Line 9: IdentitiesOnly found outside of any Host block
```

### Common Corruption Patterns

| Issue | Description | Severity |
|-------|-------------|----------|
| Orphaned directives | Host-specific directives outside Host block | Major |
| Missing Host line | Indented directives with no preceding Host | Major |
| Malformed syntax | Invalid key-value pairs | Major |
| Unknown directives | Custom directives (usually safe) | Minor |
| Duplicate Host entries | Same host pattern multiple times | Minor |

## Configuration

### Config File

```toml
# ~/.config/gt/config.toml

[backup]
# Maximum number of SSH config backups to keep
max_ssh_backups = 10

# Auto-backup before any SSH config modification
auto_backup = true

# Custom backup directory (default: same as original file)
# backup_dir = "~/.config/gt/backups"
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `GT_BACKUP_MAX` | Maximum backups to keep | 10 |
| `GT_BACKUP_AUTO` | Auto-backup enabled | true |
| `GT_BACKUP_DIR` | Custom backup directory | (same as file) |

## Best Practices

### 1. Keep Auto-Backup Enabled

Don't disable automatic backups unless you have a specific reason. The overhead is minimal and the protection is invaluable.

### 2. Validate Backups Periodically

```bash
# Monthly check
gt id backup list --validate
```

### 3. Create Manual Backups Before Manual Edits

```bash
gt id backup create
vim ~/.ssh/config
gt id scan  # Verify no corruption introduced
```

### 4. Test Restores Occasionally

```bash
# In a test environment
gt id backup list
gt id backup restore --dry-run
```

### 5. Increase Retention for Critical Systems

```toml
[backup]
max_ssh_backups = 30  # Keep a month of backups
```

### 6. Document Custom Backups

```bash
# Future feature
gt id backup create --note "Before multi-account GitLab setup"
```

### 7. Use Validation When Investigating Issues

```bash
# When things stop working
gt id scan
gt id backup list --validate
```

## Troubleshooting

### No Backups Available

**Problem**: `gt id backup list` shows no backups

**Solutions**:
1. Check if backups exist manually:
   ```bash
   ls -la ~/.ssh/*.bak
   ```

2. Create a manual backup of current state:
   ```bash
   gt id backup create
   ```

3. Check custom backup directory if configured:
   ```bash
   # In ~/.config/gt/config.toml
   echo $GT_BACKUP_DIR
   ```

### All Backups Are Corrupted

**Problem**: `gt id backup list --validate` shows all backups as CORRUPT

**Solutions**:
1. Try restoring the least corrupt one:
   ```bash
   gt id backup restore 3 --force
   ```

2. Manually inspect a backup:
   ```bash
   cat ~/.ssh/config.20240319_143022.bak
   ```

3. Recreate SSH config from scratch:
   ```bash
   # Backup corrupted config first
   cp ~/.ssh/config ~/.ssh/config.broken

   # Start fresh
   rm ~/.ssh/config
   gt id add work --email work@example.com --user-name "Your Name"
   ```

### Restore Fails

**Problem**: `gt id backup restore` fails with I/O error

**Solutions**:
1. Check permissions:
   ```bash
   ls -la ~/.ssh/config
   chmod 600 ~/.ssh/config
   ```

2. Check disk space:
   ```bash
   df -h ~
   ```

3. Verify backup file exists and is readable:
   ```bash
   cat ~/.ssh/config.20240319_143022.bak
   ```

### Can't Find Recent Backup

**Problem**: Recent backup not showing in list

**Solutions**:
1. Refresh the list:
   ```bash
   gt id backup list
   ```

2. Check file timestamps:
   ```bash
   ls -lt ~/.ssh/*.bak | head
   ```

3. Verify backup naming format matches expected pattern:
   ```bash
   ls ~/.ssh/config.*.bak
   ```

### Pre-Restore Backup Not Created

**Problem**: Restore succeeded but no pre-restore backup created

**Solutions**:
1. Check if you used `--no-backup`:
   ```bash
   # History check
   history | grep "backup restore"
   ```

2. Verify backup directory has space:
   ```bash
   df -h ~/.ssh/
   ```

3. Check if original config existed:
   ```bash
   # If ~/.ssh/config didn't exist, no pre-restore backup is made
   ```

### Restore Corrupted Config

**Problem**: Restored a backup but it's still corrupted

**Solutions**:
1. Scan to see specific issues:
   ```bash
   gt id scan
   ```

2. Try an earlier backup:
   ```bash
   gt id backup list --validate
   gt id backup restore 4  # Try #4 instead
   ```

3. Manual fix:
   ```bash
   vim ~/.ssh/config
   # Look for directives outside Host blocks
   # Ensure format is:
   # Host example.com
   #     HostName example.com
   #     User git
   ```

## Advanced Usage

### Scripted Backup Management

```bash
#!/bin/bash
# Create backup before system maintenance

set -e

echo "Creating pre-maintenance SSH config backup..."
gt id backup create

echo "Performing system updates..."
sudo apt update && sudo apt upgrade -y

echo "Verifying SSH config..."
if ! gt id scan --quiet; then
    echo "ERROR: SSH config corrupted during update!"
    echo "Restoring from backup..."
    gt id backup restore --yes
fi

echo "Maintenance complete"
```

### Scheduled Validation

```bash
#!/bin/bash
# Cron job: 0 0 * * 0 (weekly)

gt id backup list --validate | grep CORRUPT && {
    echo "WARNING: Corrupted backups detected!" | mail -s "SSH Config Alert" admin@example.com
}
```

### Backup Rotation Policy

```toml
# Keep more backups for production systems
[backup]
max_ssh_backups = 50
```

### Backup to External Location

```toml
# Store backups on network drive
[backup]
backup_dir = "/mnt/backup/ssh_configs"
```

## JSON Output

All backup commands support JSON output for scripting:

```bash
gt id backup list -o json
```

Output:
```json
{
  "success": true,
  "message": "Found 3 SSH config backup(s)",
  "table": [
    {
      "index": "1",
      "timestamp": "2024-03-19T14:30:22Z",
      "age": "2 hours ago",
      "status": "OK"
    }
  ]
}
```

## Future Enhancements

Planned features for future releases:

- **Backup notes**: Tag backups with descriptive notes
- **Diff view**: Compare current config with any backup
- **Remote backups**: Sync backups to cloud storage
- **Compression**: Compress old backups to save space
- **Backup sets**: Bundle SSH config + SSH keys together
- **Auto-recovery**: Automatically restore on corruption detection
- **Backup hooks**: Run custom scripts before/after backup

## See Also

- [ERROR_REPORTING.md](ERROR_REPORTING.md) - Error handling and reporting
- [005-security.md](005-security.md) - Security best practices
- [003-cli-reference.md](003-cli-reference.md) - Complete CLI reference
