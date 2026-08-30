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
    pub updated_at: NaiveDateTime,
}
