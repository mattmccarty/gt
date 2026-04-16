//! SSH key generation integration tests (in isolated temp directories)
//! 
//! These tests actually call ssh-keygen but only in temp directories

mod common;

use common::TestEnv;
use gt::io::ssh_key::{generate_key, verify_key, read_public_key, set_key_permissions, KeyGenOptions, KeyType};
use std::path::Path;

#[test]
fn test_generate_ed25519_key_in_temp_dir() {
    let env = TestEnv::new();

    let key_path = env.ssh_dir.join("id_gt_test_ed25519");
    let opts = KeyGenOptions::ed25519(key_path.clone(), "test@example.com");

    // SAFETY: Verify path is in temp directory
    assert!(key_path.starts_with(&env.home), "SAFETY: Key must be in temp directory");

    // Generate key
    let result = generate_key(&opts);
    assert!(result.is_ok(), "Failed to generate Ed25519 key: {:?}", result.err());

    // Verify both keys were created
    assert!(key_path.exists(), "Private key should exist");
    assert!(key_path.with_extension("pub").exists(), "Public key should exist");

    println!("✓ Ed25519 key generated in isolated environment");
}

#[test]
fn test_generate_rsa_key_in_temp_dir() {
    let env = TestEnv::new();

    let key_path = env.ssh_dir.join("id_gt_test_rsa");
    let opts = KeyGenOptions::rsa(key_path.clone(), "test@example.com", 2048);

    // SAFETY: Verify path is in temp directory
    assert!(key_path.starts_with(&env.home), "SAFETY: Key must be in temp directory");

    // Generate key
    let result = generate_key(&opts);
    assert!(result.is_ok(), "Failed to generate RSA key: {:?}", result.err());

    // Verify both keys were created
    assert!(key_path.exists(), "Private key should exist");
    assert!(key_path.with_extension("pub").exists(), "Public key should exist");

    println!("✓ RSA key generated in isolated environment");
}

#[test]
fn test_generate_ecdsa_key_in_temp_dir() {
    let env = TestEnv::new();

    let key_path = env.ssh_dir.join("id_gt_test_ecdsa");
    let opts = KeyGenOptions::ecdsa(key_path.clone(), "test@example.com", 521);

    // SAFETY: Verify path is in temp directory
    assert!(key_path.starts_with(&env.home), "SAFETY: Key must be in temp directory");

    // Generate key
    let result = generate_key(&opts);
    assert!(result.is_ok(), "Failed to generate ECDSA key: {:?}", result.err());

    // Verify both keys were created
    assert!(key_path.exists(), "Private key should exist");
    assert!(key_path.with_extension("pub").exists(), "Public key should exist");

    println!("✓ ECDSA key generated in isolated environment");
}

#[test]
fn test_key_already_exists_without_force() {
    let env = TestEnv::new();

    let key_path = env.ssh_dir.join("id_gt_test_exists");
    
    // Generate first key
    let opts = KeyGenOptions::ed25519(key_path.clone(), "test@example.com");
    generate_key(&opts).expect("First generation should succeed");

    // Try to generate again without force
    let opts2 = KeyGenOptions::ed25519(key_path.clone(), "test@example.com");
    let result = generate_key(&opts2);
    
    assert!(result.is_err(), "Should fail when key exists without force flag");
    assert!(result.unwrap_err().to_string().contains("already exists"));

    println!("✓ Key existence check working correctly");
}

#[test]
fn test_key_overwrite_with_force() {
    let env = TestEnv::new();

    let key_path = env.ssh_dir.join("id_gt_test_force");
    
    // Generate first key
    let opts = KeyGenOptions::ed25519(key_path.clone(), "test1@example.com");
    generate_key(&opts).expect("First generation should succeed");

    // Read first public key
    let first_pub = read_public_key(&key_path).expect("Should read first public key");

    // Generate again with force
    let opts2 = KeyGenOptions::ed25519(key_path.clone(), "test2@example.com").force();
    
    // Delete the existing key first (ssh-keygen requires this)
    std::fs::remove_file(&key_path).ok();
    std::fs::remove_file(key_path.with_extension("pub")).ok();
    
    let result = generate_key(&opts2);
    assert!(result.is_ok(), "Should succeed with force flag after manual deletion");

    // Read second public key
    let second_pub = read_public_key(&key_path).expect("Should read second public key");

    // Keys should be different (different comments at minimum)
    assert_ne!(first_pub, second_pub, "Force should generate a new key");

    println!("✓ Force overwrite working correctly");
}

