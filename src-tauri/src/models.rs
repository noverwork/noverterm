use crate::schema::users;
use chrono::NaiveDateTime;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Queryable, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Insertable, Serialize, Deserialize)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub name: String,
    pub email: String,
}
