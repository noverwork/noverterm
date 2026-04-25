use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use chrono::Utc;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use uuid::Uuid;

use crate::models::{NewSshHost, NewSshKey, NewUser, NewUserSetting};
use crate::schema::{ssh_hosts, ssh_keys, user_settings, users};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../migrator/migrations");

static DATABASE_READY: OnceLock<()> = OnceLock::new();

fn database_url() -> String {
    if let Ok(value) = env::var("DATABASE_URL") {
        return value;
    }

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../migrator/.env");
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    contents
        .lines()
        .find_map(|line| line.strip_prefix("DATABASE_URL="))
        .map(str::to_string)
        .unwrap_or_else(|| panic!("DATABASE_URL missing from {}", path.display()))
}

fn connection() -> PgConnection {
    let url = database_url();
    let connection = PgConnection::establish(&url)
        .unwrap_or_else(|error| panic!("failed to connect to test database: {error}"));

    DATABASE_READY.get_or_init(|| {
        let mut setup_connection = PgConnection::establish(&url)
            .unwrap_or_else(|error| panic!("failed to connect for migrations: {error}"));
        setup_connection
            .run_pending_migrations(MIGRATIONS)
            .unwrap_or_else(|error| panic!("failed to run test migrations: {error}"));
    });

    connection
}

fn insert_test_user(connection: &mut PgConnection, id: &str, now: chrono::NaiveDateTime) {
    diesel::insert_into(users::table)
        .values(&NewUser {
            id: id.to_string(),
            email: format!("{id}@example.test"),
            password_hash: "test-password-hash".to_string(),
            created_at: now,
            updated_at: now,
        })
        .execute(connection)
        .expect("test user insert should succeed");
}

#[test]
fn owner_scoped_host_foreign_keys_reject_cross_user_key_references() {
    let mut connection = connection();
    let now = Utc::now().naive_utc();
    let key_id = format!("key-{}", Uuid::new_v4());
    let owner_id = format!("alice-{}", Uuid::new_v4());
    let other_owner_id = format!("bob-{}", Uuid::new_v4());

    insert_test_user(&mut connection, &owner_id, now);
    insert_test_user(&mut connection, &other_owner_id, now);

    diesel::insert_into(ssh_keys::table)
        .values(&NewSshKey {
            id: key_id.clone(),
            owner_id: owner_id.clone(),
            name: "Primary key".to_string(),
            kind: "ed25519".to_string(),
            fingerprint: Some("SHA256:test".to_string()),
            encrypted_private_key: "encrypted-private-key".to_string(),
            encrypted_passphrase: Some("encrypted-passphrase".to_string()),
            created_at: now,
            updated_at: now,
        })
        .execute(&mut connection)
        .expect("key insert should succeed");

    let result = diesel::insert_into(ssh_hosts::table)
        .values(&NewSshHost {
            id: format!("host-{}", Uuid::new_v4()),
            owner_id: other_owner_id,
            name: "Production".to_string(),
            host: "prod.example.com".to_string(),
            port: 22,
            username: "deploy".to_string(),
            ssh_key_id: Some(key_id),
            encrypted_password: None,
            created_at: now,
            updated_at: now,
        })
        .execute(&mut connection);

    assert!(result.is_err(), "cross-owner key reference should fail");
}

#[test]
fn user_settings_allow_same_key_for_different_owners() {
    let mut connection = connection();
    let now = Utc::now().naive_utc();
    let setting_key = format!("terminal-font-size-{}", Uuid::new_v4());
    let alice_id = format!("alice-{}", Uuid::new_v4());
    let bob_id = format!("bob-{}", Uuid::new_v4());

    insert_test_user(&mut connection, &alice_id, now);
    insert_test_user(&mut connection, &bob_id, now);

    diesel::insert_into(user_settings::table)
        .values(&vec![
            NewUserSetting {
                id: format!("setting-{}", Uuid::new_v4()),
                owner_id: alice_id,
                key: setting_key.clone(),
                value: "14".to_string(),
                created_at: now,
                updated_at: now,
            },
            NewUserSetting {
                id: format!("setting-{}", Uuid::new_v4()),
                owner_id: bob_id,
                key: setting_key.clone(),
                value: "16".to_string(),
                created_at: now,
                updated_at: now,
            },
        ])
        .execute(&mut connection)
        .expect("settings inserts should succeed");

    let count = user_settings::table
        .filter(user_settings::key.eq(setting_key))
        .count()
        .get_result::<i64>(&mut connection)
        .expect("count query should succeed");

    assert_eq!(count, 2);
}
