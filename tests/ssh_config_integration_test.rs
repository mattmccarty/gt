//! SSH config integration tests (in isolated temp directories)

mod common;

use common::TestEnv;
use gt::io::ssh_config::{SshConfig, SshHostEntry};

#[test]
fn test_ssh_config_roundtrip_in_temp_dir() {
    let env = TestEnv::new();

    // Create a config
    let mut config = SshConfig::default();
    config.upsert_host(
        SshHostEntry::new("gt-work.github.com")
            .with_hostname("github.com")
            .with_user("git")
            .with_identity_file("~/.ssh/id_gt_work")
            .with_identities_only(true)
            .with_preferred_auth("publickey"),
    );

    // Save to isolated temp directory
    let ssh_config_path = env.ssh_dir.join("config");
    config.save(&ssh_config_path).expect("Failed to save SSH config");

    // Verify file exists in temp directory (NOT real system)
    assert!(ssh_config_path.exists());
    assert!(ssh_config_path.starts_with(&env.home), "SAFETY: Must be in temp directory");

    // Load it back
    let loaded = SshConfig::load(&ssh_config_path).expect("Failed to load SSH config");

    // Verify content
    assert_eq!(loaded.hosts.len(), 1);
    let host = loaded.get_host("gt-work.github.com").unwrap();
    assert_eq!(host.hostname, Some("github.com".to_string()));
    assert_eq!(host.user, Some("git".to_string()));
    assert_eq!(host.identity_file, Some("~/.ssh/id_gt_work".to_string()));
    assert_eq!(host.identities_only, Some(true));
    assert_eq!(host.preferred_auth, Some("publickey".to_string()));

    println!("✓ SSH config roundtrip test passed in isolated environment");
}

#[test]
fn test_ssh_config_upsert_in_temp_dir() {
    let env = TestEnv::new();

    // Create initial config
    env.create_ssh_config(
        r#"Host *
  AddKeysToAgent yes
  IdentitiesOnly yes

Host existing.example.com
  HostName example.com
  User existinguser
"#,
    );

    let ssh_config_path = env.ssh_dir.join("config");

    // Load existing config
    let mut config = SshConfig::load(&ssh_config_path).unwrap();
    assert_eq!(config.hosts.len(), 2); // * and existing.example.com

    // Add a new gt entry
    config.upsert_host(
        SshHostEntry::new("gt-work.github.com")
            .with_hostname("github.com")
            .with_user("git")
            .with_identity_file("~/.ssh/id_gt_work")
            .with_identities_only(true),
    );

    // Save back
    config.save(&ssh_config_path).unwrap();

    // Reload and verify
    let reloaded = SshConfig::load(&ssh_config_path).unwrap();
    assert_eq!(reloaded.hosts.len(), 3);
    assert!(reloaded.has_host("gt-work.github.com"));
    assert!(reloaded.has_host("existing.example.com"));

    println!("✓ SSH config upsert test passed in isolated environment");
}

#[test]
fn test_find_gt_hosts_in_temp_dir() {
    let env = TestEnv::new();

    // Create config with multiple gt entries
    env.create_ssh_config(
        r#"Host gt-work.github.com
  HostName github.com
  User git

Host gt-personal.github.com
  HostName github.com
  User git

Host gt-company.gitlab.com
  HostName gitlab.com
  User git

Host other.example.com
  HostName example.com
  User other
"#,
    );

    let ssh_config_path = env.ssh_dir.join("config");
    let config = SshConfig::load(&ssh_config_path).unwrap();

    // Find all gt hosts
    let gt_hosts = config.find_gt_hosts("gt");
    assert_eq!(gt_hosts.len(), 3);

    let host_names: Vec<String> = gt_hosts.iter().map(|h| h.host.clone()).collect();
    assert!(host_names.contains(&"gt-work.github.com".to_string()));
    assert!(host_names.contains(&"gt-personal.github.com".to_string()));
    assert!(host_names.contains(&"gt-company.gitlab.com".to_string()));
    assert!(!host_names.contains(&"other.example.com".to_string()));

    println!("✓ Find gt hosts test passed in isolated environment");
}

#[test]
fn test_update_existing_host_in_temp_dir() {
    let env = TestEnv::new();

    // Create initial config
    env.create_ssh_config(
        r#"Host gt-work.github.com
  HostName github.com
  User olduser
  IdentityFile ~/.ssh/old_key
"#,
    );

    let ssh_config_path = env.ssh_dir.join("config");

    // Load and update
    let mut config = SshConfig::load(&ssh_config_path).unwrap();
    assert_eq!(config.hosts.len(), 1);

    // Update the entry
    config.upsert_host(
        SshHostEntry::new("gt-work.github.com")
            .with_hostname("github.com")
            .with_user("newuser")
            .with_identity_file("~/.ssh/id_gt_work")
            .with_identities_only(true),
    );

    // Should still have 1 host (updated, not added)
    assert_eq!(config.hosts.len(), 1);

    // Save and reload
    config.save(&ssh_config_path).unwrap();
    let reloaded = SshConfig::load(&ssh_config_path).unwrap();

    let host = reloaded.get_host("gt-work.github.com").unwrap();
    assert_eq!(host.user, Some("newuser".to_string()));
    assert_eq!(host.identity_file, Some("~/.ssh/id_gt_work".to_string()));
    assert_eq!(host.identities_only, Some(true));

    println!("✓ Update existing host test passed in isolated environment");
}

