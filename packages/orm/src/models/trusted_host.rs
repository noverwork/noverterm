use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::trusted_hosts;

#[derive(Debug, Clone, Queryable, Selectable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = trusted_hosts)]
pub struct TrustedHost {
    pub host: String,
    pub port: i32,
    pub algorithm: String,
    pub fingerprint: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = trusted_hosts)]
pub struct NewTrustedHost {
    pub host: String,
    pub port: i32,
    pub algorithm: String,
    pub fingerprint: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
