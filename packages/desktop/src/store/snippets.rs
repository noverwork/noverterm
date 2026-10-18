use chrono::Utc;
use diesel::prelude::*;
use orm::models::{HostSnippet, NewHostSnippet, UpdateHostSnippet};
use orm::schema::{host_snippets, ssh_hosts};
use shared::{SnippetRecord, SnippetWriteRequest};
use uuid::Uuid;

use super::{internal_error, run_db, DbPool};

#[tauri::command]
#[specta::specta]
pub async fn snippet_list(pool: tauri::State<'_, DbPool>) -> Result<Vec<SnippetRecord>, String> {
    let pool = pool.inner().clone();
    run_db(pool, |connection| {
        host_snippets::table
            .left_join(ssh_hosts::table)
            .order((ssh_hosts::name.asc(), host_snippets::title.asc()))
            .select((HostSnippet::as_select(), ssh_hosts::name.nullable()))
            .load::<(HostSnippet, Option<String>)>(connection)
            .map(|rows| rows.into_iter().map(to_record).collect())
            .map_err(internal_error)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn snippet_get(
    id: String,
    pool: tauri::State<'_, DbPool>,
) -> Result<SnippetRecord, String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        host_snippets::table
            .left_join(ssh_hosts::table)
            .filter(host_snippets::id.eq(id))
            .select((HostSnippet::as_select(), ssh_hosts::name.nullable()))
            .first::<(HostSnippet, Option<String>)>(connection)
            .optional()
            .map_err(internal_error)?
            .map(to_record)
            .ok_or_else(|| "snippet not found".to_string())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn snippet_create(
    snippet: SnippetWriteRequest,
    pool: tauri::State<'_, DbPool>,
) -> Result<SnippetRecord, String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        let now = Utc::now().naive_utc();
        let host_id = snippet.host_id.clone();
        let created = diesel::insert_into(host_snippets::table)
            .values(&NewHostSnippet {
                id: Uuid::new_v4().to_string(),
                host_id: snippet.host_id,
                title: snippet.title,
                body: snippet.body,
                created_at: now,
                updated_at: now,
                pin_order: match snippet.pin_icon {
                    Some(_) => Some(next_pin_order(connection)?),
                    None => None,
                },
                pin_icon: snippet.pin_icon,
            })
            .get_result::<HostSnippet>(connection)
            .map_err(internal_error)?;

        Ok(to_record((
            created,
            host_name(connection, host_id.as_deref())?,
        )))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn snippet_update(
    id: String,
    snippet: SnippetWriteRequest,
    pool: tauri::State<'_, DbPool>,
) -> Result<SnippetRecord, String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        let host_id = snippet.host_id.clone();
        let current_order = host_snippets::table
            .filter(host_snippets::id.eq(&id))
            .select(host_snippets::pin_order)
            .first::<Option<i32>>(connection)
            .optional()
            .map_err(internal_error)?
            .ok_or_else(|| "snippet not found".to_string())?;
        // Keep a pinned snippet in place; a newly pinned one goes to the end.
        let pin_order = match (&snippet.pin_icon, current_order) {
            (None, _) => None,
            (Some(_), Some(order)) => Some(order),
            (Some(_), None) => Some(next_pin_order(connection)?),
        };
        let updated = diesel::update(host_snippets::table.filter(host_snippets::id.eq(id)))
            .set(UpdateHostSnippet {
                host_id: snippet.host_id,
                title: snippet.title,
                body: snippet.body,
                updated_at: Utc::now().naive_utc(),
                pin_icon: snippet.pin_icon,
                pin_order,
            })
            .get_result::<HostSnippet>(connection)
            .optional()
            .map_err(internal_error)?
            .ok_or_else(|| "snippet not found".to_string())?;

        Ok(to_record((
            updated,
            host_name(connection, host_id.as_deref())?,
        )))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn snippet_delete(id: String, pool: tauri::State<'_, DbPool>) -> Result<(), String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        diesel::delete(host_snippets::table.filter(host_snippets::id.eq(id)))
            .execute(connection)
            .map(|_| ())
            .map_err(internal_error)
    })
    .await
}

/// Stores the sidebar order of pinned snippets as given, first to last.
#[tauri::command]
#[specta::specta]
pub async fn snippet_reorder_pins(
    ids: Vec<String>,
    pool: tauri::State<'_, DbPool>,
) -> Result<(), String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        connection
            .transaction(|connection| {
                for (index, id) in ids.iter().enumerate() {
                    diesel::update(
                        host_snippets::table
                            .filter(host_snippets::id.eq(id))
                            .filter(host_snippets::pin_icon.is_not_null()),
                    )
                    .set(host_snippets::pin_order.eq(index as i32))
                    .execute(connection)?;
                }
                Ok::<_, diesel::result::Error>(())
            })
            .map_err(internal_error)
    })
    .await
}

fn next_pin_order(connection: &mut diesel::sqlite::SqliteConnection) -> Result<i32, String> {
    host_snippets::table
        .select(diesel::dsl::max(host_snippets::pin_order))
        .first::<Option<i32>>(connection)
        .map(|max| max.map_or(0, |max| max + 1))
        .map_err(internal_error)
}

const LOCAL_TERMINAL_NAME: &str = "Local Terminal";

fn host_name(
    connection: &mut diesel::sqlite::SqliteConnection,
    host_id: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(host_id) = host_id else {
        return Ok(None);
    };
    ssh_hosts::table
        .filter(ssh_hosts::id.eq(host_id))
        .select(ssh_hosts::name)
        .first::<String>(connection)
        .optional()
        .map_err(internal_error)
}

fn to_record((snippet, host_name): (HostSnippet, Option<String>)) -> SnippetRecord {
    SnippetRecord {
        host_name: match &snippet.host_id {
            Some(_) => host_name.unwrap_or_else(|| "Unknown".to_string()),
            None => LOCAL_TERMINAL_NAME.to_string(),
        },
        id: snippet.id,
        host_id: snippet.host_id,
        title: snippet.title,
        body: snippet.body,
        pin_icon: snippet.pin_icon,
        pin_order: snippet.pin_order,
    }
}
