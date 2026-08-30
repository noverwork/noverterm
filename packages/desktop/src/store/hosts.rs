use chrono::Utc;
use diesel::prelude::*;
use orm::models::{NewSshHost, SshHost, SshKey, UpdateSshHost};
use orm::schema::{ssh_hosts, ssh_keys};
use shared::{SshHostAuthMaterial, SshHostRecord};
use uuid::Uuid;

use super::{internal_error, run_db, DbPool};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SaveConnectionInput {
    pub id: Option<String>,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    #[serde(default)]
    pub group_id: Option<String>,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub passphrase: Option<String>,
    pub key_name: Option<String>,
    pub existing_key_id: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn host_list(pool: tauri::State<'_, DbPool>) -> Result<Vec<SshHostRecord>, String> {
    let pool = pool.inner().clone();
    run_db(pool, |connection| {
        let hosts = ssh_hosts::table
            .order((ssh_hosts::group_id.asc(), ssh_hosts::name.asc()))
            .load::<SshHost>(connection)
            .map_err(internal_error)?;
        let keys = ssh_keys::table
            .load::<SshKey>(connection)
            .map_err(internal_error)?;

        Ok(hosts
            .into_iter()
            .map(|host| to_record(&keys, host))
            .collect())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn host_save(
    connection: SaveConnectionInput,
    pool: tauri::State<'_, DbPool>,
) -> Result<SshHostRecord, String> {
    let pool = pool.inner().clone();
    run_db(pool, move |db| {
        let now = Utc::now().naive_utc();
        let private_key = trimmed(connection.private_key);
        let passphrase = trimmed(connection.passphrase);
        let password = trimmed(connection.password);
        let existing_key_id = trimmed(connection.existing_key_id);
        let key_name =
            trimmed(connection.key_name).unwrap_or_else(|| format!("{} key", connection.name));

        let ssh_key_id = match private_key {
            Some(private_key) => Some(super::keys::upsert_inline_key(
                db,
                existing_key_id,
                key_name,
                private_key,
                passphrase,
                now,
            )?),
            None => existing_key_id,
        };

        let group_id = trimmed(connection.group_id);
        let host = match connection.id {
            Some(id) => diesel::update(ssh_hosts::table.filter(ssh_hosts::id.eq(id)))
                .set(UpdateSshHost {
                    name: connection.name,
                    host: connection.host,
                    port: connection.port,
                    username: connection.username,
                    ssh_key_id: Some(ssh_key_id),
                    password: Some(password),
                    group_id: Some(group_id),
                    updated_at: now,
                })
                .get_result::<SshHost>(db)
                .optional()
                .map_err(internal_error)?
                .ok_or_else(|| "host not found".to_string())?,
            None => diesel::insert_into(ssh_hosts::table)
                .values(&NewSshHost {
                    id: Uuid::new_v4().to_string(),
                    name: connection.name,
                    host: connection.host,
                    port: connection.port,
                    username: connection.username,
                    ssh_key_id,
                    password,
                    group_id,
                    created_at: now,
                    updated_at: now,
                })
                .get_result::<SshHost>(db)
                .map_err(internal_error)?,
        };

        let keys = ssh_keys::table.load::<SshKey>(db).map_err(internal_error)?;
        Ok(to_record(&keys, host))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn host_delete(
    id: String,
    ssh_key_id: Option<String>,
    pool: tauri::State<'_, DbPool>,
) -> Result<(), String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        diesel::delete(ssh_hosts::table.filter(ssh_hosts::id.eq(id)))
            .execute(connection)
            .map_err(internal_error)?;

        if let Some(ssh_key_id) = ssh_key_id {
            diesel::delete(ssh_keys::table.filter(ssh_keys::id.eq(ssh_key_id)))
                .execute(connection)
                .map_err(internal_error)?;
        }

        Ok(())
    })
    .await
}

fn to_record(keys: &[SshKey], host: SshHost) -> SshHostRecord {
    let key = host
        .ssh_key_id
        .as_ref()
        .and_then(|id| keys.iter().find(|key| &key.id == id));

    let auth = match (key, host.password.clone()) {
        (Some(key), Some(password)) => Some(SshHostAuthMaterial::PublicKeyAndPassword {
            private_key: key.private_key.clone(),
            passphrase: key.passphrase.clone(),
            password,
        }),
        (Some(key), None) => Some(SshHostAuthMaterial::PublicKey {
            private_key: key.private_key.clone(),
            passphrase: key.passphrase.clone(),
        }),
        (None, Some(password)) => Some(SshHostAuthMaterial::Password { password }),
        (None, None) => None,
    };

    SshHostRecord {
        id: host.id,
        name: host.name,
        host: host.host,
        port: host.port,
        username: host.username,
        ssh_key_id: host.ssh_key_id,
        group_id: host.group_id,
        auth,
    }
}

fn trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
