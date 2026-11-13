use chrono::Utc;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use orm::models::{Link, NewLink, UpdateLink};
use orm::schema::links;
use shared::{LinkRecord, LinkWriteRequest};
use uuid::Uuid;

use super::{internal_error, run_db, DbPool};

pub async fn link_list(pool: crate::app::State<'_, DbPool>) -> Result<Vec<LinkRecord>, String> {
    let pool = pool.inner().clone();
    run_db(pool, |connection| {
        links::table
            .order(links::subdomain.asc())
            .select(Link::as_select())
            .load::<Link>(connection)
            .map(|rows| rows.into_iter().map(to_record).collect())
            .map_err(internal_error)
    })
    .await
}

pub async fn link_create(
    link: LinkWriteRequest,
    pool: crate::app::State<'_, DbPool>,
) -> Result<LinkRecord, String> {
    let link = validate(link)?;
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        let now = Utc::now().naive_utc();
        diesel::insert_into(links::table)
            .values(&NewLink {
                id: Uuid::new_v4().to_string(),
                subdomain: link.subdomain,
                target_host: link.target_host,
                target_port: link.target_port,
                created_at: now,
                updated_at: now,
            })
            .returning(Link::as_returning())
            .get_result(connection)
            .map(to_record)
            .map_err(write_error)
    })
    .await
}

pub async fn link_update(
    id: String,
    link: LinkWriteRequest,
    pool: crate::app::State<'_, DbPool>,
) -> Result<LinkRecord, String> {
    let link = validate(link)?;
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        diesel::update(links::table.find(&id))
            .set(UpdateLink {
                subdomain: link.subdomain,
                target_host: link.target_host,
                target_port: link.target_port,
                updated_at: Utc::now().naive_utc(),
            })
            .returning(Link::as_returning())
            .get_result(connection)
            .optional()
            .map_err(write_error)?
            .map(to_record)
            .ok_or_else(|| "link not found".to_string())
    })
    .await
}

pub async fn link_delete(id: String, pool: crate::app::State<'_, DbPool>) -> Result<(), String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        diesel::delete(links::table.find(id))
            .execute(connection)
            .map(|_| ())
            .map_err(internal_error)
    })
    .await
}

/// Rejects names the relay would refuse and targets that can't be dialed.
fn validate(link: LinkWriteRequest) -> Result<LinkWriteRequest, String> {
    let subdomain = shared::link_subdomain(&link.subdomain)
        .ok_or("name may only use a-z, 0-9 and -, up to 63 characters")?;
    let target_host = link.target_host.trim().to_string();
    if target_host.is_empty() {
        return Err("target host is required".to_string());
    }
    if !(1..=65535).contains(&link.target_port) {
        return Err(format!(
            "port {} must be between 1 and 65535",
            link.target_port
        ));
    }
    Ok(LinkWriteRequest {
        subdomain,
        target_host,
        target_port: link.target_port,
    })
}

fn write_error(error: DieselError) -> String {
    match error {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
            "a link with this name already exists".to_string()
        }
        other => internal_error(other),
    }
}

fn to_record(link: Link) -> LinkRecord {
    LinkRecord {
        id: link.id,
        subdomain: link.subdomain,
        target_host: link.target_host,
        target_port: link.target_port,
    }
}
