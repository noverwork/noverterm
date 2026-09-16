use std::time::Duration;

use russh::keys::ssh_key::rand_core::OsRng;
use russh::keys::{Algorithm, HashAlg, PrivateKey};
use russh::{server, Channel, ChannelId, ChannelMsg};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use super::*;
use crate::store::test_pool;
use crate::trust::HostTrustConfirmation;

#[derive(Default)]
struct TestServer {
    channels: HashMap<ChannelId, Channel<server::Msg>>,
}

impl server::Handler for TestServer {
    type Error = russh::Error;

    async fn auth_none(&mut self, _user: &str) -> Result<server::Auth, Self::Error> {
        Ok(server::Auth::Accept)
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<server::Msg>,
        _session: &mut server::Session,
    ) -> Result<bool, Self::Error> {
        self.channels.insert(channel.id(), channel);
        Ok(true)
    }

    async fn subsystem_request(
        &mut self,
        channel: ChannelId,
        name: &str,
        session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        assert_eq!(name, "sftp");
        session.channel_success(channel)?;
        let channel = self.channels.remove(&channel).expect("SFTP channel");
        russh_sftp::server::run(channel.into_stream(), TestSftp).await;
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        if self.channels.contains_key(&channel) {
            session.data(channel, data.to_vec())?;
        }
        Ok(())
    }
}

struct TestSftp;

impl russh_sftp::server::Handler for TestSftp {
    type Error = russh_sftp::protocol::StatusCode;

    fn unimplemented(&self) -> Self::Error {
        russh_sftp::protocol::StatusCode::OpUnsupported
    }
}

struct Fixture {
    manager: SshSessionManager,
    sftp_id: String,
    handle: Arc<Mutex<client::Handle<ClientHandler>>>,
    server: JoinHandle<()>,
    keepalive_stopped: oneshot::Receiver<()>,
    // A terminal owns its transport independently from SFTP.
    _terminal_transport: Option<DirectSftpTransport>,
    _directory: tempfile::TempDir,
}

async fn fixture(direct: bool) -> Fixture {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("listen");
    let port = listener.local_addr().expect("address").port();
    let key = PrivateKey::random(&mut OsRng, Algorithm::Ed25519).expect("server key");
    let directory = tempfile::tempdir().expect("database directory");
    let trust_store = SshTrustStore::new(test_pool(&directory));
    trust_store
        .confirm(HostTrustConfirmation {
            host: "127.0.0.1".to_string(),
            port,
            algorithm: key.public_key().algorithm().to_string(),
            fingerprint: key.public_key().fingerprint(HashAlg::Sha256).to_string(),
        })
        .await
        .expect("trust test server");
    let config = Arc::new(server::Config {
        keys: vec![key],
        ..Default::default()
    });
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let session = server::run_stream(config, stream, TestServer::default())
            .await
            .expect("server connection");
        // Forced TCP shutdown may surface as EOF rather than a clean SSH disconnect.
        let _ = session.await;
    });
    let (stream, transport) = DirectSftpTransport::connect("127.0.0.1", port, true)
        .await
        .expect("client socket");
    let handler = ClientHandler::new(
        "127.0.0.1".to_string(),
        port,
        trust_store,
        Arc::new(Mutex::new(None)),
    );
    let mut handle = client::connect_stream(client_config(None), stream, handler)
        .await
        .expect("client connection");
    assert!(handle
        .authenticate_none("test")
        .await
        .expect("authenticate")
        .success());
    let sftp = open_sftp_session(&mut handle).await.expect("SFTP session");
    let sftp_id = sftp.id().to_string();
    let handle = Arc::new(Mutex::new(handle));
    let (keepalive_sender, keepalive_stopped) = oneshot::channel();
    let keepalive_handle = handle.clone();
    let keepalive_task = tokio::spawn(async move {
        std::future::pending::<()>().await;
        drop((keepalive_handle, keepalive_sender));
    });
    let (direct_transport, terminal_transport) = if direct {
        (Some(transport), None)
    } else {
        (None, Some(transport))
    };
    let manager = SshSessionManager::new();
    manager.sessions.lock().await.insert(
        "ssh".to_string(),
        SshSession {
            handle: handle.clone(),
            write_tx: None,
            sftp_sessions: HashMap::from([(sftp_id.clone(), Arc::new(sftp))]),
            port_forwards: HashMap::new(),
            keepalive_task: Some(keepalive_task),
            writer_task: None,
            output: None,
            direct_sftp_transport: direct_transport,
        },
    );
    Fixture {
        manager,
        sftp_id,
        handle,
        server,
        keepalive_stopped,
        _terminal_transport: terminal_transport,
        _directory: directory,
    }
}

