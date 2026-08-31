use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::MigrationHarness;
use tauri::test::MockRuntime;
use tauri::Manager;
use tempfile::TempDir;

use super::hosts::SaveConnectionInput;
use super::keys::{KeyInput, KeyUpdateInput};
use super::{init_pool, test_pool, DbPool, SqlitePragmas, MIGRATIONS};

#[derive(diesel::QueryableByName)]
struct JournalMode {
    #[diesel(sql_type = diesel::sql_types::Text)]
    journal_mode: String,
}

fn memory_pool() -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
    let pool = r2d2::Pool::builder()
        .max_size(1)
        .connection_customizer(Box::new(SqlitePragmas))
        .build(manager)
        .expect("failed to build in-memory pool");

    pool.get()
        .expect("failed to acquire in-memory connection")
        .run_pending_migrations(MIGRATIONS)
        .expect("failed to run migrations");

    pool
}

/// Wires a real file-backed pool into a mock Tauri app so the tests below drive
/// the actual commands, state injection included.
fn test_app() -> (tauri::App<MockRuntime>, TempDir) {
    let directory = tempfile::tempdir().expect("temp dir");
    let app = tauri::test::mock_app();
    app.manage(test_pool(&directory));
    (app, directory)
}

fn connection_input(name: &str) -> SaveConnectionInput {
    SaveConnectionInput {
        id: None,
        name: name.to_string(),
        host: format!("{name}.example.test"),
        port: 22,
        username: "deploy".to_string(),
        group_id: None,
        password: None,
        private_key: None,
        passphrase: None,
        key_name: None,
        existing_key_id: None,
    }
}

#[test]
fn migrations_apply_and_foreign_keys_cascade() {
    let pool = memory_pool();
    let mut connection = pool.get().expect("connection");
    let now = chrono::Utc::now().naive_utc();

    diesel::insert_into(orm::schema::ssh_keys::table)
        .values(&orm::models::NewSshKey {
            id: "key-1".to_string(),
            name: "key".to_string(),
            kind: "inline".to_string(),
            fingerprint: None,
            private_key: "PRIVATE".to_string(),
            passphrase: None,
            created_at: now,
            updated_at: now,
        })
        .execute(&mut connection)
        .expect("insert key");

    diesel::insert_into(orm::schema::ssh_hosts::table)
        .values(&orm::models::NewSshHost {
            id: "host-1".to_string(),
            name: "host".to_string(),
            host: "example.test".to_string(),
            port: 22,
            username: "root".to_string(),
            ssh_key_id: Some("key-1".to_string()),
            password: None,
            group_id: None,
            created_at: now,
            updated_at: now,
        })
        .execute(&mut connection)
        .expect("insert host");

    // Deleting a key must null the host's reference rather than fail or orphan it,
    // which only holds while `PRAGMA foreign_keys` is on for every pooled connection.
    diesel::delete(orm::schema::ssh_keys::table)
        .execute(&mut connection)
        .expect("delete key");

    let ssh_key_id = orm::schema::ssh_hosts::table
        .select(orm::schema::ssh_hosts::ssh_key_id)
        .first::<Option<String>>(&mut connection)
        .expect("host still present");
    assert_eq!(ssh_key_id, None);
}

#[test]
fn opening_a_fresh_database_applies_migrations_and_enables_wal() {
    let (app, _directory) = test_app();
    let pool = app.state::<DbPool>();
    let mut connection = pool.get().expect("connection");

    let mode = diesel::sql_query("PRAGMA journal_mode")
        .get_result::<JournalMode>(&mut connection)
        .expect("read journal mode");
    assert_eq!(mode.journal_mode, "wal");

    let hosts = orm::schema::ssh_hosts::table
        .count()
        .get_result::<i64>(&mut connection)
        .expect("migrated schema is queryable");
    assert_eq!(hosts, 0);
}

