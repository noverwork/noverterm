use chrono::Utc;
use diesel::prelude::*;
use orm::models::{NewSetting, SettingRow};
use orm::schema::settings;
use shared::Setting;

use super::{internal_error, run_db, DbPool};

#[tauri::command]
#[specta::specta]
pub async fn get_all_settings(pool: tauri::State<'_, DbPool>) -> Result<Vec<Setting>, String> {
    let pool = pool.inner().clone();
    run_db(pool, |connection| {
        settings::table
            .order(settings::key.asc())
            .load::<SettingRow>(connection)
            .map(|rows| rows.into_iter().map(to_setting).collect())
            .map_err(internal_error)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_setting(
    key: String,
    pool: tauri::State<'_, DbPool>,
) -> Result<Option<Setting>, String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        settings::table
            .filter(settings::key.eq(key))
            .first::<SettingRow>(connection)
            .optional()
            .map(|row| row.map(to_setting))
            .map_err(internal_error)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn set_setting(
    setting: Setting,
    pool: tauri::State<'_, DbPool>,
) -> Result<Setting, String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        let now = Utc::now().naive_utc();
        diesel::insert_into(settings::table)
            .values(&NewSetting {
                key: setting.key,
                value: setting.value,
                created_at: now,
                updated_at: now,
            })
            .on_conflict(settings::key)
            .do_update()
            .set((
                settings::value.eq(diesel::upsert::excluded(settings::value)),
                settings::updated_at.eq(now),
            ))
            .get_result::<SettingRow>(connection)
            .map(to_setting)
            .map_err(internal_error)
    })
    .await
}

fn to_setting(row: SettingRow) -> Setting {
    Setting {
        key: row.key,
        value: row.value,
    }
}
