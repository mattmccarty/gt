//! Common test utilities
//!
//! ⚠️  SAFETY GUARANTEE: This module NEVER touches real system files.
//! All tests use isolated temporary directories that are automatically cleaned up.

use std::path::PathBuf;
use tempfile::TempDir;

/// A test environment with isolated directories
///
/// SAFETY: This creates a completely isolated environment using tempfile::TempDir.
/// It will NEVER touch your real ~/.ssh/config, ~/.gitconfig, or SSH keys.
/// All files are created in a temporary directory that is automatically deleted.
pub struct TestEnv {
    /// Temporary directory (dropped when TestEnv is dropped)
    _temp_dir: TempDir,
    /// Home directory for this test
    pub home: PathBuf,
    /// SSH directory
    pub ssh_dir: PathBuf,
    /// Config directory for gt
    pub config_dir: PathBuf,
}

impl TestEnv {
    /// Create a new test environment with isolated directories
    ///
    /// SAFETY: Verifies the created path is NOT the real home directory
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let home = temp_dir.path().to_owned();

        // SAFETY CHECK: Verify we're NOT using the real home directory
        let real_home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok();
        if let Some(ref real_home_path) = real_home {
            assert_ne!(
                home.to_str().unwrap(),
                real_home_path,
                "SAFETY VIOLATION: Test env must not use real home directory!"
            );
        }

        let ssh_dir = home.join(".ssh");
        let config_dir = home.join(".config").join("gt");

        std::fs::create_dir_all(&ssh_dir).expect("Failed to create SSH dir");
        std::fs::create_dir_all(&config_dir).expect("Failed to create config dir");

        Self {
            _temp_dir: temp_dir,
            home,
            ssh_dir,
            config_dir,
        }
    }

    /// Create an SSH config file with content (in isolated temp directory)
    pub fn create_ssh_config(&self, content: &str) -> PathBuf {
        let path = self.ssh_dir.join("config");
        // SAFETY: Path is guaranteed to be in temp directory
        assert!(
            path.starts_with(&self.home),
            "SAFETY: Path must be in temp directory"
        );
        std::fs::write(&path, content).expect("Failed to write SSH config");
        path
    }

    /// Create a gt config file with content (in isolated temp directory)
    pub fn create_gt_config(&self, content: &str) -> PathBuf {
        let path = self.config_dir.join("config.toml");
        // SAFETY: Path is guaranteed to be in temp directory
        assert!(
            path.starts_with(&self.home),
            "SAFETY: Path must be in temp directory"
        );
        std::fs::write(&path, content).expect("Failed to write gt config");
        path
    }

    /// Read SSH config content (from isolated temp directory)
    pub fn read_ssh_config(&self) -> String {
        let path = self.ssh_dir.join("config");
        std::fs::read_to_string(&path).unwrap_or_default()
    }

    /// Read gt config content (from isolated temp directory)
    pub fn read_gt_config(&self) -> String {
        let path = self.config_dir.join("config.toml");
        std::fs::read_to_string(&path).unwrap_or_default()
    }

    /// Create a mock SSH key (FAKE key for testing only, in isolated temp directory)
    pub fn create_ssh_key(&self, name: &str) -> PathBuf {
        let key_path = self.ssh_dir.join(name);
        let pub_path = self.ssh_dir.join(format!("{}.pub", name));

        // SAFETY: Verify paths are in temp directory
        assert!(
            key_path.starts_with(&self.home),
            "SAFETY: Key must be in temp directory"
        );
        assert!(
            pub_path.starts_with(&self.home),
            "SAFETY: Key must be in temp directory"
        );

        // Write FAKE keys (not real cryptographic material)
        std::fs::write(&key_path, "-----BEGIN OPENSSH PRIVATE KEY-----\nFAKE_TEST_KEY_NOT_REAL\n-----END OPENSSH PRIVATE KEY-----\n")
            .expect("Failed to write key");
        std::fs::write(
            &pub_path,
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAFAKETEST test@test",
        )
        .expect("Failed to write public key");

        // Set proper permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&key_path)
                .expect("Failed to get metadata")
                .permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&key_path, perms).expect("Failed to set permissions");
        }

        key_path
    }

    /// Create a mock Git repository (in isolated temp directory)
    pub fn create_repo(&self, name: &str, remote_url: Option<&str>) -> PathBuf {
        let repo_path = self.home.join(name);
        let git_dir = repo_path.join(".git");

        // SAFETY: Verify path is in temp directory
        assert!(
            repo_path.starts_with(&self.home),
            "SAFETY: Repo must be in temp directory"
        );

        std::fs::create_dir_all(&git_dir).expect("Failed to create repo");

        // Write minimal git config
        let git_config = if let Some(url) = remote_url {
            format!("[core]\n\trepositoryformatversion = 0\n[remote \"origin\"]\n\turl = {}\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n", url)
        } else {
            "[core]\n\trepositoryformatversion = 0\n".to_string()
        };

        std::fs::write(git_dir.join("config"), git_config).expect("Failed to write git config");
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n")
            .expect("Failed to write HEAD");

        repo_path
    }
}