#[tokio::test]
async fn saving_a_connection_stores_its_key_and_password_for_reload() {
    let (app, _directory) = test_app();

    let saved = super::hosts::host_save(
        SaveConnectionInput {
            password: Some("  hunter2  ".to_string()),
            private_key: Some("PRIVATE KEY".to_string()),
            passphrase: Some("secret".to_string()),
            key_name: Some("prod key".to_string()),
            ..connection_input("prod")
        },
        app.state(),
    )
    .await
    .expect("save connection");

    let hosts = super::hosts::host_list(app.state())
        .await
        .expect("list hosts");
    assert_eq!(hosts.len(), 1);

    let reloaded = &hosts[0];
    assert_eq!(reloaded.id, saved.id);
    assert_eq!(reloaded.name, "prod");
    assert_eq!(reloaded.ssh_key_id, saved.ssh_key_id);
    match reloaded.auth.as_ref().expect("auth material") {
        shared::SshHostAuthMaterial::PublicKeyAndPassword {
            private_key,
            passphrase,
            password,
        } => {
            assert_eq!(private_key, "PRIVATE KEY");
            assert_eq!(passphrase.as_deref(), Some("secret"));
            assert_eq!(password, "hunter2");
        }
        other => panic!("unexpected auth material: {other:?}"),
    }

    let keys = super::keys::key_list(app.state()).await.expect("list keys");
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].name, "prod key");
}

#[tokio::test]
async fn editing_a_connection_reuses_its_key_instead_of_adding_another() {
    let (app, _directory) = test_app();

    let saved = super::hosts::host_save(
        SaveConnectionInput {
            private_key: Some("PRIVATE KEY".to_string()),
            ..connection_input("prod")
        },
        app.state(),
    )
    .await
    .expect("save connection");

    super::hosts::host_save(
        SaveConnectionInput {
            id: Some(saved.id.clone()),
            name: "prod renamed".to_string(),
            private_key: Some("ROTATED KEY".to_string()),
            existing_key_id: saved.ssh_key_id.clone(),
            ..connection_input("prod")
        },
        app.state(),
    )
    .await
    .expect("update connection");

    let hosts = super::hosts::host_list(app.state())
        .await
        .expect("list hosts");
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].name, "prod renamed");
    assert_eq!(hosts[0].ssh_key_id, saved.ssh_key_id);

    let keys = super::keys::key_list(app.state()).await.expect("list keys");
    assert_eq!(keys.len(), 1, "editing must not leave a second key behind");
}

#[tokio::test]
async fn deleting_a_connection_takes_its_inline_key_with_it() {
    let (app, _directory) = test_app();

    let saved = super::hosts::host_save(
        SaveConnectionInput {
            private_key: Some("PRIVATE KEY".to_string()),
            ..connection_input("prod")
        },
        app.state(),
    )
    .await
    .expect("save connection");

    super::hosts::host_delete(saved.id, saved.ssh_key_id, app.state())
        .await
        .expect("delete connection");

    assert!(super::hosts::host_list(app.state())
        .await
        .expect("list hosts")
        .is_empty());
    assert!(super::keys::key_list(app.state())
        .await
        .expect("list keys")
        .is_empty());
}

#[tokio::test]
async fn renaming_a_key_keeps_the_stored_private_key() {
    let (app, _directory) = test_app();

    let key = super::keys::key_create(
        KeyInput {
            name: "prod key".to_string(),
            kind: "inline".to_string(),
            fingerprint: None,
            private_key: "PRIVATE KEY".to_string(),
            passphrase: Some("secret".to_string()),
        },
        app.state(),
    )
    .await
    .expect("create key");

    super::keys::key_update(
        key.id.clone(),
        KeyUpdateInput {
            name: "renamed".to_string(),
            kind: "inline".to_string(),
            fingerprint: None,
            private_key: None,
            passphrase: None,
        },
        app.state(),
    )
    .await
    .expect("rename key");

    let secret = super::keys::key_secret(key.id, app.state())
        .await
        .expect("reveal secret");
    assert_eq!(secret.private_key, "PRIVATE KEY");
    assert_eq!(secret.passphrase.as_deref(), Some("secret"));
}

#[tokio::test]
async fn settings_upsert_by_key_instead_of_piling_up_rows() {
    let (app, _directory) = test_app();

    for value in ["first", "second"] {
        super::settings::set_setting(
            shared::Setting {
                key: "noverterm-config".to_string(),
                value: value.to_string(),
            },
            app.state(),
        )
        .await
        .expect("save setting");
    }

    let settings = super::settings::get_all_settings(app.state())
        .await
        .expect("list settings");
    assert_eq!(settings.len(), 1);
    assert_eq!(settings[0].value, "second");

    let single = super::settings::get_setting("noverterm-config".to_string(), app.state())
        .await
        .expect("get setting");
    assert_eq!(
        single.map(|setting| setting.value).as_deref(),
        Some("second")
    );
}

