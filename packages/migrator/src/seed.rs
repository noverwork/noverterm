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

fn main() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut conn = PgConnection::establish(&database_url).expect("Failed to connect to DB");

    let now = Utc::now().naive_utc();

    // ── Default User ─────────────────────────────────────────────────────────
    diesel::insert_into(users::table)
        .values(NewUser {
            id: "default-user-0000-0000-0000-000000000001".to_string(),
            email: "admin@nover.local".to_string(),
            password_hash: hash_password("admin123"),
            created_at: now,
            updated_at: now,
        })
        .execute(&mut conn)
        .expect("Failed to seed default user");
    println!("✓ Seeded default user");

    println!("\n✅ Base seed data inserted successfully");
    println!("   Admin email: admin@nover.local");
    println!("   Admin password: admin123");
}
