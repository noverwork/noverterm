use chrono::Utc;
use diesel::prelude::*;
use orm::models::{NewSshHost, SshHost, SshKey, UpdateSshHost};
use orm::schema::{ssh_hosts, ssh_keys};
use shared::{AiAccess, SshHostAuthMaterial, SshHostRecord};
use uuid::Uuid;

use super::{internal_error, run_db, DbPool};

/// Most bastions a saved host may tunnel through.
const MAX_JUMP_HOPS: usize = 5;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaveConnectionInput {
    pub id: Option<String>,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub jump_host_id: Option<String>,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub passphrase: Option<String>,
    pub key_name: Option<String>,
    pub existing_key_id: Option<String>,
    /// `None` keeps an existing host's AI access; a new host then asks.
    #[serde(default)]
    pub ai_access: Option<AiAccess>,
}

pub async fn host_list(pool: crate::app::State<'_, DbPool>) -> Result<Vec<SshHostRecord>, String> {
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

pub async fn host_save(
    connection: SaveConnectionInput,
    pool: crate::app::State<'_, DbPool>,
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
        let jump_host_id = trimmed(connection.jump_host_id);
        if let Some(jump_host_id) = &jump_host_id {
            validate_jump_chain(db, connection.id.as_deref(), jump_host_id)?;
        }
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
                    jump_host_id: Some(jump_host_id),
                    ai_access: connection
                        .ai_access
                        .map(|access| access.as_str().to_string()),
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
                    jump_host_id,
                    ai_access: connection
                        .ai_access
                        .unwrap_or(AiAccess::Ask)
                        .as_str()
                        .to_string(),
                })
                .get_result::<SshHost>(db)
                .map_err(internal_error)?,
        };

        let keys = ssh_keys::table.load::<SshKey>(db).map_err(internal_error)?;
        Ok(to_record(&keys, host))
    })
    .await
}

pub async fn host_delete(
    id: String,
    ssh_key_id: Option<String>,
    pool: crate::app::State<'_, DbPool>,
) -> Result<(), String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        diesel::delete(ssh_hosts::table.filter(ssh_hosts::id.eq(id)))
            .execute(connection)
            .map_err(internal_error)?;

        // Keys can be shared between hosts; only drop one no other host still uses,
        // otherwise ON DELETE SET NULL silently strips it from the remaining hosts.
        if let Some(ssh_key_id) = ssh_key_id {
            let still_used = diesel::select(diesel::dsl::exists(
                ssh_hosts::table.filter(ssh_hosts::ssh_key_id.eq(&ssh_key_id)),
            ))
            .get_result::<bool>(connection)
            .map_err(internal_error)?;
            if !still_used {
                diesel::delete(ssh_keys::table.filter(ssh_keys::id.eq(ssh_key_id)))
                    .execute(connection)
                    .map_err(internal_error)?;
            }
        }

        Ok(())
    })
    .await
}

pub async fn host_set_ai_access(
    id: String,
    access: AiAccess,
    pool: crate::app::State<'_, DbPool>,
) -> Result<(), String> {
    set_ai_access(pool.inner().clone(), id, access).await
}

pub(crate) async fn set_ai_access(
    pool: DbPool,
    id: String,
    access: AiAccess,
) -> Result<(), String> {
    run_db(pool, move |connection| {
        let updated = diesel::update(ssh_hosts::table.filter(ssh_hosts::id.eq(id)))
            .set(ssh_hosts::ai_access.eq(access.as_str()))
            .execute(connection)
            .map_err(internal_error)?;
        if updated == 0 {
            return Err("host not found".to_string());
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
        jump_host_id: host.jump_host_id,
        auth,
        ai_access: AiAccess::parse(&host.ai_access),
    }
}

/// Rejects a jump chain that loops back to the saved host or exceeds
/// `MAX_JUMP_HOPS`, so connecting can always resolve a finite chain.
pub(super) fn validate_jump_chain(
    db: &mut SqliteConnection,
    host_id: Option<&str>,
    jump_host_id: &str,
) -> Result<(), String> {
    let mut next = Some(jump_host_id.to_string());
    let mut hops = 0;
    while let Some(current) = next {
        if Some(current.as_str()) == host_id {
            return Err("Jump host chain loops back to this host".to_string());
        }
        hops += 1;
        if hops > MAX_JUMP_HOPS {
            return Err(format!("Jump host chain exceeds {MAX_JUMP_HOPS} hops"));
        }
        next = ssh_hosts::table
            .filter(ssh_hosts::id.eq(&current))
            .select(ssh_hosts::jump_host_id)
            .first::<Option<String>>(db)
            .optional()
            .map_err(internal_error)?
            .ok_or_else(|| "Jump host not found".to_string())?;
    }
    Ok(())
}

fn trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
