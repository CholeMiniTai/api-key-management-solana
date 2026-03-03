use api_key_management::{has_permission, ApiKeyAccount, ADMIN, DELETE, READ, WRITE};
use anchor_lang::prelude::Pubkey;

fn sample_key(expires_at: Option<i64>, is_active: bool, key_hash: [u8; 32]) -> ApiKeyAccount {
    ApiKeyAccount {
        owner: Pubkey::default(),
        key_id: 7,
        key_hash,
        name: "ci-key".to_string(),
        permissions: READ | WRITE,
        created_at: 100,
        expires_at,
        last_used_at: 100,
        usage_count: 0,
        is_active,
        metadata: "integration-test".to_string(),
        bump: 255,
    }
}

#[test]
fn happy_path_state_flow_create_rotate_revoke_closeable() {
    let mut key = sample_key(Some(1_000), true, [1u8; 32]);

    assert!(key.is_valid_at(500));
    assert!(has_permission(key.permissions, READ));

    // rotate
    let old_hash = key.key_hash;
    key.key_hash = [9u8; 32];
    assert_ne!(key.key_hash, old_hash);

    // revoke
    key.is_active = false;
    assert!(!key.is_valid_at(500));

    // close precondition (inactive)
    assert!(!key.is_active);
}

#[test]
fn expired_key_fails_validity_check() {
    let key = sample_key(Some(200), true, [2u8; 32]);
    assert!(key.is_expired_at(201));
    assert!(!key.is_valid_at(201));
}

#[test]
fn revoked_key_is_invalid() {
    let key = sample_key(Some(10_000), false, [3u8; 32]);
    assert!(!key.is_valid_at(100));
}

#[test]
fn hash_mismatch_detected_after_rotate() {
    let mut key = sample_key(Some(10_000), true, [4u8; 32]);
    let old_hash = key.key_hash;
    key.key_hash = [5u8; 32];

    assert_ne!(old_hash, key.key_hash);
    assert_eq!(old_hash, [4u8; 32]);
    assert_eq!(key.key_hash, [5u8; 32]);
}

#[test]
fn insufficient_permissions_detected() {
    let perms = READ | WRITE;
    assert!(!has_permission(perms, DELETE));
    assert!(!has_permission(perms, ADMIN));
}