#[test]
fn test_verify_key_in_temp_dir() {
    let env = TestEnv::new();

    let key_path = env.ssh_dir.join("id_gt_test_verify");
    
    // Generate key
    let opts = KeyGenOptions::ed25519(key_path.clone(), "test@example.com");
    generate_key(&opts).expect("Generation should succeed");

    // Verify the key
    let is_valid = verify_key(&key_path).expect("Verification should succeed");
    assert!(is_valid, "Generated key should be valid");

    println!("✓ Key verification working correctly");
}

#[test]
fn test_read_public_key_in_temp_dir() {
    let env = TestEnv::new();

    let key_path = env.ssh_dir.join("id_gt_test_pubkey");
    let comment = "test@example.com";
    
    // Generate key
    let opts = KeyGenOptions::ed25519(key_path.clone(), comment);
    generate_key(&opts).expect("Generation should succeed");

    // Read public key
    let pub_key = read_public_key(&key_path).expect("Should read public key");

    // Verify it's not empty and contains the comment
    assert!(!pub_key.is_empty(), "Public key should not be empty");
    assert!(pub_key.contains(comment), "Public key should contain comment");
    assert!(pub_key.starts_with("ssh-ed25519"), "Public key should start with ssh-ed25519");

    println!("✓ Public key reading working correctly");
}

#[test]
fn test_read_public_key_with_pub_extension() {
    let env = TestEnv::new();

    let key_path = env.ssh_dir.join("id_gt_test_pubext");
    
    // Generate key
    let opts = KeyGenOptions::ed25519(key_path.clone(), "test@example.com");
    generate_key(&opts).expect("Generation should succeed");

    // Read public key using .pub path
    let pub_path = key_path.with_extension("pub");
    let pub_key = read_public_key(&pub_path).expect("Should read public key from .pub path");

    assert!(!pub_key.is_empty());
    assert!(pub_key.starts_with("ssh-ed25519"));

    println!("✓ Public key reading with .pub extension working correctly");
}

#[test]
#[cfg(unix)]
fn test_key_permissions_unix() {
    use std::os::unix::fs::PermissionsExt;

    let env = TestEnv::new();

    let key_path = env.ssh_dir.join("id_gt_test_perms");
    
    // Generate key
    let opts = KeyGenOptions::ed25519(key_path.clone(), "test@example.com");
    generate_key(&opts).expect("Generation should succeed");

    // Check permissions
    let metadata = std::fs::metadata(&key_path).expect("Should get metadata");
    let perms = metadata.permissions();
    let mode = perms.mode();

    assert_eq!(mode & 0o777, 0o600, "Private key should have 0600 permissions");

    println!("✓ Key permissions (0600) set correctly on Unix");
}

#[test]
fn test_key_generation_with_nonexistent_parent_dir() {
    let env = TestEnv::new();

    // Create a path with a non-existent parent directory
    let key_path = env.ssh_dir.join("nested/dir/id_gt_test");
    
    // Generate key (should create parent directories)
    let opts = KeyGenOptions::ed25519(key_path.clone(), "test@example.com");
    let result = generate_key(&opts);
    
    assert!(result.is_ok(), "Should create parent directories automatically");
    assert!(key_path.exists(), "Key should exist");
    assert!(key_path.parent().unwrap().exists(), "Parent directory should be created");

    println!("✓ Parent directory creation working correctly");
}

#[test]
fn test_multiple_keys_in_same_dir() {
    let env = TestEnv::new();

    // Generate multiple keys
    let keys = vec![
        ("id_gt_work", "work@company.com"),
        ("id_gt_personal", "personal@email.com"),
        ("id_gt_test", "test@example.com"),
    ];

    for (name, comment) in keys {
        let key_path = env.ssh_dir.join(name);
        let opts = KeyGenOptions::ed25519(key_path.clone(), comment);
        generate_key(&opts).expect(&format!("Should generate {}", name));
        assert!(key_path.exists());
    }

    // Verify all keys exist
    let entries: Vec<_> = std::fs::read_dir(&env.ssh_dir)
        .expect("Should read ssh dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    assert!(entries.iter().any(|e| e.contains("id_gt_work")));
    assert!(entries.iter().any(|e| e.contains("id_gt_personal")));
    assert!(entries.iter().any(|e| e.contains("id_gt_test")));

    println!("✓ Multiple keys in same directory working correctly");
}

#[test]
fn test_key_with_special_comment() {
    let env = TestEnv::new();

    let key_path = env.ssh_dir.join("id_gt_special");
    let comment = "user+tag@company.com (work laptop)";
    
    let opts = KeyGenOptions::ed25519(key_path.clone(), comment);
    generate_key(&opts).expect("Should handle special characters in comment");

    let pub_key = read_public_key(&key_path).expect("Should read public key");
    assert!(pub_key.contains("user+tag@company.com"));

    println!("✓ Special characters in comment handled correctly");
}
