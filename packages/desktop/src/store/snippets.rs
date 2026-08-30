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
            .inner_join(ssh_hosts::table)
            .order((ssh_hosts::name.asc(), host_snippets::title.asc()))
            .select((HostSnippet::as_select(), ssh_hosts::name))
            .load::<(HostSnippet, String)>(connection)
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
            .inner_join(ssh_hosts::table)
            .filter(host_snippets::id.eq(id))
            .select((HostSnippet::as_select(), ssh_hosts::name))
            .first::<(HostSnippet, String)>(connection)
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
            })
            .get_result::<HostSnippet>(connection)
            .map_err(internal_error)?;

        Ok(to_record((created, host_name(connection, &host_id)?)))
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
        let updated = diesel::update(host_snippets::table.filter(host_snippets::id.eq(id)))
            .set(UpdateHostSnippet {
                host_id: snippet.host_id,
                title: snippet.title,
                body: snippet.body,
                updated_at: Utc::now().naive_utc(),
            })
            .get_result::<HostSnippet>(connection)
            .optional()
            .map_err(internal_error)?
            .ok_or_else(|| "snippet not found".to_string())?;

        Ok(to_record((updated, host_name(connection, &host_id)?)))
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

fn host_name(
    connection: &mut diesel::sqlite::SqliteConnection,
    host_id: &str,
) -> Result<String, String> {
    ssh_hosts::table
        .filter(ssh_hosts::id.eq(host_id))
        .select(ssh_hosts::name)
        .first::<String>(connection)
        .optional()
        .map_err(internal_error)
        .map(|name| name.unwrap_or_else(|| "Unknown".to_string()))
}

fn to_record((snippet, host_name): (HostSnippet, String)) -> SnippetRecord {
    SnippetRecord {
        id: snippet.id,
        host_id: snippet.host_id,
        host_name,
        title: snippet.title,
        body: snippet.body,
    }
}
