use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use chrono::Utc;
use diesel::prelude::*;
use orm::models::{NewSshHost, NewSshKey, NewUser};
use orm::schema::{ssh_hosts, ssh_keys, users};
use std::env;

fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("argon2 hashing should not fail")
        .to_string()
}

const DEV_USER_ID: &str = "dev-user-00000000-0000-0000-0000-000000000001";
const DEV_KEY_ID: &str = "dev-key-00000000-0000-0000-0000-000000000001";
const DEV_HOST_ID: &str = "dev-host-00000000-0000-0000-0000-000000000001";

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

    let dummy_key = concat!(
        "-----BEGIN OPENSSH PRIVATE KEY-----\n",
        "b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAAAAAAAAAAAAAAAABAAAA\n",
        "AAAADGV4dGVyc2VjcmV0AAAAAAAAAAAAAAADAAAAAAAAAAGpAAAAAAAL\n",
        "AAAAFgAAAAAAAAdkeGJyYzEyMzQ1Njc4OTABEGEAAABRw7yuKEhAQEAA\n",
        "AAAAAIQAAAAAAAAdkeGJyYzEyMzQ1Njc4OTABEGEAAAAAAAAAAAAAAAE\n",
        "AAABRw7yuKEhAQEAAABAAAAAAEAAAAPAAAAAQIDBAUGBwgJEhcYGBoaG\n",
        "AAAAAQAAAAABAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEAAAAPAAAAAQI\n",
        "DAQYHBwgJCAkKCwwNDg8QERITFBUWFxgZGhscHR4fAAAAAAECAwQFBgcI\n",
        "CQonLC0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fn8GC\n",
        "AAAAAAAAAAAAAAA=\n",
        "-----END OPENSSH PRIVATE KEY-----\n"
    );

    diesel::insert_into(ssh_keys::table)
        .values(NewSshKey {
            id: DEV_KEY_ID.to_string(),
            name: "Dev Local SSH Key".to_string(),
            kind: "ed25519".to_string(),
            fingerprint: Some("SHA256:devonlynotreal00000000000000000000".to_string()),
            encrypted_private_key: dummy_key.to_string(),
            encrypted_passphrase: None,
            created_at: now,
            updated_at: now,
            owner_id: DEV_USER_ID.to_string(),
        })
        .execute(&mut conn)
        .expect("Failed to seed dev SSH key");
    println!("✓ Seeded dev SSH key");

    diesel::insert_into(ssh_hosts::table)
        .values(NewSshHost {
            id: DEV_HOST_ID.to_string(),
            name: "Dev Local Server".to_string(),
            host: "127.0.0.1".to_string(),
            port: 22,
            username: "ubuntu".to_string(),
            ssh_key_id: Some(DEV_KEY_ID.to_string()),
            encrypted_password: None,
            created_at: now,
            updated_at: now,
            owner_id: DEV_USER_ID.to_string(),
        })
        .execute(&mut conn)
        .expect("Failed to seed dev SSH host");
    println!("✓ Seeded dev SSH host");

    println!("\n✅ All dev seed data inserted successfully");
    println!("   DEV_USER_ID  = {}", DEV_USER_ID);
    println!("   DEV_KEY_ID   = {}", DEV_KEY_ID);
    println!("   DEV_HOST_ID  = {}", DEV_HOST_ID);
    println!("\n   Dev email: dev@nover.local");
    println!("   Dev password: dev123");
}
