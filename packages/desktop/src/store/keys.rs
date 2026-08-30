use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use orm::models::{NewSshKey, SshKey, UpdateSshKey};
use orm::schema::ssh_keys;
use shared::{SshKeyRecord, SshKeySecret};
use uuid::Uuid;

use super::{internal_error, run_db, DbPool};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct KeyInput {
    pub name: String,
    pub kind: String,
    pub fingerprint: Option<String>,
    pub private_key: String,
    pub passphrase: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn key_list(pool: tauri::State<'_, DbPool>) -> Result<Vec<SshKeyRecord>, String> {
    let pool = pool.inner().clone();
    run_db(pool, |connection| {
        ssh_keys::table
            .order(ssh_keys::created_at.asc())
            .load::<SshKey>(connection)
            .map(|keys| keys.into_iter().map(to_record).collect())
            .map_err(internal_error)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn key_create(
    key: KeyInput,
    pool: tauri::State<'_, DbPool>,
) -> Result<SshKeyRecord, String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        let now = Utc::now().naive_utc();
        diesel::insert_into(ssh_keys::table)
            .values(&NewSshKey {
                id: Uuid::new_v4().to_string(),
                name: key.name,
                kind: key.kind,
                fingerprint: key.fingerprint,
                private_key: key.private_key,
                passphrase: key.passphrase,
                created_at: now,
                updated_at: now,
            })
            .get_result::<SshKey>(connection)
            .map(to_record)
            .map_err(internal_error)
    })
    .await
}

/// `private_key` is optional so a rename keeps the stored key; supplying one
/// rotates it, and the passphrase then follows the new key rather than the old.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct KeyUpdateInput {
    pub name: String,
    pub kind: String,
    pub fingerprint: Option<String>,
    pub private_key: Option<String>,
    pub passphrase: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn key_update(
    id: String,
    key: KeyUpdateInput,
    pool: tauri::State<'_, DbPool>,
) -> Result<SshKeyRecord, String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        let existing = ssh_keys::table
            .filter(ssh_keys::id.eq(&id))
            .first::<SshKey>(connection)
            .optional()
            .map_err(internal_error)?
            .ok_or_else(|| "ssh key not found".to_string())?;

        let rotating_private_key = key.private_key.is_some();
        diesel::update(ssh_keys::table.filter(ssh_keys::id.eq(&id)))
            .set(UpdateSshKey {
                name: key.name,
                kind: key.kind,
                fingerprint: key.fingerprint.or(existing.fingerprint),
                private_key: key.private_key.unwrap_or(existing.private_key),
                passphrase: if rotating_private_key {
                    key.passphrase
                } else {
                    existing.passphrase
                },
                updated_at: Utc::now().naive_utc(),
            })
            .get_result::<SshKey>(connection)
            .map(to_record)
            .map_err(internal_error)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn key_delete(id: String, pool: tauri::State<'_, DbPool>) -> Result<(), String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        diesel::delete(ssh_keys::table.filter(ssh_keys::id.eq(id)))
            .execute(connection)
            .map(|_| ())
            .map_err(internal_error)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn key_secret(
    id: String,
    pool: tauri::State<'_, DbPool>,
) -> Result<SshKeySecret, String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        ssh_keys::table
            .filter(ssh_keys::id.eq(id))
            .first::<SshKey>(connection)
            .optional()
            .map_err(internal_error)?
            .map(|key| SshKeySecret {
                private_key: key.private_key,
                passphrase: key.passphrase,
            })
            .ok_or_else(|| "ssh key not found".to_string())
    })
    .await
}

/// Shared by `host_save`: writes the key a connection form supplied inline,
/// replacing the existing row when the connection already had one.
pub(super) fn upsert_inline_key(
    connection: &mut SqliteConnection,
    existing_key_id: Option<String>,
    name: String,
    private_key: String,
    passphrase: Option<String>,
    now: NaiveDateTime,
) -> Result<String, String> {
    match existing_key_id {
        Some(id) => diesel::update(ssh_keys::table.filter(ssh_keys::id.eq(&id)))
            .set(UpdateSshKey {
                name,
                kind: "inline".to_string(),
                fingerprint: None,
                private_key,
                passphrase,
                updated_at: now,
            })
            .get_result::<SshKey>(connection)
            .optional()
            .map_err(internal_error)?
            .map(|key| key.id)
            .ok_or_else(|| "ssh key not found".to_string()),
        None => diesel::insert_into(ssh_keys::table)
            .values(&NewSshKey {
                id: Uuid::new_v4().to_string(),
                name,
                kind: "inline".to_string(),
                fingerprint: None,
                private_key,
                passphrase,
                created_at: now,
                updated_at: now,
            })
            .get_result::<SshKey>(connection)
            .map(|key| key.id)
            .map_err(internal_error),
    }
}

fn to_record(key: SshKey) -> SshKeyRecord {
    SshKeyRecord {
        id: key.id,
        name: key.name,
        kind: key.kind,
        fingerprint: key.fingerprint,
    }
}
