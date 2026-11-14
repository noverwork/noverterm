use tracing_subscriber::EnvFilter;

use crate::app::AppHandle;
use crate::runtime::local::LocalSessionManager;
use crate::runtime::port_forward::PortForwardManager;
use crate::runtime::ssh::SshSessionManager;
use crate::runtime::terminal_output::TerminalOutput;
use crate::sftp::state::TransferState;
use crate::trust::SshTrustStore;

/// Opens the database, registers every backend service, and starts the MCP
/// server.
pub fn backend() -> Result<AppHandle, String> {
    std::fs::create_dir_all(crate::app::data_dir()?).map_err(|error| error.to_string())?;
    let database = crate::store::database_path()?;
    if let Err(error) = crate::store::apply_pending_restore(&database) {
        tracing::error!("{error}");
    }
    let pool = crate::store::init_pool(&database)?;

    let app = AppHandle::default();
    app.manage(SshTrustStore::new(pool.clone()));
    app.manage(pool);
    app.manage(TerminalOutput::default());
    app.manage(SshSessionManager::new());
    app.manage(LocalSessionManager::new());
    app.manage(PortForwardManager::new());
    app.manage(crate::runtime::link::LinkManager::new());
    app.manage(TransferState::new());
    app.manage(crate::mcp::McpState::default());
    crate::app::runtime().spawn(crate::mcp::start(app.clone()));
    Ok(app)
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let app = backend().expect("failed to start the backend");
    crate::ui::run(app);
}
