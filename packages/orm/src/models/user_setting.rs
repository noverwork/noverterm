use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::user_settings;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = user_settings)]
pub struct UserSetting {
    pub owner_id: String,
    pub key: String,
    pub value: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = user_settings)]
pub struct NewUserSetting {
    pub owner_id: String,
    pub key: String,
    pub value: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = user_settings)]
pub struct UpdateUserSetting {
    pub value: String,
    pub updated_at: NaiveDateTime,
}
