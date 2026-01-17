use super::models::*;
use super::{DbError, DbResult, DbPool};
use uuid::Uuid;

// ============================================================================
// Group Repository
// ============================================================================

pub async fn get_groups(pool: &DbPool) -> DbResult<Vec<Group>> {
    let groups = sqlx::query_as::<_, Group>("SELECT * FROM groups ORDER BY name")
        .fetch_all(pool)
        .await?;
    Ok(groups)
}

pub async fn get_group(pool: &DbPool, id: &str) -> DbResult<Group> {
    let group = sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("Group {} not found", id)))?;
    Ok(group)
}

pub async fn create_group(pool: &DbPool, input: CreateGroup) -> DbResult<Group> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO groups (id, name, color) VALUES (?, ?, ?)"
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.color)
    .execute(pool)
    .await?;

    get_group(pool, &id).await
}

pub async fn update_group(pool: &DbPool, id: &str, input: UpdateGroup) -> DbResult<Group> {
    let mut query = String::from("UPDATE groups SET ");
    let mut updates = Vec::new();
    let mut bind_index = 1;

    if input.name.is_some() {
        updates.push(format!("name = ${}", bind_index));
        bind_index += 1;
    }
    if input.color.is_some() {
        updates.push(format!("color = ${}", bind_index));
        bind_index += 1;
    }

    if updates.is_empty() {
        return get_group(pool, id).await;
    }

    query.push_str(&updates.join(", "));
    query.push_str(&format!(" WHERE id = ${}", bind_index));

    let mut q = sqlx::query(&query);

    if let Some(name) = input.name {
        q = q.bind(name);
    }
    if let Some(color) = input.color {
        q = q.bind(color);
    }
    q = q.bind(id);

    q.execute(pool).await?;

    get_group(pool, id).await
}

pub async fn delete_group(pool: &DbPool, id: &str) -> DbResult<()> {
    let result = sqlx::query("DELETE FROM groups WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("Group {} not found", id)));
    }
    Ok(())
}

// ============================================================================
// SSH Key Repository
// ============================================================================

pub async fn get_ssh_keys(pool: &DbPool) -> DbResult<Vec<SshKey>> {
    let keys = sqlx::query_as::<_, SshKey>("SELECT * FROM ssh_keys ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    Ok(keys)
}

pub async fn get_ssh_key(pool: &DbPool, id: &str) -> DbResult<SshKey> {
    let key = sqlx::query_as::<_, SshKey>("SELECT * FROM ssh_keys WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("SSH Key {} not found", id)))?;
    Ok(key)
}

pub async fn create_ssh_key(pool: &DbPool, input: CreateSshKey) -> DbResult<SshKey> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO ssh_keys (id, name, type, public_key, private_key_path, fingerprint, has_passphrase, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.key_type)
    .bind(&input.public_key)
    .bind(&input.private_key_path)
    .bind(&input.fingerprint)
    .bind(input.has_passphrase as i64)
    .bind(now)
    .execute(pool)
    .await?;

    get_ssh_key(pool, &id).await
}

pub async fn update_ssh_key(pool: &DbPool, id: &str, input: UpdateSshKey) -> DbResult<SshKey> {
    if let Some(name) = input.name {
        sqlx::query("UPDATE ssh_keys SET name = ? WHERE id = ?")
            .bind(&name)
            .bind(id)
            .execute(pool)
            .await?;
    }
    get_ssh_key(pool, id).await
}

pub async fn delete_ssh_key(pool: &DbPool, id: &str) -> DbResult<()> {
    let result = sqlx::query("DELETE FROM ssh_keys WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("SSH Key {} not found", id)));
    }
    Ok(())
}

// ============================================================================
// Session Repository
// ============================================================================

pub async fn get_sessions(pool: &DbPool) -> DbResult<Vec<Session>> {
    let sessions = sqlx::query_as::<_, Session>("SELECT * FROM sessions ORDER BY name")
        .fetch_all(pool)
        .await?;
    Ok(sessions)
}

pub async fn get_session(pool: &DbPool, id: &str) -> DbResult<Session> {
    let session = sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("Session {} not found", id)))?;
    Ok(session)
}

pub async fn create_session(pool: &DbPool, input: CreateSession) -> DbResult<Session> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO sessions (id, name, group_id, host, port, username, auth_method, key_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.group_id)
    .bind(&input.host)
    .bind(input.port)
    .bind(&input.username)
    .bind(&input.auth_method)
    .bind(&input.key_id)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    get_session(pool, &id).await
}

