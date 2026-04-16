//! Integration tests for `gt id add` command
//!
//! Tests the complete workflow of adding identities

mod common;

use common::TestEnv;
use gt::cli::args::{AddOpts, KeyTypeArg};
use gt::cmd::{add, Context};
use gt::io::toml_config::GtConfig;

/// Create a test context with isolated environment
fn create_test_context(env: &TestEnv) -> Context {
    let config_path = env.config_dir.join("config.toml");

    Context {
        config: None, // Will be created by add command
        config_path,
        output_format: gt::cli::args::OutputFormat::Terminal,
        verbosity: 0,
        quiet: true,
        dry_run: false,
        force: false,
        auto: false,
        all: false,
        no_color: true,
    }
}

#[test]
fn test_add_identity_basic() {
    let env = TestEnv::new();
    let ctx = create_test_context(&env);

    // SAFETY: Verify config path is in temp directory
    assert!(
        ctx.config_path.starts_with(&env.home),
        "SAFETY: Config must be in temp directory"
    );

    let opts = AddOpts {
        name: "work".to_string(),
        email: Some("work@company.com".to_string()),
        user_name: Some("Work User".to_string()),
        provider: "github".to_string(),
        strategy: None, // Will use default (ssh-alias)
        key: None,
        key_type: KeyTypeArg::Ed25519,
        no_key: true, // Skip key generation for basic test
        host: None,
        scope: None,
        directory: None,
    };

    let result = add::execute(&opts, &ctx);
    assert!(result.is_ok(), "Failed to add identity: {:?}", result.err());

    // Verify config was created
    assert!(ctx.config_path.exists(), "Config file should be created");

    // Load and verify config
    let config = GtConfig::load(&ctx.config_path).expect("Failed to load config");
    assert!(config.identities.contains_key("work"), "Identity should be in config");

    let identity = config.get_identity("work").expect("Should get identity");
    assert_eq!(identity.email, "work@company.com");
    assert_eq!(identity.name, "Work User");
    assert_eq!(identity.provider.to_lowercase(), "github");

    println!("✓ Basic identity added successfully");
}

#[test]
fn test_add_identity_with_ssh_config() {
    let env = TestEnv::new();

    // SAFETY: Override HOME to use temp directory
    std::env::set_var("HOME", &env.home);

    let ctx = create_test_context(&env);

    // Create SSH directory in temp
    std::fs::create_dir_all(&env.ssh_dir).expect("Failed to create SSH dir");

    let opts = AddOpts {
        name: "personal".to_string(),
        email: Some("personal@email.com".to_string()),
        user_name: Some("Personal User".to_string()),
        provider: "github".to_string(),
        strategy: None,
        key: None,
        key_type: KeyTypeArg::Ed25519,
        no_key: false, // Will generate SSH key
        host: None,
        scope: None,
        directory: None,
    };

    let result = add::execute(&opts, &ctx);
    assert!(result.is_ok(), "Failed to add identity with SSH: {:?}", result.err());

    // Verify config
    let config = GtConfig::load(&ctx.config_path).expect("Failed to load config");
    let identity = config.get_identity("personal").expect("Should get identity");

    assert!(identity.ssh.is_some(), "SSH config should be present");
    let ssh = identity.ssh.as_ref().unwrap();
    assert_eq!(ssh.key_type, Some("ed25519".to_string()));

    // Verify SSH key was generated in temp directory
    let key_path = env.ssh_dir.join("id_gt_personal");
    assert!(key_path.exists(), "SSH key should be generated in temp dir");
    assert!(key_path.with_extension("pub").exists(), "Public key should exist");

    // Verify SSH config was created in temp directory
    let ssh_config_path = env.ssh_dir.join("config");
    assert!(ssh_config_path.exists(), "SSH config should be created");

    println!("✓ Identity with SSH config added successfully");
}

#[test]
fn test_add_identity_duplicate_name() {
    let env = TestEnv::new();
    let ctx = create_test_context(&env);

    // First add
    let opts1 = AddOpts {
        name: "work".to_string(),
        email: Some("work@company.com".to_string()),
        user_name: Some("Work User".to_string()),
        provider: "github".to_string(),
        strategy: None,
        key: None,
        key_type: KeyTypeArg::Ed25519,
        no_key: true,
        host: None,
        scope: None,
        directory: None,
    };

    add::execute(&opts1, &ctx).expect("First add should succeed");

    // Try to add again with same name
    let ctx2 = Context {
        config: Some(GtConfig::load(&ctx.config_path).unwrap()),
        ..ctx
    };

    let opts2 = AddOpts {
        name: "work".to_string(), // Same name
        email: Some("other@company.com".to_string()),
        user_name: Some("Other User".to_string()),
        provider: "github".to_string(),
        strategy: None,
        key: None,
        key_type: KeyTypeArg::Ed25519,
        no_key: true,
        host: None,
        scope: None,
        directory: None,
    };

    let result = add::execute(&opts2, &ctx2);
    assert!(result.is_err(), "Should fail on duplicate name");

    let err = result.unwrap_err();
    assert!(err.to_string().contains("already exists"), "Error should mention duplicate");

    println!("✓ Duplicate identity detection working");
}

#[test]
fn test_add_identity_missing_email() {
    let env = TestEnv::new();
    let ctx = create_test_context(&env);

    let opts = AddOpts {
        name: "work".to_string(),
        email: None, // Missing email
        user_name: Some("Work User".to_string()),
        provider: "github".to_string(),
        strategy: None,
        key: None,
        key_type: KeyTypeArg::Ed25519,
        no_key: true,
        host: None,
        scope: None,
        directory: None,
    };

    let result = add::execute(&opts, &ctx);
    assert!(result.is_err(), "Should fail without email");

    let err = result.unwrap_err();
    assert!(err.to_string().contains("email"), "Error should mention email");

    println!("✓ Email validation working");
}

