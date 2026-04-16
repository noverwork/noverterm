use diesel::pg::PgConnection;
use diesel::Connection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../orm/migrations");

use std::env;
use std::fs;
use std::path::PathBuf;

fn default_db_url() -> String {
    env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/app".to_string())
}

fn run_migrations(db_url: &str) {
    let mut conn = PgConnection::establish(db_url)
        .unwrap_or_else(|e| panic!("failed to connect to database: {e}"));

    let applied = conn
        .run_pending_migrations(MIGRATIONS)
        .expect("failed to run migrations");

    if applied.is_empty() {
        println!("No pending migrations.");
    } else {
        println!("Applied {} migration(s):", applied.len());
        for m in &applied {
            println!("  - {m}");
        }
    }
}

fn revert_migration(db_url: &str) {
    let mut conn = PgConnection::establish(db_url)
        .unwrap_or_else(|e| panic!("failed to connect to database: {e}"));

    let result = conn.revert_last_migration(MIGRATIONS);
    match result {
        Ok(reverted) => println!("Reverted: {reverted}"),
        Err(_) => println!("No migrations to revert."),
    }
}

fn create_migration(name: &str) {
    let migrations_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../orm/migrations");

    fs::create_dir_all(&migrations_dir).expect("failed to create migrations directory");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs();

    let migration_name = format!("{:014}_{}", timestamp, name.replace(' ', "_"));
    let migration_dir = migrations_dir.join(&migration_name);

    fs::create_dir_all(&migration_dir)
        .unwrap_or_else(|e| panic!("failed to create migration directory: {e}"));

    fs::write(migration_dir.join("up.sql"), "-- Add migration SQL here\n")
        .expect("failed to create up.sql");
    fs::write(migration_dir.join("down.sql"), "-- Add revert SQL here\n")
        .expect("failed to create down.sql");

    println!("Created migration: {migration_name}");
    println!("  -> {}/up.sql", migration_dir.display());
    println!("  -> {}/down.sql", migration_dir.display());
}

fn print_usage() {
    println!("Usage: migrator <command> [args]");
    println!();
    println!("Commands:");
    println!("  up               Run pending migrations");
    println!("  down             Revert last migration");
    println!("  create <name>    Create a new migration");
    println!();
    println!("Options:");
    println!("  --db <url>       DATABASE_URL (default: from env or postgres://localhost)");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut db_url = default_db_url();
    let mut command: Option<&str> = None;
    let mut command_args: Vec<&str> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--db" => {
                i += 1;
                if i < args.len() {
                    db_url = args[i].clone();
                } else {
                    eprintln!("Error: --db requires a URL argument");
                    std::process::exit(1);
                }
            }
            "up" | "down" | "create" => {
                command = Some(args[i].as_str());
                i += 1;
                while i < args.len() {
                    command_args.push(&args[i]);
                    i += 1;
                }
            }
            "--help" | "-h" => {
                print_usage();
                return;
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_usage();
                std::process::exit(1);
            }
        }
    }

    let Some(cmd) = command else {
        print_usage();
        std::process::exit(1);
    };

    match cmd {
        "up" => run_migrations(&db_url),
        "down" => revert_migration(&db_url),
        "create" => {
            let name = command_args
                .first()
                .expect("create requires a migration name");
            create_migration(name);
        }
        _ => unreachable!(),
    }
}
