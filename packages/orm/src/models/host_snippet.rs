use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::host_snippets;

#[derive(
    Debug, Clone, Queryable, Selectable, Associations, AsChangeset, Serialize, Deserialize,
)]
#[diesel(table_name = host_snippets)]
#[diesel(belongs_to(super::SshHost, foreign_key = host_id))]
pub struct HostSnippet {
    pub id: String,
    pub host_id: String,
    pub owner_id: String,
    pub title: String,
    pub body: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = host_snippets)]
pub struct NewHostSnippet {
    pub id: String,
    pub host_id: String,
    pub owner_id: String,
    pub title: String,
    pub body: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = host_snippets)]
pub struct UpdateHostSnippet {
    pub title: String,
    pub body: String,
    pub updated_at: NaiveDateTime,
}
