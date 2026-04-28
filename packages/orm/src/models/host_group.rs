use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::host_groups;

#[derive(
    Debug, Clone, Queryable, Selectable, Associations, AsChangeset, Serialize, Deserialize,
)]
#[diesel(table_name = host_groups)]
#[diesel(belongs_to(super::User, foreign_key = owner_id))]
pub struct HostGroup {
    pub id: String,
    pub owner_id: String,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = host_groups)]
pub struct NewHostGroup {
    pub id: String,
    pub owner_id: String,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = host_groups)]
pub struct UpdateHostGroup {
    pub name: String,
    pub updated_at: NaiveDateTime,
}
