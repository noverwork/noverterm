use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use chrono::Utc;
use diesel::prelude::*;
use orm::models::NewUser;
use orm::schema::users;
use std::env;

fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("argon2 hashing should not fail")
        .to_string()
}

const DEV_USER_ID: &str = "dev-user-00000000-0000-0000-0000-000000000001";

fn main() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut conn = PgConnection::establish(&database_url).expect("Failed to connect to DB");

    let now = Utc::now().naive_utc();

    diesel::insert_into(users::table)
        .values(NewUser {
            id: DEV_USER_ID.to_string(),
            email: "dev@nover.local".to_string(),
            password_hash: hash_password("dev123"),
            created_at: now,
            updated_at: now,
        })
        .execute(&mut conn)
        .expect("Failed to seed dev user");
    println!("✓ Seeded dev user");

    println!("\n✅ Dev account inserted successfully");
    println!("   DEV_USER_ID  = {}", DEV_USER_ID);
    println!("\n   Dev email: dev@nover.local");
    println!("   Dev password: dev123");
}
