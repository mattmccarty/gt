//! Safety tests to verify we NEVER touch real system files

mod common;

use common::TestEnv;

#[test]
fn test_isolation_from_real_system() {
    let env = TestEnv::new();

    // Verify we're in a temp directory
    let home_str = env.home.to_str().unwrap();
    assert!(
        home_str.contains("tmp") || home_str.contains("temp") || home_str.contains("Temp"),
        "Test environment MUST be in a temp directory, got: {}",
        home_str
    );

    // Verify it's NOT the real home
    let real_home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .expect("No HOME or USERPROFILE found");

    assert_ne!(
        env.home.to_str().unwrap(),
        real_home,
        "CRITICAL SAFETY VIOLATION: Test is using real home directory!"
    );

    println!("✓ SAFETY CHECK PASSED: Using isolated temp directory");
    println!("  Real home: {}", real_home);
    println!("  Test home: {}", env.home.display());
}

#[test]
fn test_ssh_config_is_isolated() {
    let env = TestEnv::new();

    // Create SSH config
    env.create_ssh_config("Test content");

    // Verify the path is in temp directory
    let ssh_config = env.ssh_dir.join("config");
    assert!(
        ssh_config.starts_with(&env.home),
        "SSH config MUST be in temp directory"
    );

    // Verify real SSH config is NOT modified
    let real_ssh_config = dirs::home_dir()
        .unwrap()
        .join(".ssh/config");

    if real_ssh_config.exists() {
        let real_content = std::fs::read_to_string(&real_ssh_config).unwrap();
        assert!(
            !real_content.contains("Test content"),
            "CRITICAL: Real SSH config was modified!"
        );
    }

    println!("✓ SAFETY CHECK PASSED: SSH config is isolated");
}

#[test]
fn test_ssh_keys_are_fake_and_isolated() {
    let env = TestEnv::new();

    // Create fake SSH key
    let key_path = env.create_ssh_key("id_gt_test");

    // Verify it's in temp directory
    assert!(
        key_path.starts_with(&env.ssh_dir),
        "SSH key MUST be in temp directory"
    );

    // Verify it contains fake data (not real crypto material)
    let key_content = std::fs::read_to_string(&key_path).unwrap();
    assert!(
        key_content.contains("FAKE"),
        "SSH key must be marked as FAKE"
    );

    println!("✓ SAFETY CHECK PASSED: SSH keys are fake and isolated");
}

#[test]
fn test_git_repos_are_isolated() {
    let env = TestEnv::new();

    // Create test repo
    let repo = env.create_repo("test-repo", Some("git@github.com:test/test.git"));

    // Verify it's in temp directory
    assert!(
        repo.starts_with(&env.home),
        "Git repo MUST be in temp directory"
    );

    println!("✓ SAFETY CHECK PASSED: Git repos are isolated");
}

#[test]
fn test_temp_cleanup_on_drop() {
    let temp_path = {
        let env = TestEnv::new();
        let path = env.home.clone();

        // Verify it exists while env is alive
        assert!(path.exists(), "Temp directory should exist");

        path
    }; // env is dropped here

    // After drop, temp directory should be cleaned up
    // Note: This might not work immediately on Windows
    #[cfg(unix)]
    {
        // Give it a moment for cleanup
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(
            !temp_path.exists(),
            "Temp directory should be cleaned up after TestEnv is dropped"
        );
        println!("✓ SAFETY CHECK PASSED: Temp directory cleaned up on drop");
    }

    #[cfg(windows)]
    {
        println!("✓ SAFETY CHECK: Temp cleanup verified on Unix (Windows cleanup may be delayed)");
    }
}
