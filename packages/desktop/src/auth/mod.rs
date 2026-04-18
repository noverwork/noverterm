mod backend_client;
mod session;
mod token_store;

use tauri::State;

pub(crate) use backend_client::BackendConnectAuthMaterial;
pub use session::{
    AuthBootstrapStatus, BootstrapMetadata, DesktopAuthManager, SaveConnectionInput,
};

#[derive(Debug, serde::Deserialize, specta::Type)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[tauri::command]
#[specta::specta]
pub async fn auth_login(
    login: LoginInput,
    auth_manager: State<'_, DesktopAuthManager>,
) -> Result<AuthBootstrapStatus, String> {
    auth_manager.login(login.username, login.password).await
}

#[tauri::command]
#[specta::specta]
pub async fn auth_logout(auth_manager: State<'_, DesktopAuthManager>) -> Result<(), String> {
    auth_manager.logout().await
}

#[cfg(test)]
mod tests;
