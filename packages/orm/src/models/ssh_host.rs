use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::ssh_hosts;

#[derive(
    Debug, Clone, Queryable, Selectable, Associations, AsChangeset, Serialize, Deserialize,
)]
#[diesel(table_name = ssh_hosts)]
#[diesel(belongs_to(super::SshKey, foreign_key = ssh_key_id))]
pub struct SshHost {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub ssh_key_id: Option<String>,
    pub password: Option<String>,
    pub group_id: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub jump_host_id: Option<String>,
    /// `ask`, `allow`, or `deny`: what MCP clients may do here without asking.
    pub ai_access: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = ssh_hosts)]
pub struct NewSshHost {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub ssh_key_id: Option<String>,
    pub password: Option<String>,
    pub group_id: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub jump_host_id: Option<String>,
    pub ai_access: String,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = ssh_hosts)]
pub struct UpdateSshHost {
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub ssh_key_id: Option<Option<String>>,
    pub password: Option<Option<String>>,
    pub group_id: Option<Option<String>>,
    pub jump_host_id: Option<Option<String>>,
    /// `None` keeps the host's current AI access.
    pub ai_access: Option<String>,
    pub updated_at: NaiveDateTime,
}
