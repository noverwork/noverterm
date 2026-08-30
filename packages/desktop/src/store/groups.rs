use chrono::Utc;
use diesel::prelude::*;
use orm::models::{HostGroup, NewHostGroup};
use orm::schema::host_groups;
use shared::HostGroupRecord;
use uuid::Uuid;

use super::{internal_error, run_db, DbPool};

#[tauri::command]
#[specta::specta]
pub async fn host_group_list(
    pool: tauri::State<'_, DbPool>,
) -> Result<Vec<HostGroupRecord>, String> {
    let pool = pool.inner().clone();
    run_db(pool, |connection| {
        host_groups::table
            .order(host_groups::name.asc())
            .load::<HostGroup>(connection)
            .map(|groups| groups.into_iter().map(to_record).collect())
            .map_err(internal_error)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn host_group_create(
    name: String,
    pool: tauri::State<'_, DbPool>,
) -> Result<HostGroupRecord, String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("host group name is required".to_string());
        }

        let now = Utc::now().naive_utc();
        diesel::insert_into(host_groups::table)
            .values(&NewHostGroup {
                id: Uuid::new_v4().to_string(),
                name,
                created_at: now,
                updated_at: now,
            })
            .get_result::<HostGroup>(connection)
            .map(to_record)
            .map_err(|error| match error {
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                ) => "host group already exists".to_string(),
                other => internal_error(other),
            })
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn host_group_delete(id: String, pool: tauri::State<'_, DbPool>) -> Result<(), String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        diesel::delete(host_groups::table.filter(host_groups::id.eq(id)))
            .execute(connection)
            .map(|_| ())
            .map_err(internal_error)
    })
    .await
}

fn to_record(group: HostGroup) -> HostGroupRecord {
    HostGroupRecord {
        id: group.id,
        name: group.name,
    }
}