#[test]
fn test_add_identity_missing_user_name() {
    let env = TestEnv::new();
    let ctx = create_test_context(&env);

    let opts = AddOpts {
        name: "work".to_string(),
        email: Some("work@company.com".to_string()),
        user_name: None, // Missing user_name
        provider: "github".to_string(),
        strategy: None,
        key: None,
        key_type: KeyTypeArg::Ed25519,
        no_key: true,
        host: None,
        scope: None,
        directory: None,
    };

    let result = add::execute(&opts, &ctx);
    assert!(result.is_err(), "Should fail without user_name");

    let err = result.unwrap_err();
    assert!(err.to_string().contains("user name"), "Error should mention user name");

    println!("✓ User name validation working");
}

#[test]
fn test_add_identity_invalid_name() {
    let env = TestEnv::new();
    let ctx = create_test_context(&env);

    let opts = AddOpts {
        name: "gt-work".to_string(), // Invalid: contains "gt-" prefix
        email: Some("work@company.com".to_string()),
        user_name: Some("Work User".to_string()),
        provider: "github".to_string(),
        strategy: None,
        key: None,
        key_type: KeyTypeArg::Ed25519,
        no_key: true,
        host: None,
        scope: None,
        directory: None,
    };

    let result = add::execute(&opts, &ctx);
    assert!(result.is_err(), "Should fail with invalid name");

    let err = result.unwrap_err();
    assert!(err.to_string().contains("gt-"), "Error should mention reserved prefix");

    println!("✓ Identity name validation working");
}

#[test]
fn test_add_identity_dry_run() {
    let env = TestEnv::new();
    let mut ctx = create_test_context(&env);
    ctx.dry_run = true;

    let opts = AddOpts {
        name: "work".to_string(),
        email: Some("work@company.com".to_string()),
        user_name: Some("Work User".to_string()),
        provider: "github".to_string(),
        strategy: None,
        key: None,
        key_type: KeyTypeArg::Ed25519,
        no_key: true,
        host: None,
        scope: None,
        directory: None,
    };

    let result = add::execute(&opts, &ctx);
    assert!(result.is_ok(), "Dry run should succeed");

    // Verify config was NOT created
    assert!(!ctx.config_path.exists(), "Config file should not be created in dry run");

    println!("✓ Dry run mode working");
}

#[test]
fn test_add_multiple_identities() {
    let env = TestEnv::new();
    let ctx = create_test_context(&env);

    // Add first identity
    let opts1 = AddOpts {
        name: "work".to_string(),
        email: Some("work@company.com".to_string()),
        user_name: Some("Work User".to_string()),
        provider: "github".to_string(),
        strategy: None,
        key: None,
        key_type: KeyTypeArg::Ed25519,
        no_key: true,
        host: None,
        scope: None,
        directory: None,
    };

    add::execute(&opts1, &ctx).expect("First identity should be added");

    // Add second identity
    let config_path = ctx.config_path.clone();
    let ctx2 = Context {
        config: Some(GtConfig::load(&ctx.config_path).unwrap()),
        ..ctx
    };

    let opts2 = AddOpts {
        name: "personal".to_string(),
        email: Some("personal@email.com".to_string()),
        user_name: Some("Personal User".to_string()),
        provider: "github".to_string(),
        strategy: None,
        key: None,
        key_type: KeyTypeArg::Rsa,
        no_key: true,
        host: None,
        scope: None,
        directory: None,
    };

    add::execute(&opts2, &ctx2).expect("Second identity should be added");

    // Verify both identities are in config
    let config = GtConfig::load(&config_path).expect("Failed to load config");
    assert!(config.identities.contains_key("work"), "Work identity should exist");
    assert!(config.identities.contains_key("personal"), "Personal identity should exist");
    assert_eq!(config.identities.len(), 2, "Should have 2 identities");

    println!("✓ Multiple identities added successfully");
}

#[test]
fn test_add_identity_with_custom_key_path() {
    let env = TestEnv::new();

    // SAFETY: Override HOME to use temp directory
    std::env::set_var("HOME", &env.home);

    // Create SSH directory in temp
    std::fs::create_dir_all(&env.ssh_dir).expect("Failed to create SSH dir");

    let ctx = create_test_context(&env);

    let custom_key = env.ssh_dir.join("custom_key");

    let opts = AddOpts {
        name: "custom".to_string(),
        email: Some("custom@example.com".to_string()),
        user_name: Some("Custom User".to_string()),
        provider: "github".to_string(),
        strategy: None,
        key: Some(custom_key.clone()),
        key_type: KeyTypeArg::Ed25519,
        no_key: false,
        host: None,
        scope: None,
        directory: None,
    };

    let result = add::execute(&opts, &ctx);
    assert!(result.is_ok(), "Should add identity with custom key path");

    // Verify config has custom key path
    let config = GtConfig::load(&ctx.config_path).expect("Failed to load config");
    let identity = config.get_identity("custom").expect("Should get identity");
    let ssh = identity.ssh.as_ref().expect("Should have SSH config");
    assert!(
        ssh.key_path.as_ref().unwrap().contains("custom_key"),
        "Should use custom key path"
    );

    // Verify SSH key was generated with custom path in temp directory
    assert!(custom_key.exists(), "Custom SSH key should be generated");
    assert!(custom_key.with_extension("pub").exists(), "Public key should exist");

    println!("✓ Custom key path working");
}