#[test]
fn test_remove_host_in_temp_dir() {
    let env = TestEnv::new();

    // Create config with multiple hosts
    env.create_ssh_config(
        r#"Host gt-work.github.com
  HostName github.com

Host gt-personal.github.com
  HostName github.com

Host keep-this.example.com
  HostName example.com
"#,
    );

    let ssh_config_path = env.ssh_dir.join("config");

    // Load, remove, and save
    let mut config = SshConfig::load(&ssh_config_path).unwrap();
    assert_eq!(config.hosts.len(), 3);

    let removed = config.remove_host("gt-work.github.com");
    assert!(removed.is_some());
    assert_eq!(config.hosts.len(), 2);

    config.save(&ssh_config_path).unwrap();

    // Reload and verify
    let reloaded = SshConfig::load(&ssh_config_path).unwrap();
    assert_eq!(reloaded.hosts.len(), 2);
    assert!(!reloaded.has_host("gt-work.github.com"));
    assert!(reloaded.has_host("gt-personal.github.com"));
    assert!(reloaded.has_host("keep-this.example.com"));

    println!("✓ Remove host test passed in isolated environment");
}

#[test]
#[cfg(unix)]
fn test_ssh_config_permissions_unix() {
    use std::os::unix::fs::PermissionsExt;

    let env = TestEnv::new();

    let mut config = SshConfig::default();
    config.upsert_host(SshHostEntry::new("test.com").with_hostname("example.com"));

    let ssh_config_path = env.ssh_dir.join("config");
    config.save(&ssh_config_path).unwrap();

    // Verify permissions are 0600 (owner read/write only)
    let metadata = std::fs::metadata(&ssh_config_path).unwrap();
    let permissions = metadata.permissions();
    let mode = permissions.mode();

    assert_eq!(mode & 0o777, 0o600, "SSH config should have 0600 permissions");

    println!("✓ SSH config permissions test passed");
}

#[test]
fn test_empty_config_handling() {
    let env = TestEnv::new();

    // Create empty SSH config
    env.create_ssh_config("");

    let ssh_config_path = env.ssh_dir.join("config");
    let config = SshConfig::load(&ssh_config_path).unwrap();

    assert_eq!(config.hosts.len(), 0);
    assert!(config.global.is_empty());

    // Add a host and save
    let mut config = config;
    config.upsert_host(SshHostEntry::new("gt-work.github.com").with_hostname("github.com"));

    config.save(&ssh_config_path).unwrap();

    // Verify
    let reloaded = SshConfig::load(&ssh_config_path).unwrap();
    assert_eq!(reloaded.hosts.len(), 1);

    println!("✓ Empty config handling test passed");
}

#[test]
fn test_corrupt_ssh_config_detection() {
    let env = TestEnv::new();

    // Create corrupt SSH config with orphaned host-specific directives
    // (like what happened during incomplete cleanup)
    let corrupt_config = r#"
# Some global settings
PreferredAuthentications publickey

# Missing Host line here! The directives below are orphaned
    HostName github.com
    User git
    IdentityFile ~/.ssh/id_gt_work
    IdentitiesOnly yes
    PreferredAuthentications publickey

# This host is valid
Host gt-personal.github.com
    HostName github.com
    User git
    IdentityFile ~/.ssh/id_gt_personal
"#;

    env.create_ssh_config(corrupt_config);

    let ssh_config_path = env.ssh_dir.join("config");
    let config = SshConfig::load(&ssh_config_path).unwrap();

    // Should have warnings about orphaned directives
    assert!(config.has_warnings(), "Should detect orphaned directives");

    let warnings = config.get_warnings();
    assert!(!warnings.is_empty(), "Should have at least one warning");

    // Check that we detected the orphaned HostName, User, and IdentityFile
    let has_hostname_warning = warnings.iter().any(|w| w.directive == "HostName");
    let has_user_warning = warnings.iter().any(|w| w.directive == "User");
    let has_identityfile_warning = warnings.iter().any(|w| w.directive == "IdentityFile");
    let has_identitiesonly_warning = warnings.iter().any(|w| w.directive == "IdentitiesOnly");

    assert!(has_hostname_warning, "Should warn about orphaned HostName");
    assert!(has_user_warning, "Should warn about orphaned User");
    assert!(has_identityfile_warning, "Should warn about orphaned IdentityFile");
    assert!(has_identitiesonly_warning, "Should warn about orphaned IdentitiesOnly");

    // Verify warning messages are helpful
    for warning in warnings {
        assert!(warning.message.contains("outside of any Host block"),
            "Warning should explain the issue");
        assert!(warning.message.contains("corrupted"),
            "Warning should mention corruption");
        assert!(warning.line_number > 0, "Should have valid line number");
    }

    // Should still parse the valid host
    assert_eq!(config.hosts.len(), 1, "Should still parse valid host");
    assert!(config.has_host("gt-personal.github.com"), "Should have the valid host");

    println!("✓ Corrupt SSH config detection test passed");
    println!("  Detected {} warnings for orphaned directives", warnings.len());
}
