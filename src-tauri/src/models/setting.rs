use crate::schema::settings;
use diesel::sqlite::Sqlite;
use diesel::{Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = settings)]
#[diesel(check_for_backend(Sqlite))]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Insertable, Serialize, Deserialize)]
#[diesel(table_name = settings)]
pub struct NewSetting {
    pub key: String,
    pub value: String,
}
