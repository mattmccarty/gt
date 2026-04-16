//! Git hook management
//!
//! This module handles installation and removal of Git hooks,
//! with support for preserving existing hooks via wrapper chaining.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::Result;

/// Marker to identify gt-managed hooks
const GT_MARKER: &str = "# GT_MARKER: managed by gt";

/// Pre-push hook template
const PRE_PUSH_HOOK_TEMPLATE: &str = r#"#!/bin/sh
# GT_MARKER: managed by gt
# This hook prevents pushes when a schedule is active

# Call gt to check if push should be blocked
gt_result=$(gt push --hook-check "$1" "$2" 2>&1)
gt_exit=$?

if [ $gt_exit -eq 1 ]; then
    echo "$gt_result"
    exit 1
fi

# Chain to original hook if it exists
if [ -x ".git/hooks/pre-push.gt-original" ]; then
    exec .git/hooks/pre-push.gt-original "$@"
fi

exit 0
"#;

/// Get the hooks directory for a repository
fn hooks_dir(repo_path: &Path) -> PathBuf {
    repo_path.join(".git").join("hooks")
}

/// Get the pre-push hook path
fn pre_push_hook_path(repo_path: &Path) -> PathBuf {
    hooks_dir(repo_path).join("pre-push")
}

/// Get the backup path for original pre-push hook
fn pre_push_original_path(repo_path: &Path) -> PathBuf {
    hooks_dir(repo_path).join("pre-push.gt-original")
}

/// Check if the pre-push hook is managed by gt
pub fn is_hook_managed(repo_path: &Path) -> Result<bool> {
    let hook_path = pre_push_hook_path(repo_path);

    if !hook_path.exists() {
        return Ok(false);
    }

    let contents = fs::read_to_string(&hook_path)?;
    Ok(contents.contains(GT_MARKER))
}

/// Install the pre-push hook
///
/// If an existing pre-push hook is found that is not managed by gt,
/// it will be renamed to pre-push.gt-original and chained.
pub fn install_pre_push_hook(repo_path: &Path) -> Result<()> {
    let hooks_dir = hooks_dir(repo_path);
    let hook_path = pre_push_hook_path(repo_path);
    let original_path = pre_push_original_path(repo_path);

    // Ensure hooks directory exists
    if !hooks_dir.exists() {
        fs::create_dir_all(&hooks_dir)?;
    }

    // Check if there's an existing hook that's not managed by gt
    if hook_path.exists() {
        let contents = fs::read_to_string(&hook_path)?;

        // If it's already managed by gt, we're done
        if contents.contains(GT_MARKER) {
            return Ok(());
        }

        // Otherwise, back it up
        fs::rename(&hook_path, &original_path)?;
    }

    // Write the new hook
    let mut file = fs::File::create(&hook_path)?;
    file.write_all(PRE_PUSH_HOOK_TEMPLATE.as_bytes())?;

    // Make it executable (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    Ok(())
}

/// Remove the pre-push hook
///
/// If a backup of the original hook exists (pre-push.gt-original),
/// it will be restored. Otherwise, the hook is simply deleted.
pub fn remove_pre_push_hook(repo_path: &Path) -> Result<()> {
    let hook_path = pre_push_hook_path(repo_path);
    let original_path = pre_push_original_path(repo_path);

    // Only remove if it's managed by gt
    if !is_hook_managed(repo_path)? {
        return Ok(());
    }

    // Remove the gt-managed hook
    if hook_path.exists() {
        fs::remove_file(&hook_path)?;
    }

    // Restore original if it exists
    if original_path.exists() {
        fs::rename(&original_path, &hook_path)?;
    }

    Ok(())
}

/// Check if the repository has a .git directory
pub fn is_git_repo(repo_path: &Path) -> bool {
    repo_path.join(".git").is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let git_dir = dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();
        dir
    }

    #[test]
    fn test_is_git_repo() {
        let temp_dir = setup_test_repo();
        assert!(is_git_repo(temp_dir.path()));

        let non_repo = TempDir::new().unwrap();
        assert!(!is_git_repo(non_repo.path()));
    }

    #[test]
    fn test_install_hook() {
        let temp_dir = setup_test_repo();
        let result = install_pre_push_hook(temp_dir.path());
        assert!(result.is_ok());

        let hook_path = pre_push_hook_path(temp_dir.path());
        assert!(hook_path.exists());

        let contents = fs::read_to_string(&hook_path).unwrap();
        assert!(contents.contains(GT_MARKER));
    }

    #[test]
    fn test_is_hook_managed() {
        let temp_dir = setup_test_repo();

        // No hook initially
        assert!(!is_hook_managed(temp_dir.path()).unwrap());

        // Install hook
        install_pre_push_hook(temp_dir.path()).unwrap();
        assert!(is_hook_managed(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_install_preserves_existing_hook() {
        let temp_dir = setup_test_repo();
        let hooks_dir = hooks_dir(temp_dir.path());
        fs::create_dir_all(&hooks_dir).unwrap();

        let hook_path = pre_push_hook_path(temp_dir.path());
        let original_content = "#!/bin/sh\necho 'original hook'\n";
        fs::write(&hook_path, original_content).unwrap();

        // Install gt hook
        install_pre_push_hook(temp_dir.path()).unwrap();

        // Original should be backed up
        let original_path = pre_push_original_path(temp_dir.path());
        assert!(original_path.exists());
        let backed_up = fs::read_to_string(&original_path).unwrap();
        assert_eq!(backed_up, original_content);

        // New hook should be gt-managed
        assert!(is_hook_managed(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_remove_hook_restores_original() {
        let temp_dir = setup_test_repo();
        let hooks_dir = hooks_dir(temp_dir.path());
        fs::create_dir_all(&hooks_dir).unwrap();

        let hook_path = pre_push_hook_path(temp_dir.path());
        let original_content = "#!/bin/sh\necho 'original hook'\n";
        fs::write(&hook_path, original_content).unwrap();

        // Install gt hook
        install_pre_push_hook(temp_dir.path()).unwrap();
        assert!(is_hook_managed(temp_dir.path()).unwrap());

        // Remove gt hook
        remove_pre_push_hook(temp_dir.path()).unwrap();

        // Original should be restored
        assert!(hook_path.exists());
        let restored = fs::read_to_string(&hook_path).unwrap();
        assert_eq!(restored, original_content);
        assert!(!is_hook_managed(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_idempotent_install() {
        let temp_dir = setup_test_repo();

        // Install hook twice
        install_pre_push_hook(temp_dir.path()).unwrap();
        install_pre_push_hook(temp_dir.path()).unwrap();

        // Should still be managed
        assert!(is_hook_managed(temp_dir.path()).unwrap());

        // Should not have created multiple backups
        let original_path = pre_push_original_path(temp_dir.path());
        assert!(!original_path.exists());
    }
}
