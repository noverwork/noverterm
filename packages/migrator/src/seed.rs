use chrono::Utc;
use diesel::prelude::*;
use orm::models::{NewUser, NewUserSetting};
use orm::schema::{user_settings, users};
use std::env;

fn main() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut conn = PgConnection::establish(&database_url).expect("Failed to connect to DB");

    let now = Utc::now().naive_utc();

    // ── Default User ─────────────────────────────────────────────────────────
    diesel::insert_into(users::table)
        .values(NewUser {
            id: "default-user-0000-0000-0000-000000000001".to_string(),
            email: "admin@nover.local".to_string(),
            password_hash: "240be518fabd2724ddb6f04eeb1da5967448d7e831c08c8fa822809f74c720a9".to_string(),
            created_at: now,
            updated_at: now,
        })
        .execute(&mut conn)
        .expect("Failed to seed default user");
    println!("✓ Seeded default user");

    // ── Default Settings ─────────────────────────────────────────────────────
    diesel::insert_into(user_settings::table)
        .values(&vec![
            NewUserSetting {
                owner_id: "default-user-0000-0000-0000-000000000001".to_string(),
                key: "theme".to_string(),
                value: "dark".to_string(),
                created_at: now,
                updated_at: now,
            },
            NewUserSetting {
                owner_id: "default-user-0000-0000-0000-000000000001".to_string(),
                key: "language".to_string(),
                value: "en".to_string(),
                created_at: now,
                updated_at: now,
            },
        ])
        .execute(&mut conn)
        .expect("Failed to seed default settings");
    println!("✓ Seeded default settings");

    println!("\n✅ Base seed data inserted successfully");
    println!("   Admin email: admin@nover.local");
    println!("   Admin password: admin123");
}
