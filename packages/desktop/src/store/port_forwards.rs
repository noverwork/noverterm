use std::collections::{HashMap, HashSet};

use chrono::Utc;
use diesel::prelude::*;
use orm::models::{
    NewPortForward, NewPortForwardMapping, PortForward, PortForwardMapping, UpdatePortForward,
};
use orm::schema::{port_forward_mappings, port_forwards, ssh_hosts};
use shared::{PortForwardMappingInput, PortForwardRecord, PortForwardWriteRequest};
use uuid::Uuid;

use super::{internal_error, run_db, DbPool};

#[tauri::command]
#[specta::specta]
pub async fn port_forward_preset_list(
    pool: tauri::State<'_, DbPool>,
) -> Result<Vec<PortForwardRecord>, String> {
    let pool = pool.inner().clone();
    run_db(pool, |connection| {
        let forwards = port_forwards::table
            .inner_join(ssh_hosts::table)
            .order((ssh_hosts::name.asc(), port_forwards::name.asc()))
            .select((PortForward::as_select(), ssh_hosts::name))
            .load::<(PortForward, String)>(connection)
            .map_err(internal_error)?;

        // One extra query for every mapping, grouped in memory: a preset list is
        // small and this keeps it off the N+1 path.
        let mut mappings: HashMap<String, Vec<PortForwardMappingInput>> = HashMap::new();
        for mapping in port_forward_mappings::table
            .order(port_forward_mappings::position.asc())
            .select(PortForwardMapping::as_select())
            .load::<PortForwardMapping>(connection)
            .map_err(internal_error)?
        {
            mappings
                .entry(mapping.forward_id)
                .or_default()
                .push(PortForwardMappingInput {
                    bind_host: mapping.bind_host,
                    bind_port: mapping.bind_port,
                    target_host: mapping.target_host,
                    target_port: mapping.target_port,
                });
        }

        Ok(forwards
            .into_iter()
            .map(|(forward, host_name)| PortForwardRecord {
                mappings: mappings.remove(&forward.id).unwrap_or_default(),
                id: forward.id,
                name: forward.name,
                host_id: forward.host_id,
                host_name,
            })
            .collect())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn port_forward_preset_create(
    forward: PortForwardWriteRequest,
    pool: tauri::State<'_, DbPool>,
) -> Result<PortForwardRecord, String> {
    let forward = validate(forward)?;
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        let id = Uuid::new_v4().to_string();
        connection
            .transaction(|connection| {
                let now = Utc::now().naive_utc();
                diesel::insert_into(port_forwards::table)
                    .values(&NewPortForward {
                        id: id.clone(),
                        name: forward.name.clone(),
                        host_id: forward.host_id.clone(),
                        created_at: now,
                        updated_at: now,
                    })
                    .execute(connection)?;
                insert_mappings(connection, &id, &forward.mappings)
            })
            .map_err(internal_error)?;

        read_record(connection, &id)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn port_forward_preset_update(
    id: String,
    forward: PortForwardWriteRequest,
    pool: tauri::State<'_, DbPool>,
) -> Result<PortForwardRecord, String> {
    let forward = validate(forward)?;
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        let updated = connection
            .transaction(|connection| {
                let updated = diesel::update(port_forwards::table.find(&id))
                    .set(UpdatePortForward {
                        name: forward.name.clone(),
                        host_id: forward.host_id.clone(),
                        updated_at: Utc::now().naive_utc(),
                    })
                    .execute(connection)?;
                if updated == 0 {
                    return Ok(false);
                }

                // Mappings carry no state worth preserving, so a save replaces the
                // whole set rather than diffing it.
                diesel::delete(
                    port_forward_mappings::table.filter(port_forward_mappings::forward_id.eq(&id)),
                )
                .execute(connection)?;
                insert_mappings(connection, &id, &forward.mappings)?;
                Ok(true)
            })
            .map_err(internal_error)?;

        if !updated {
            return Err("port forward not found".to_string());
        }

        read_record(connection, &id)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn port_forward_preset_delete(
    id: String,
    pool: tauri::State<'_, DbPool>,
) -> Result<(), String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        diesel::delete(port_forwards::table.find(id))
            .execute(connection)
            .map(|_| ())
            .map_err(internal_error)
    })
    .await
}

/// Rejects presets the runtime could never start, and duplicate bind addresses
/// inside one group, which would always collide with each other.
fn validate(forward: PortForwardWriteRequest) -> Result<PortForwardWriteRequest, String> {
    if forward.name.trim().is_empty() {
        return Err("name is required".to_string());
    }
    if forward.mappings.is_empty() {
        return Err("at least one port mapping is required".to_string());
    }

    let mut seen = HashSet::new();
    for mapping in &forward.mappings {
        if mapping.bind_host.trim().is_empty() || mapping.target_host.trim().is_empty() {
            return Err("bind host and target host are required".to_string());
        }
        for port in [mapping.bind_port, mapping.target_port] {
            if !(1..=65535).contains(&port) {
                return Err(format!("port {port} must be between 1 and 65535"));
            }
        }
        if !seen.insert((mapping.bind_host.trim(), mapping.bind_port)) {
            return Err(format!(
                "duplicate bind address {}:{}",
                mapping.bind_host.trim(),
                mapping.bind_port
            ));
        }
    }

    Ok(forward)
}

fn insert_mappings(
    connection: &mut SqliteConnection,
    forward_id: &str,
    mappings: &[PortForwardMappingInput],
) -> Result<(), diesel::result::Error> {
    let rows: Vec<NewPortForwardMapping> = mappings
        .iter()
        .enumerate()
        .map(|(position, mapping)| NewPortForwardMapping {
            id: Uuid::new_v4().to_string(),
            forward_id: forward_id.to_string(),
            position: position as i32,
            bind_host: mapping.bind_host.trim().to_string(),
            bind_port: mapping.bind_port,
            target_host: mapping.target_host.trim().to_string(),
            target_port: mapping.target_port,
        })
        .collect();

    diesel::insert_into(port_forward_mappings::table)
        .values(&rows)
        .execute(connection)
        .map(|_| ())
}

fn read_record(connection: &mut SqliteConnection, id: &str) -> Result<PortForwardRecord, String> {
    let (forward, host_name) = port_forwards::table
        .inner_join(ssh_hosts::table)
        .filter(port_forwards::id.eq(id))
        .select((PortForward::as_select(), ssh_hosts::name))
        .first::<(PortForward, String)>(connection)
        .optional()
        .map_err(internal_error)?
        .ok_or_else(|| "port forward not found".to_string())?;

    let mappings = port_forward_mappings::table
        .filter(port_forward_mappings::forward_id.eq(id))
        .order(port_forward_mappings::position.asc())
        .select(PortForwardMapping::as_select())
        .load::<PortForwardMapping>(connection)
        .map_err(internal_error)?
        .into_iter()
        .map(|mapping| PortForwardMappingInput {
            bind_host: mapping.bind_host,
            bind_port: mapping.bind_port,
            target_host: mapping.target_host,
            target_port: mapping.target_port,
        })
        .collect();

    Ok(PortForwardRecord {
        id: forward.id,
        name: forward.name,
        host_id: forward.host_id,
        host_name,
        mappings,
    })
}
