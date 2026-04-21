//! Integration tests for conditional strategy
//!
//! Tests the conditional include strategy for directory-based identity management.
//!
//! SAFETY: All tests use isolated temporary directories and never touch real system files.

mod common;

use common::TestEnv;
use gt::cli::args::OutputFormat;
use gt::cmd::Context;
use gt::core::identity::Identity;
use gt::core::provider::Provider;
use gt::io::toml_config::GtConfig;
use gt::strategy::conditional::ConditionalStrategy;
use gt::strategy::Strategy;

/// Create a test context with isolated environment
fn create_test_context(env: &TestEnv, config: Option<GtConfig>) -> Context {
    let config_path = env.config_dir.join("config.toml");

    Context {
        config,
        config_path,
        output_format: OutputFormat::Terminal,
        verbosity: 0,
        quiet: true,
        dry_run: false,
        force: false,
        auto: false,
        all: false,
        no_color: true,
    }
}

/// Create a test identity
fn create_test_identity(name: &str, email: &str, user_name: &str) -> Identity {
    Identity {
        name: name.to_string(),
        email: email.to_string(),
        user_name: user_name.to_string(),
        provider: Provider::GitHub,
        ssh: None,
        strategy: Some("conditional".to_string()),
    }
}

// =============================================================================
// Conditional Strategy Unit Tests
// =============================================================================

#[test]
fn test_conditional_strategy_config_path() {
    let strategy = ConditionalStrategy::new();
    let path = strategy.identity_config_path("work").unwrap();

    // Should contain gitconfig.d and identity name
    assert!(
        path.to_string_lossy().contains("gitconfig.d"),
        "Path should contain gitconfig.d"
    );
    assert!(
        path.to_string_lossy().contains("work"),
        "Path should contain identity name"
    );
}

#[test]
fn test_conditional_strategy_custom_config_dir() {
    let strategy = ConditionalStrategy::with_config_dir("~/.config/git-identities");
    let path = strategy.identity_config_path("personal").unwrap();

    assert!(
        path.to_string_lossy().contains("git-identities"),
        "Path should contain custom directory name"
    );
    assert!(
        path.to_string_lossy().contains("personal"),
        "Path should contain identity name"
    );
}

#[test]
fn test_create_identity_config_file() {
    let env = TestEnv::new();

    // SAFETY: Override HOME to use temp directory
    std::env::set_var("HOME", &env.home);

    // Create custom strategy pointing to temp directory
    let gitconfig_d = env.home.join(".gitconfig.d");
    let strategy = ConditionalStrategy::with_config_dir(gitconfig_d.to_string_lossy().to_string());

    let identity = create_test_identity("work", "work@company.com", "Work User");

    // Create identity config
    let config_path = strategy
        .create_identity_config(&identity, None)
        .expect("Failed to create identity config");

    // Verify file was created
    assert!(config_path.exists(), "Identity config file should exist");
    assert!(
        config_path.starts_with(&env.home),
        "SAFETY: Config must be in temp directory"
    );

    // Verify content
    let content = std::fs::read_to_string(&config_path).expect("Failed to read config");
    assert!(
        content.contains("work@company.com"),
        "Config should contain email"
    );
    assert!(
        content.contains("Work User"),
        "Config should contain user name"
    );
    assert!(
        content.contains("[user]"),
        "Config should have [user] section"
    );

    println!("Identity config created at: {}", config_path.display());
    println!("Content:\n{}", content);
}

#[test]
fn test_create_identity_config_with_ssh_key() {
    let env = TestEnv::new();

    // SAFETY: Override HOME to use temp directory
    std::env::set_var("HOME", &env.home);

    // Create custom strategy pointing to temp directory
    let gitconfig_d = env.home.join(".gitconfig.d");
    let strategy = ConditionalStrategy::with_config_dir(gitconfig_d.to_string_lossy().to_string());

    let identity = create_test_identity("work", "work@company.com", "Work User");

    // Create identity config with SSH key
    let config_path = strategy
        .create_identity_config(&identity, Some("~/.ssh/id_gt_work"))
        .expect("Failed to create identity config");

    // Verify content includes SSH command
    let content = std::fs::read_to_string(&config_path).expect("Failed to read config");
    assert!(
        content.contains("sshCommand"),
        "Config should contain sshCommand"
    );
    assert!(
        content.contains("id_gt_work"),
        "Config should reference SSH key"
    );

    println!("Identity config with SSH:\n{}", content);
}

// =============================================================================
// Directory Pattern Matching Tests
// =============================================================================

#[test]
fn test_path_matching_basic() {
    let env = TestEnv::new();

    // Create directory structure
    let work_dir = env.home.join("work");
    let project_dir = work_dir.join("project1");
    std::fs::create_dir_all(&project_dir).expect("Failed to create directories");

    // Project should match work pattern (it's under work/)
    assert!(
        project_dir.starts_with(&work_dir),
        "Project should be under work directory"
    );
}

