use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::ssh_hosts;

#[derive(Debug, Clone, Queryable, Selectable, Associations, Serialize, Deserialize)]
#[diesel(table_name = ssh_hosts)]
#[diesel(belongs_to(super::SshKey, foreign_key = ssh_key_id))]
pub struct SshHost {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub auth_mode: String,
    pub ssh_key_id: Option<String>,
    pub encrypted_password: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_connected_at: Option<NaiveDateTime>,
    pub owner_id: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = ssh_hosts)]
pub struct NewSshHost {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub auth_mode: String,
    pub ssh_key_id: Option<String>,
    pub encrypted_password: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_connected_at: Option<NaiveDateTime>,
    pub owner_id: String,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = ssh_hosts)]
pub struct UpdateSshHost {
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub auth_mode: String,
    pub ssh_key_id: Option<String>,
    pub encrypted_password: Option<String>,
    pub updated_at: NaiveDateTime,
    pub last_connected_at: Option<NaiveDateTime>,
}
