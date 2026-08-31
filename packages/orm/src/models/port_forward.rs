use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::{port_forward_mappings, port_forwards};

#[derive(
    Debug, Clone, Queryable, Selectable, Associations, AsChangeset, Serialize, Deserialize,
)]
#[diesel(table_name = port_forwards)]
#[diesel(belongs_to(super::SshHost, foreign_key = host_id))]
pub struct PortForward {
    pub id: String,
    pub name: String,
    pub host_id: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = port_forwards)]
pub struct NewPortForward {
    pub id: String,
    pub name: String,
    pub host_id: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = port_forwards)]
pub struct UpdatePortForward {
    pub name: String,
    pub host_id: String,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Queryable, Selectable, Associations, Serialize, Deserialize)]
#[diesel(table_name = port_forward_mappings)]
#[diesel(belongs_to(PortForward, foreign_key = forward_id))]
pub struct PortForwardMapping {
    pub id: String,
    pub forward_id: String,
    pub position: i32,
    pub bind_host: String,
    pub bind_port: i32,
    pub target_host: String,
    pub target_port: i32,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = port_forward_mappings)]
pub struct NewPortForwardMapping {
    pub id: String,
    pub forward_id: String,
    pub position: i32,
    pub bind_host: String,
    pub bind_port: i32,
    pub target_host: String,
    pub target_port: i32,
}
