use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::links;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = links)]
pub struct Link {
    pub id: String,
    pub subdomain: String,
    pub target_host: String,
    pub target_port: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = links)]
pub struct NewLink {
    pub id: String,
    pub subdomain: String,
    pub target_host: String,
    pub target_port: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = links)]
pub struct UpdateLink {
    pub subdomain: String,
    pub target_host: String,
    pub target_port: i32,
    pub updated_at: NaiveDateTime,
}
