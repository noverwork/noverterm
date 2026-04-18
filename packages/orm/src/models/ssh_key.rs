use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::ssh_keys;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ssh_keys)]
pub struct SshKey {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub fingerprint: Option<String>,
    pub encrypted_private_key: String,
    pub encrypted_passphrase: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub owner_id: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = ssh_keys)]
pub struct NewSshKey {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub fingerprint: Option<String>,
    pub encrypted_private_key: String,
    pub encrypted_passphrase: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub owner_id: String,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = ssh_keys)]
pub struct UpdateSshKey {
    pub name: String,
    pub kind: String,
    pub fingerprint: Option<String>,
    pub encrypted_private_key: String,
    pub encrypted_passphrase: Option<String>,
    pub updated_at: NaiveDateTime,
}
