use crate::db::{DbPool, models::*, repository::*};

// ============================================================================
// Group Commands
// ============================================================================

#[tauri::command]
pub async fn db_get_groups(pool: tauri::State<'_, DbPool>) -> Result<Vec<Group>, String> {
    get_groups(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_create_group(pool: tauri::State<'_, DbPool>, input: CreateGroup) -> Result<Group, String> {
    create_group(pool.inner(), input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_update_group(pool: tauri::State<'_, DbPool>, id: String, input: UpdateGroup) -> Result<Group, String> {
    update_group(pool.inner(), &id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_delete_group(pool: tauri::State<'_, DbPool>, id: String) -> Result<(), String> {
    delete_group(pool.inner(), &id)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// SSH Key Commands
// ============================================================================

#[tauri::command]
pub async fn db_get_ssh_keys(pool: tauri::State<'_, DbPool>) -> Result<Vec<SshKey>, String> {
    get_ssh_keys(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_get_ssh_key(pool: tauri::State<'_, DbPool>, id: String) -> Result<SshKey, String> {
    get_ssh_key(pool.inner(), &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_create_ssh_key(pool: tauri::State<'_, DbPool>, input: CreateSshKey) -> Result<SshKey, String> {
    create_ssh_key(pool.inner(), input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_update_ssh_key(pool: tauri::State<'_, DbPool>, id: String, input: UpdateSshKey) -> Result<SshKey, String> {
    update_ssh_key(pool.inner(), &id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_delete_ssh_key(pool: tauri::State<'_, DbPool>, id: String) -> Result<(), String> {
    delete_ssh_key(pool.inner(), &id)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Session Commands
// ============================================================================

#[tauri::command]
pub async fn db_get_all_sessions(pool: tauri::State<'_, DbPool>) -> Result<Vec<Session>, String> {
    get_sessions(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_get_session(pool: tauri::State<'_, DbPool>, id: String) -> Result<Session, String> {
    get_session(pool.inner(), &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_create_session(pool: tauri::State<'_, DbPool>, input: CreateSession) -> Result<Session, String> {
    create_session(pool.inner(), input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_update_session(pool: tauri::State<'_, DbPool>, id: String, input: UpdateSession) -> Result<Session, String> {
    update_session(pool.inner(), &id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_delete_session(pool: tauri::State<'_, DbPool>, id: String) -> Result<(), String> {
    delete_session(pool.inner(), &id)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Port Forward Commands
// ============================================================================

#[tauri::command]
pub async fn db_get_port_forwards(pool: tauri::State<'_, DbPool>, session_id: Option<String>) -> Result<Vec<PortForward>, String> {
    get_port_forwards(pool.inner(), session_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_get_port_forward(pool: tauri::State<'_, DbPool>, id: String) -> Result<PortForward, String> {
    get_port_forward(pool.inner(), &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_create_port_forward(pool: tauri::State<'_, DbPool>, input: CreatePortForward) -> Result<PortForward, String> {
    create_port_forward(pool.inner(), input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_update_port_forward(pool: tauri::State<'_, DbPool>, id: String, input: UpdatePortForward) -> Result<PortForward, String> {
    update_port_forward(pool.inner(), &id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_delete_port_forward(pool: tauri::State<'_, DbPool>, id: String) -> Result<(), String> {
    delete_port_forward(pool.inner(), &id)
        .await
        .map_err(|e| e.to_string())
}