#[tokio::test]
async fn close_missing_sftp_is_idempotent() {
    let manager = SshSessionManager::new();
    assert_eq!(manager.close_sftp("expired").await, Ok(()));
    assert_eq!(manager.close_sftp("expired").await, Ok(()));
}

#[tokio::test]
async fn close_direct_sftp_releases_ssh_and_keepalive() {
    let fixture = fixture(true).await;
    fixture
        .manager
        .close_sftp(&fixture.sftp_id)
        .await
        .expect("close");
    assert!(!fixture.manager.contains_session("ssh").await);
    assert!(
        !fixture
            .manager
            .contains_sftp_session(&fixture.sftp_id)
            .await
    );
    assert!(timeout(Duration::from_secs(1), fixture.keepalive_stopped)
        .await
        .expect("keepalive must stop")
        .is_err());
    timeout(Duration::from_secs(1), fixture.server)
        .await
        .expect("SSH transport must stop")
        .expect("server task");
    assert_eq!(fixture.manager.close_sftp(&fixture.sftp_id).await, Ok(()));
}

#[tokio::test]
async fn close_sftp_preserves_terminal_channel() {
    let fixture = fixture(false).await;
    let mut shell = fixture
        .handle
        .lock()
        .await
        .channel_open_session()
        .await
        .expect("shell");
    fixture
        .manager
        .close_sftp(&fixture.sftp_id)
        .await
        .expect("close SFTP");
    assert!(fixture.manager.contains_session("ssh").await);
    assert!(
        !fixture
            .manager
            .contains_sftp_session(&fixture.sftp_id)
            .await
    );
    shell
        .data(&b"still connected"[..])
        .await
        .expect("send shell data");
    let response = timeout(Duration::from_secs(1), shell.wait())
        .await
        .expect("shell response");
    assert!(
        matches!(response, Some(ChannelMsg::Data { data }) if data.as_ref() == b"still connected")
    );
    assert!(!fixture.server.is_finished());

    let mut parent = fixture
        .manager
        .sessions
        .lock()
        .await
        .remove("ssh")
        .expect("parent");
    parent.stop_runtime_tasks(None, true, true);
    fixture
        .handle
        .lock()
        .await
        .disconnect(Disconnect::ByApplication, "test done", "")
        .await
        .expect("disconnect");
    timeout(Duration::from_secs(1), fixture.server)
        .await
        .expect("server stopped")
        .expect("server task");
}

#[tokio::test]
async fn blocked_direct_disconnect_retires_ownership_and_forces_socket_closed() {
    let fixture = fixture(true).await;
    let _blocked_handle = fixture.handle.lock().await;
    let manager = fixture.manager.clone();
    let sftp_id = fixture.sftp_id.clone();
    let close = tokio::spawn(async move { manager.close_sftp(&sftp_id).await });

    timeout(Duration::from_secs(1), async {
        while fixture.manager.contains_session("ssh").await {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("manager lock must not wait for transport shutdown");
    let result = timeout(SSH_KEEPALIVE_TIMEOUT + Duration::from_secs(1), close)
        .await
        .expect("close must be bounded")
        .expect("close task");
    assert!(result.is_err());
    assert!(
        !fixture
            .manager
            .contains_sftp_session(&fixture.sftp_id)
            .await
    );
    assert_eq!(fixture.manager.close_sftp(&fixture.sftp_id).await, Ok(()));
    timeout(Duration::from_secs(1), fixture.server)
        .await
        .expect("socket must close even while handle is locked")
        .expect("server task");
    assert!(timeout(Duration::from_secs(1), fixture.keepalive_stopped)
        .await
        .expect("keepalive must stop")
        .is_err());
}