#[tokio::test]
async fn grouped_connections_and_snippets_reload_with_their_host() {
    let (app, _directory) = test_app();

    let group = super::groups::host_group_create("Production".to_string(), app.state())
        .await
        .expect("create group");

    let host = super::hosts::host_save(
        SaveConnectionInput {
            group_id: Some(group.id.clone()),
            ..connection_input("prod")
        },
        app.state(),
    )
    .await
    .expect("save connection");

    super::snippets::snippet_create(
        shared::SnippetWriteRequest {
            host_id: host.id.clone(),
            title: "restart".to_string(),
            body: "systemctl restart app".to_string(),
        },
        app.state(),
    )
    .await
    .expect("create snippet");

    let hosts = super::hosts::host_list(app.state())
        .await
        .expect("list hosts");
    assert_eq!(hosts[0].group_id.as_deref(), Some(group.id.as_str()));

    let snippets = super::snippets::snippet_list(app.state())
        .await
        .expect("list snippets");
    assert_eq!(snippets.len(), 1);
    assert_eq!(snippets[0].host_name, "prod");
    assert_eq!(snippets[0].body, "systemctl restart app");
}

#[tokio::test]
async fn a_reopened_database_still_has_everything() {
    let directory = tempfile::tempdir().expect("temp dir");
    let database_path = directory.path().join("noverterm.db");

    {
        let app = tauri::test::mock_app();
        app.manage(init_pool(&database_path).expect("open database"));
        super::hosts::host_save(
            SaveConnectionInput {
                password: Some("hunter2".to_string()),
                ..connection_input("prod")
            },
            app.state(),
        )
        .await
        .expect("save connection");
    }

    let app = tauri::test::mock_app();
    app.manage(init_pool(&database_path).expect("reopen database"));
    let hosts = super::hosts::host_list(app.state())
        .await
        .expect("list hosts");

    assert_eq!(hosts.len(), 1, "data must survive an app restart");
    assert!(matches!(
        hosts[0].auth.as_ref().expect("auth material"),
        shared::SshHostAuthMaterial::Password { password } if password == "hunter2"
    ));
}

fn mapping(bind_port: i32, target_port: i32) -> shared::PortForwardMappingInput {
    shared::PortForwardMappingInput {
        bind_host: "127.0.0.1".to_string(),
        bind_port,
        target_host: "127.0.0.1".to_string(),
        target_port,
    }
}

#[tokio::test]
async fn a_port_forward_preset_keeps_its_mappings_in_order_and_dies_with_its_host() {
    let (app, _directory) = test_app();

    let host = super::hosts::host_save(connection_input("prod"), app.state())
        .await
        .expect("save connection");

    let created = super::port_forwards::port_forward_preset_create(
        shared::PortForwardWriteRequest {
            name: "stack".to_string(),
            host_id: host.id.clone(),
            mappings: vec![mapping(8080, 80), mapping(5432, 5432), mapping(6379, 6379)],
        },
        app.state(),
    )
    .await
    .expect("create preset");

    let presets = super::port_forwards::port_forward_preset_list(app.state())
        .await
        .expect("list presets");
    assert_eq!(presets.len(), 1);
    assert_eq!(presets[0].host_name, "prod");
    assert_eq!(
        presets[0]
            .mappings
            .iter()
            .map(|mapping| mapping.bind_port)
            .collect::<Vec<_>>(),
        vec![8080, 5432, 6379]
    );

    // A save replaces the whole mapping set, so the dropped rows must not linger.
    super::port_forwards::port_forward_preset_update(
        created.id.clone(),
        shared::PortForwardWriteRequest {
            name: "stack".to_string(),
            host_id: host.id.clone(),
            mappings: vec![mapping(6379, 6379), mapping(8080, 80)],
        },
        app.state(),
    )
    .await
    .expect("update preset");

    let presets = super::port_forwards::port_forward_preset_list(app.state())
        .await
        .expect("list presets");
    assert_eq!(
        presets[0]
            .mappings
            .iter()
            .map(|mapping| mapping.bind_port)
            .collect::<Vec<_>>(),
        vec![6379, 8080]
    );

    let duplicate = super::port_forwards::port_forward_preset_create(
        shared::PortForwardWriteRequest {
            name: "clash".to_string(),
            host_id: host.id.clone(),
            mappings: vec![mapping(8080, 80), mapping(8080, 81)],
        },
        app.state(),
    )
    .await;
    assert!(
        duplicate.is_err(),
        "duplicate bind addresses must be rejected"
    );

    super::hosts::host_delete(host.id, None, app.state())
        .await
        .expect("delete host");

    let presets = super::port_forwards::port_forward_preset_list(app.state())
        .await
        .expect("list presets");
    assert!(presets.is_empty(), "presets cascade with their host");
}