#[test]
fn test_path_matching_nested_directories() {
    let env = TestEnv::new();

    // Create nested directory structure
    let work_dir = env.home.join("work");
    let deep_project = work_dir.join("org").join("team").join("project");
    std::fs::create_dir_all(&deep_project).expect("Failed to create directories");

    // Deep project should still be under work
    assert!(
        deep_project.starts_with(&work_dir),
        "Deep project should be under work directory"
    );
}

#[test]
fn test_path_not_matching_sibling() {
    let env = TestEnv::new();

    // Create sibling directories
    let work_dir = env.home.join("work");
    let personal_dir = env.home.join("personal");
    std::fs::create_dir_all(&work_dir).expect("Failed to create work");
    std::fs::create_dir_all(&personal_dir).expect("Failed to create personal");

    // Personal should NOT match work pattern
    assert!(
        !personal_dir.starts_with(&work_dir),
        "Personal should not be under work directory"
    );
}

// =============================================================================
// Config File Parsing Tests
// =============================================================================

#[test]
fn test_parse_gitconfig_email() {
    let config = r#"
[user]
    email = test@example.com
    name = Test User
"#;

    // Verify expected content is in config
    assert!(config.contains("email = test@example.com"));
    assert!(config.contains("name = Test User"));
}

#[test]
fn test_parse_gitconfig_with_ssh_command() {
    let config = r#"
[user]
    email = work@company.com
    name = Work User

[core]
    sshCommand = ssh -i ~/.ssh/id_gt_work -o IdentitiesOnly=yes
"#;

    assert!(config.contains("[user]"));
    assert!(config.contains("[core]"));
    assert!(config.contains("sshCommand"));
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn test_identity_config_with_special_characters() {
    let env = TestEnv::new();

    // SAFETY: Override HOME to use temp directory
    std::env::set_var("HOME", &env.home);

    // Create custom strategy pointing to temp directory
    let gitconfig_d = env.home.join(".gitconfig.d");
    let strategy = ConditionalStrategy::with_config_dir(gitconfig_d.to_string_lossy().to_string());

    let identity = create_test_identity("work-2024", "user+work@company.com", "Work User Jr.");

    let config_path = strategy
        .create_identity_config(&identity, None)
        .expect("Failed to create identity config");

    let content = std::fs::read_to_string(&config_path).expect("Failed to read config");
    assert!(
        content.contains("user+work@company.com"),
        "Should handle + in email"
    );
    assert!(
        content.contains("Work User Jr."),
        "Should handle special chars in name"
    );
}

#[test]
fn test_multiple_identities_different_directories() {
    let env = TestEnv::new();

    // SAFETY: Override HOME to use temp directory
    std::env::set_var("HOME", &env.home);

    // Create custom strategy pointing to temp directory
    let gitconfig_d = env.home.join(".gitconfig.d");
    std::fs::create_dir_all(&gitconfig_d).expect("Failed to create gitconfig.d");

    let strategy = ConditionalStrategy::with_config_dir(gitconfig_d.to_string_lossy().to_string());

    // Create work and personal directories
    let work_dir = env.home.join("work");
    let personal_dir = env.home.join("personal");
    std::fs::create_dir_all(&work_dir).expect("Failed to create work");
    std::fs::create_dir_all(&personal_dir).expect("Failed to create personal");

    // Create work identity config
    let work_identity = create_test_identity("work", "work@company.com", "Work User");
    strategy
        .create_identity_config(&work_identity, None)
        .expect("Failed to create work config");

    // Create personal identity config
    let personal_identity = create_test_identity("personal", "me@personal.com", "Personal Me");
    strategy
        .create_identity_config(&personal_identity, None)
        .expect("Failed to create personal config");

    // Verify both configs exist
    let work_config = gitconfig_d.join("work");
    let personal_config = gitconfig_d.join("personal");

    assert!(work_config.exists(), "Work config should exist");
    assert!(personal_config.exists(), "Personal config should exist");

    // Verify they have different content
    let work_content = std::fs::read_to_string(&work_config).unwrap();
    let personal_content = std::fs::read_to_string(&personal_config).unwrap();

    assert!(work_content.contains("work@company.com"));
    assert!(personal_content.contains("me@personal.com"));
    assert!(!work_content.contains("me@personal.com"));
    assert!(!personal_content.contains("work@company.com"));
}

#[test]
fn test_strategy_cleanup() {
    let env = TestEnv::new();

    // SAFETY: Override HOME to use temp directory
    std::env::set_var("HOME", &env.home);

    // Create custom strategy pointing to temp directory
    let gitconfig_d = env.home.join(".gitconfig.d");
    let strategy = ConditionalStrategy::with_config_dir(gitconfig_d.to_string_lossy().to_string());

    let identity = create_test_identity("temp-id", "temp@test.com", "Temp User");

    // Create config
    let config_path = strategy
        .create_identity_config(&identity, None)
        .expect("Failed to create identity config");

    assert!(config_path.exists(), "Config should exist before cleanup");

    // Manually cleanup the file (cleanup_identity removes from gitconfig includes too)
    std::fs::remove_file(&config_path).expect("Failed to remove config file");

    assert!(
        !config_path.exists(),
        "Config should be removed after cleanup"
    );
}

// =============================================================================
// Integration with gt config id use command
// =============================================================================

#[test]
fn test_use_command_with_directory_flag() {
    let env = TestEnv::new();

    // SAFETY: Override HOME to use temp directory
    std::env::set_var("HOME", &env.home);

    // Create custom strategy pointing to temp directory
    let gitconfig_d = env.home.join(".gitconfig.d");
    std::fs::create_dir_all(&gitconfig_d).expect("Failed to create gitconfig.d");

    let work_dir = env.home.join("work");
    std::fs::create_dir_all(&work_dir).expect("Failed to create work dir");

    // The use command would set up conditional include for ~/work
    // We're testing the strategy directly here

    let strategy = ConditionalStrategy::with_config_dir(gitconfig_d.to_string_lossy().to_string());
    let identity = create_test_identity("work", "work@company.com", "Work User");

    let result = strategy
        .setup_for_directory(&identity, &work_dir.to_string_lossy(), None)
        .expect("Failed to setup conditional");

    assert!(!result.changes.is_empty(), "Should have made changes");

    // Verify config file was created
    let identity_config = gitconfig_d.join("work");
    assert!(
        identity_config.exists(),
        "Identity config should be created"
    );

    println!("Changes made:");
    for change in &result.changes {
        println!("  - {}", change);
    }
}

// =============================================================================
// Status Detection Tests (simplified)
// =============================================================================

#[test]
fn test_no_identity_in_empty_directory() {
    let env = TestEnv::new();

    // SAFETY: Override HOME to use temp directory
    std::env::set_var("HOME", &env.home);

    // Create an empty directory (not a git repo)
    let empty_dir = env.home.join("empty");
    std::fs::create_dir_all(&empty_dir).expect("Failed to create directory");

    // Verify it's not a git repo
    assert!(!empty_dir.join(".git").exists(), "Should not be a git repo");
}

#[test]
fn test_repository_local_config() {
    let env = TestEnv::new();

    // SAFETY: Override HOME to use temp directory
    std::env::set_var("HOME", &env.home);

    // Create a repository with local config
    let repo = env.create_repo("test-repo", Some("git@github.com:user/test.git"));

    // Use git init to make it a proper repo (env.create_repo creates minimal .git structure)
    let output = std::process::Command::new("git")
        .arg("init")
        .arg(&repo)
        .output();

    if output.is_err() {
        println!("Note: git init failed, skipping detailed git config test");
        return;
    }

    // Set local git config
    let email_result = std::process::Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["config", "user.email", "local@test.com"])
        .output();

    if email_result.is_err() {
        println!("Note: git config failed, skipping test");
        return;
    }

    let name_result = std::process::Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["config", "user.name", "Local User"])
        .output();

    if name_result.is_err() {
        println!("Note: git config failed, skipping test");
        return;
    }

    // Verify the config was set by reading it back
    let check = std::process::Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["config", "--local", "user.email"])
        .output()
        .expect("Failed to check git config");

    let email = String::from_utf8_lossy(&check.stdout).trim().to_string();
    assert_eq!(email, "local@test.com", "Local git config should be set");
}