pub async fn update_session(pool: &DbPool, id: &str, input: UpdateSession) -> DbResult<Session> {
    let mut query = String::from("UPDATE sessions SET updated_at = ?");
    let mut updates = Vec::new();
    let mut bind_index = 2;
    let mut has_updates = false;

    if input.name.is_some() {
        updates.push(format!("name = ${}", bind_index));
        bind_index += 1;
    }
    if input.group_id.is_some() {
        updates.push(format!("group_id = ${}", bind_index));
        bind_index += 1;
    }
    if input.host.is_some() {
        updates.push(format!("host = ${}", bind_index));
        bind_index += 1;
    }
    if input.port.is_some() {
        updates.push(format!("port = ${}", bind_index));
        bind_index += 1;
    }
    if input.username.is_some() {
        updates.push(format!("username = ${}", bind_index));
        bind_index += 1;
    }
    if input.auth_method.is_some() {
        updates.push(format!("auth_method = ${}", bind_index));
        bind_index += 1;
    }
    if input.key_id.is_some() {
        updates.push(format!("key_id = ${}", bind_index));
        bind_index += 1;
    }

    if updates.is_empty() {
        return get_session(pool, id).await;
    }

    query.push_str(&updates.join(", "));
    query.push_str(&format!(" WHERE id = ${}", bind_index));

    let now = chrono::Utc::now().timestamp();
    let mut q = sqlx::query(&query).bind(now);

    if let Some(name) = input.name {
        q = q.bind(name);
    }
    if let Some(group_id) = input.group_id {
        q = q.bind(group_id);
    }
    if let Some(host) = input.host {
        q = q.bind(host);
    }
    if let Some(port) = input.port {
        q = q.bind(port);
    }
    if let Some(username) = input.username {
        q = q.bind(username);
    }
    if let Some(auth_method) = input.auth_method {
        q = q.bind(auth_method);
    }
    if let Some(key_id) = input.key_id {
        q = q.bind(key_id);
    }
    q = q.bind(id);

    q.execute(pool).await?;

    get_session(pool, id).await
}

pub async fn delete_session(pool: &DbPool, id: &str) -> DbResult<()> {
    let result = sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("Session {} not found", id)));
    }
    Ok(())
}

// ============================================================================
// Port Forward Repository
// ============================================================================

pub async fn get_port_forwards(pool: &DbPool, session_id: Option<&str>) -> DbResult<Vec<PortForward>> {
    let forwards = if let Some(sid) = session_id {
        sqlx::query_as::<_, PortForward>("SELECT * FROM port_forwards WHERE session_id = ? ORDER BY name")
            .bind(sid)
            .fetch_all(pool)
            .await?
    } else {
        sqlx::query_as::<_, PortForward>("SELECT * FROM port_forwards ORDER BY name")
            .fetch_all(pool)
            .await?
    };
    Ok(forwards)
}

pub async fn get_port_forward(pool: &DbPool, id: &str) -> DbResult<PortForward> {
    let forward = sqlx::query_as::<_, PortForward>("SELECT * FROM port_forwards WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("Port forward {} not found", id)))?;
    Ok(forward)
}

pub async fn create_port_forward(pool: &DbPool, input: CreatePortForward) -> DbResult<PortForward> {
    let id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO port_forwards (id, session_id, name, type, local_host, local_port, remote_host, remote_port)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&input.session_id)
    .bind(&input.name)
    .bind(&input.forward_type)
    .bind(&input.local_host)
    .bind(input.local_port)
    .bind(&input.remote_host)
    .bind(input.remote_port)
    .execute(pool)
    .await?;

    get_port_forward(pool, &id).await
}

pub async fn update_port_forward(pool: &DbPool, id: &str, input: UpdatePortForward) -> DbResult<PortForward> {
    let mut query = String::from("UPDATE port_forwards SET ");
    let mut updates = Vec::new();
    let mut bind_index = 1;

    if input.name.is_some() {
        updates.push(format!("name = ${}", bind_index));
        bind_index += 1;
    }
    if input.forward_type.is_some() {
        updates.push(format!("type = ${}", bind_index));
        bind_index += 1;
    }
    if input.local_host.is_some() {
        updates.push(format!("local_host = ${}", bind_index));
        bind_index += 1;
    }
    if input.local_port.is_some() {
        updates.push(format!("local_port = ${}", bind_index));
        bind_index += 1;
    }
    if input.remote_host.is_some() {
        updates.push(format!("remote_host = ${}", bind_index));
        bind_index += 1;
    }
    if input.remote_port.is_some() {
        updates.push(format!("remote_port = ${}", bind_index));
        bind_index += 1;
    }

    if updates.is_empty() {
        return get_port_forward(pool, id).await;
    }

    query.push_str(&updates.join(", "));
    query.push_str(&format!(" WHERE id = ${}", bind_index));

    let mut q = sqlx::query(&query);

    if let Some(name) = input.name {
        q = q.bind(name);
    }
    if let Some(forward_type) = input.forward_type {
        q = q.bind(forward_type);
    }
    if let Some(local_host) = input.local_host {
        q = q.bind(local_host);
    }
    if let Some(local_port) = input.local_port {
        q = q.bind(local_port);
    }
    if let Some(remote_host) = input.remote_host {
        q = q.bind(remote_host);
    }
    if let Some(remote_port) = input.remote_port {
        q = q.bind(remote_port);
    }
    q = q.bind(id);

    q.execute(pool).await?;

    get_port_forward(pool, id).await
}

pub async fn delete_port_forward(pool: &DbPool, id: &str) -> DbResult<()> {
    let result = sqlx::query("DELETE FROM port_forwards WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("Port forward {} not found", id)));
    }
    Ok(())
}
