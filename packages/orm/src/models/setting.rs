use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::settings;

#[derive(Debug, Clone, Queryable, Selectable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = settings)]
pub struct SettingRow {
    pub key: String,
    pub value: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = settings)]
pub struct NewSetting {
    pub key: String,
    pub value: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
