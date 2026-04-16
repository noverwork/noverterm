use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::settings;

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = settings)]
pub struct Setting {
    pub id: i32,
    pub key: String,
    pub value: String,
}