impl Default for TestEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// Sample SSH config content (for testing)
pub const SAMPLE_SSH_CONFIG: &str = r#"Host *
  AddKeysToAgent yes
  IdentitiesOnly yes

Host gt-work.github.com
  HostName github.com
  User git
  IdentityFile ~/.ssh/id_gt_work
  IdentitiesOnly yes
  PreferredAuthentications publickey

Host gt-personal.github.com
  HostName github.com
  User git
  IdentityFile ~/.ssh/id_gt_personal
  IdentitiesOnly yes
  PreferredAuthentications publickey
"#;

/// Sample gt config content (TOML format for testing)
pub const SAMPLE_GT_CONFIG: &str = r#"[gt]
strategy = "ssh-alias"
prefix = "gt"

[[identities]]
name = "work"
email = "work@company.com"
user_name = "Work User"
provider = "github"
ssh_key = "~/.ssh/id_gt_work"
strategy = "ssh-alias"

[[identities]]
name = "personal"
email = "personal@email.com"
user_name = "Personal User"
provider = "github"
ssh_key = "~/.ssh/id_gt_personal"
strategy = "ssh-alias"
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_is_isolated() {
        let env = TestEnv::new();

        // Verify the environment is in a temp directory
        assert!(
            env.home.to_str().unwrap().contains("tmp")
                || env.home.to_str().unwrap().contains("temp"),
            "Test environment should be in a temp directory"
        );

        // Verify it's NOT the real home
        let real_home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok();
        if let Some(real_home_path) = real_home {
            assert_ne!(
                env.home.to_str().unwrap(),
                real_home_path,
                "Test must NOT use real home directory!"
            );
        }
    }

    #[test]
    fn test_ssh_config_operations() {
        let env = TestEnv::new();

        let path = env.create_ssh_config("test content");
        assert!(path.exists());
        assert_eq!(env.read_ssh_config(), "test content");

        // Verify it's in temp directory
        assert!(path.starts_with(&env.home));
    }

    #[test]
    fn test_fake_ssh_key_creation() {
        let env = TestEnv::new();

        let key_path = env.create_ssh_key("id_gt_test");
        assert!(key_path.exists());
        assert!(key_path.with_extension("pub").exists());

        // Verify it's in temp directory
        assert!(key_path.starts_with(&env.ssh_dir));

        // Verify it contains fake data
        let content = std::fs::read_to_string(&key_path).unwrap();
        assert!(content.contains("FAKE_TEST_KEY"));
    }

    #[test]
    fn test_repo_creation() {
        let env = TestEnv::new();

        let repo = env.create_repo("test-repo", Some("git@github.com:user/test.git"));
        assert!(repo.exists());
        assert!(repo.join(".git").exists());
        assert!(repo.join(".git/config").exists());

        // Verify it's in temp directory
        assert!(repo.starts_with(&env.home));

        // Verify git config has the remote
        let git_config = std::fs::read_to_string(repo.join(".git/config")).unwrap();
        assert!(git_config.contains("git@github.com:user/test.git"));
    }
}