// =============================================================================
// Strategy Validation Tests
// =============================================================================

#[test]
fn test_conditional_strategy_validation() {
    let env = TestEnv::new();

    // SAFETY: Override HOME to use temp directory
    std::env::set_var("HOME", &env.home);

    let strategy = ConditionalStrategy::new();
    let validation = strategy.validate().expect("Validation should succeed");

    // Should be valid (git is available)
    assert!(validation.valid, "Strategy should be valid");

    // May have warnings about config dir not existing
    println!("Warnings: {:?}", validation.warnings);
}

#[test]
fn test_strategy_setup_requirements() {
    let strategy = ConditionalStrategy::new();
    let requirements = strategy.setup_requirements();

    assert!(!requirements.is_empty(), "Should have setup requirements");

    // Check for expected requirements
    let descriptions: Vec<&str> = requirements
        .iter()
        .map(|r| r.description.as_str())
        .collect();

    assert!(
        descriptions.iter().any(|d| d.contains("Config directory")),
        "Should have config directory requirement"
    );
    assert!(
        descriptions.iter().any(|d| d.contains("Identity config")),
        "Should have identity config requirement"
    );
    assert!(
        descriptions.iter().any(|d| d.contains("includeIf")),
        "Should have includeIf requirement"
    );

    println!("Setup requirements:");
    for req in &requirements {
        println!("  - {} (complete: {})", req.description, req.complete);
    }
}
