use chrono::Utc;
use diesel::prelude::*;
use orm::models::{NewTrustedHost, TrustedHost};
use orm::schema::trusted_hosts;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::store::{internal_error, run_db, DbPool};

/// Host keys the user has accepted, backed by the same database as everything
/// else so a backup covers them too.
#[derive(Debug, Clone)]
pub struct SshTrustStore {
    pool: DbPool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
pub struct HostTrustPrompt {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
pub struct HostTrustMismatch {
    pub host: String,
    pub port: u16,
    pub expected_algorithm: String,
    pub expected_fingerprint: String,
    pub presented_algorithm: String,
    pub presented_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
pub struct HostTrustConfirmation {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrustCheck {
    Trusted,
    TrustRequired(HostTrustPrompt),
    TrustMismatch(HostTrustMismatch),
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
pub struct TrustedSshHost {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
pub struct KnownHostsResponse {
    pub hosts: Vec<TrustedSshHost>,
}

impl SshTrustStore {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn confirm(&self, confirmation: HostTrustConfirmation) -> Result<(), String> {
        let pool = self.pool.clone();
        run_db(pool, move |connection| {
            let now = Utc::now().naive_utc();
            diesel::insert_into(trusted_hosts::table)
                .values(&NewTrustedHost {
                    host: confirmation.host,
                    port: i32::from(confirmation.port),
                    algorithm: confirmation.algorithm,
                    fingerprint: confirmation.fingerprint,
                    created_at: now,
                    updated_at: now,
                })
                .on_conflict((trusted_hosts::host, trusted_hosts::port))
                .do_update()
                .set((
                    trusted_hosts::algorithm.eq(diesel::upsert::excluded(trusted_hosts::algorithm)),
                    trusted_hosts::fingerprint
                        .eq(diesel::upsert::excluded(trusted_hosts::fingerprint)),
                    trusted_hosts::updated_at.eq(now),
                ))
                .execute(connection)
                .map(|_| ())
                .map_err(internal_error)
        })
        .await
    }

    pub(crate) async fn evaluate(
        &self,
        host: &str,
        port: u16,
        algorithm: &str,
        fingerprint: &str,
    ) -> TrustCheck {
        let pool = self.pool.clone();
        let lookup_host = host.to_string();
        let record = run_db(pool, move |connection| {
            trusted_hosts::table
                .filter(trusted_hosts::host.eq(lookup_host))
                .filter(trusted_hosts::port.eq(i32::from(port)))
                .first::<TrustedHost>(connection)
                .optional()
                .map_err(internal_error)
        })
        .await;

        // A database error must not silently downgrade to "trusted"; treating it
        // like an unknown host re-prompts instead.
        let Ok(Some(record)) = record else {
            return TrustCheck::TrustRequired(HostTrustPrompt {
                host: host.to_string(),
                port,
                algorithm: algorithm.to_string(),
                fingerprint: fingerprint.to_string(),
            });
        };

        if record.algorithm == algorithm && record.fingerprint == fingerprint {
            TrustCheck::Trusted
        } else {
            TrustCheck::TrustMismatch(HostTrustMismatch {
                host: host.to_string(),
                port,
                expected_algorithm: record.algorithm,
                expected_fingerprint: record.fingerprint,
                presented_algorithm: algorithm.to_string(),
                presented_fingerprint: fingerprint.to_string(),
            })
        }
    }

    pub async fn list(&self) -> Result<Vec<TrustedSshHost>, String> {
        let pool = self.pool.clone();
        run_db(pool, |connection| {
            trusted_hosts::table
                .order((trusted_hosts::host.asc(), trusted_hosts::port.asc()))
                .load::<TrustedHost>(connection)
                .map(|records| records.into_iter().map(to_record).collect())
                .map_err(internal_error)
        })
        .await
    }

    pub async fn remove(&self, host: &str, port: u16) -> Result<(), String> {
        let pool = self.pool.clone();
        let host = host.to_string();
        run_db(pool, move |connection| {
            let removed = diesel::delete(
                trusted_hosts::table
                    .filter(trusted_hosts::host.eq(&host))
                    .filter(trusted_hosts::port.eq(i32::from(port))),
            )
            .execute(connection)
            .map_err(internal_error)?;

            if removed == 0 {
                return Err(format!("No trusted host found for {host}:{port}"));
            }

            Ok(())
        })
        .await
    }
}

#[tauri::command]
#[specta::specta]
pub async fn known_hosts_get(
    trust_store: State<'_, SshTrustStore>,
) -> Result<KnownHostsResponse, String> {
    Ok(KnownHostsResponse {
        hosts: trust_store.list().await?,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn known_hosts_remove(
    host: String,
    port: u16,
    trust_store: State<'_, SshTrustStore>,
) -> Result<KnownHostsResponse, String> {
    trust_store.remove(&host, port).await?;
    Ok(KnownHostsResponse {
        hosts: trust_store.list().await?,
    })
}

fn to_record(record: TrustedHost) -> TrustedSshHost {
    TrustedSshHost {
        host: record.host,
        port: record.port as u16,
        algorithm: record.algorithm,
        fingerprint: record.fingerprint,
    }
}

#[cfg(test)]
mod tests;
