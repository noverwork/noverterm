//! Runs one non-interactive command on a host and returns its output, for
//! callers such as the MCP server that need a clean exit code rather than a
//! terminal screen.

use std::time::Duration;

use russh::{ChannelMsg, Disconnect};
use tokio::time::timeout;
use uuid::Uuid;

use crate::trust::{SshTrustStore, TrustCheck};

use super::authentication::{authenticate_session, client_config};
use super::jump::{open_transport, OpenTransportError, SshHopInput};
use super::AuthMethod;

/// Each stream keeps at most this much; the rest is dropped and flagged.
const OUTPUT_LIMIT: usize = 64 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

pub struct ExecTarget {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: AuthMethod,
    pub jumps: Vec<SshHopInput>,
}

#[derive(Debug, Default)]
pub struct ExecOutput {
    pub exit_status: Option<u32>,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
}

// ponytail: one SSH connection per command; pool them if agents run many in a row.
pub async fn exec_command(
    trust_store: &SshTrustStore,
    target: ExecTarget,
    command: &str,
    run_timeout: Duration,
) -> Result<ExecOutput, String> {
    let exec_id = Uuid::new_v4().to_string();
    let config = client_config(None);
    let transport = timeout(
        CONNECT_TIMEOUT,
        open_transport(
            config,
            &target.host,
            target.port,
            target.jumps,
            trust_store,
            &exec_id,
        ),
    )
    .await
    .map_err(|_| format!("Timed out connecting to {}", target.host))?;

    let mut session = match transport {
        Ok(session) => session,
        Err(OpenTransportError::Connect { error, trust_check }) => {
            return Err(match trust_check.lock().await.clone() {
                Some(TrustCheck::TrustRequired(prompt)) => format!(
                    "The host key of {}:{} is not trusted yet. Connect to it once in noverterm to verify and trust it.",
                    prompt.host, prompt.port
                ),
                Some(TrustCheck::TrustMismatch(mismatch)) => format!(
                    "The host key of {}:{} changed since it was trusted. Refusing to connect; check it in noverterm.",
                    mismatch.host, mismatch.port
                ),
                _ => format!("Failed to connect: {error}"),
            });
        }
        Err(OpenTransportError::Other(error)) => return Err(error),
    };
    authenticate_session(&mut session, &target.username, target.auth, &exec_id).await?;

    let mut channel = session
        .channel_open_session()
        .await
        .map_err(|error| format!("Failed to open a channel: {error}"))?;
    channel
        .exec(true, command)
        .await
        .map_err(|error| format!("Failed to start the command: {error}"))?;

    let mut output = ExecOutput::default();
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let collect = async {
        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::Data { data } => append(&mut stdout, &data, &mut output.truncated),
                ChannelMsg::ExtendedData { data, .. } => {
                    append(&mut stderr, &data, &mut output.truncated)
                }
                ChannelMsg::ExitStatus { exit_status } => output.exit_status = Some(exit_status),
                ChannelMsg::Close => break,
                _ => {}
            }
        }
    };
    let finished = timeout(run_timeout, collect).await.is_ok();
    let _ = session
        .disconnect(Disconnect::ByApplication, "Command finished", "")
        .await;
    if !finished {
        return Err(format!(
            "The command did not finish within {} seconds and was stopped",
            run_timeout.as_secs()
        ));
    }

    output.stdout = String::from_utf8_lossy(&stdout).into_owned();
    output.stderr = String::from_utf8_lossy(&stderr).into_owned();
    Ok(output)
}

fn append(buffer: &mut Vec<u8>, data: &[u8], truncated: &mut bool) {
    let room = OUTPUT_LIMIT.saturating_sub(buffer.len());
    if data.len() > room {
        *truncated = true;
    }
    buffer.extend_from_slice(&data[..data.len().min(room)]);
}
