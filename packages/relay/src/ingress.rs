//! HTTP side: visitors (through Caddy) are routed by Host to a tunnel, and
//! Caddy asks which hosts may get a certificate.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use tokio::io::{copy_bidirectional, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tracing::{debug, info};

use crate::routes;
use crate::ssh::State;

const MAX_HEAD: usize = 16 * 1024;
const HEAD_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn serve_visitors(port: u16, base_domain: Arc<str>, state: Arc<State>) -> Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    info!(port, "HTTP ingress listening");
    loop {
        let (stream, peer) = listener.accept().await?;
        let (state, base_domain) = (state.clone(), base_domain.clone());
        tokio::spawn(async move {
            if let Err(error) = visit(stream, peer, &base_domain, &state).await {
                debug!(%peer, %error, "visitor connection ended");
            }
        });
    }
}

async fn visit(
    mut stream: TcpStream,
    peer: SocketAddr,
    base_domain: &str,
    state: &State,
) -> Result<()> {
    let head = read_head(&mut stream).await?;
    let host = header(&head, "host").unwrap_or_default();
    let Some(target) =
        routes::subdomain_of_host(&host, base_domain).and_then(|name| state.routes.get(&name))
    else {
        return respond(
            &mut stream,
            "404 Not Found",
            &format!("No tunnel is running at {host}."),
        )
        .await;
    };
    let channel = match target
        .handle
        .channel_open_forwarded_tcpip(
            target.address,
            target.port,
            peer.ip().to_string(),
            u32::from(peer.port()),
        )
        .await
    {
        Ok(channel) => channel,
        Err(error) => {
            respond(
                &mut stream,
                "503 Service Unavailable",
                "The tunnel is connected but did not answer.",
            )
            .await?;
            bail!("opening forwarded-tcpip channel: {error}");
        }
    };
    let mut upstream = channel.into_stream();
    // One request per connection: a proxy in front reuses upstream connections
    // across hosts, and routing only looks at the first request.
    upstream.write_all(&close_after_response(&head)).await?;
    copy_bidirectional(&mut stream, &mut upstream).await?;
    Ok(())
}

/// Caddy's on-demand TLS `ask` endpoint: 200 only for hosts with a live tunnel,
/// so certificates are never requested for names nobody uses.
pub async fn serve_tls_ask(port: u16, base_domain: Arc<str>, state: Arc<State>) -> Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    info!(port, "TLS ask endpoint listening");
    loop {
        let (mut stream, _) = listener.accept().await?;
        let (state, base_domain) = (state.clone(), base_domain.clone());
        tokio::spawn(async move {
            let Ok(head) = read_head(&mut stream).await else {
                return;
            };
            let live = ask_domain(&head)
                .and_then(|domain| routes::subdomain_of_host(&domain, &base_domain))
                .is_some_and(|name| state.routes.get(&name).is_some());
            let status = if live { "200 OK" } else { "404 Not Found" };
            let _ = respond(&mut stream, status, "").await;
        });
    }
}

/// Reads up to the end of the request headers (plus whatever came with them).
async fn read_head(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut head = Vec::new();
    let mut buffer = [0_u8; 4096];
    while !head.windows(4).any(|window| window == b"\r\n\r\n") {
        if head.len() > MAX_HEAD {
            bail!("request headers too large");
        }
        let read = timeout(HEAD_TIMEOUT, stream.read(&mut buffer))
            .await
            .context("timed out reading request")??;
        if read == 0 {
            bail!("connection closed before headers ended");
        }
        head.extend_from_slice(&buffer[..read]);
    }
    Ok(head)
}

fn header(head: &[u8], name: &str) -> Option<String> {
    String::from_utf8_lossy(head)
        .split("\r\n\r\n")
        .next()?
        .lines()
        .skip(1)
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case(name)
                .then(|| value.trim().to_string())
        })
}

/// `domain` from `GET /tls-check?domain=<host> HTTP/1.1`.
fn ask_domain(head: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(head);
    let target = text.lines().next()?.split_whitespace().nth(1)?;
    let query = target.strip_prefix("/tls-check?")?;
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("domain="))
        .map(str::to_string)
}

/// Replaces or adds `Connection: close` in the request headers. Upgrades
/// (WebSocket) keep theirs: the connection becomes the socket.
fn close_after_response(head: &[u8]) -> Vec<u8> {
    let upgrade = header(head, "connection")
        .is_some_and(|value| value.to_ascii_lowercase().contains("upgrade"));
    let Some(end) = head
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .filter(|_| !upgrade)
    else {
        return head.to_vec();
    };
    let Ok(headers) = std::str::from_utf8(&head[..end]) else {
        return head.to_vec();
    };
    let mut out: Vec<&str> = headers
        .split("\r\n")
        .filter(|line| {
            !line
                .split_once(':')
                .is_some_and(|(key, _)| key.trim().eq_ignore_ascii_case("connection"))
        })
        .collect();
    out.push("Connection: close");
    let mut bytes = out.join("\r\n").into_bytes();
    bytes.extend_from_slice(&head[end..]);
    bytes
}

async fn respond(stream: &mut TcpStream, status: &str, message: &str) -> Result<()> {
    let body = if message.is_empty() {
        String::new()
    } else {
        shared::error_page(status, message)
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_host_header() {
        let head = b"GET / HTTP/1.1\r\nHOST: app.example.com\r\nAccept: */*\r\n\r\n";
        assert_eq!(header(head, "host"), Some("app.example.com".into()));
        assert_eq!(header(head, "cookie"), None);
    }

    #[test]
    fn reads_ask_domain() {
        assert_eq!(
            ask_domain(b"GET /tls-check?domain=app.example.com HTTP/1.1\r\n\r\n"),
            Some("app.example.com".into())
        );
        assert_eq!(ask_domain(b"GET /other?domain=x HTTP/1.1\r\n\r\n"), None);
    }

    #[test]
    fn forces_connection_close_and_keeps_body() {
        let head = b"POST / HTTP/1.1\r\nHost: a\r\nConnection: keep-alive\r\n\r\nbody";
        assert_eq!(
            close_after_response(head),
            b"POST / HTTP/1.1\r\nHost: a\r\nConnection: close\r\n\r\nbody"
        );
        assert_eq!(
            close_after_response(b"GET / HTTP/1.1\r\nHost: a\r\n\r\n"),
            b"GET / HTTP/1.1\r\nHost: a\r\nConnection: close\r\n\r\n"
        );
        let websocket: &[u8] =
            b"GET /ws HTTP/1.1\r\nConnection: Upgrade\r\nUpgrade: websocket\r\n\r\n";
        assert_eq!(close_after_response(websocket), websocket);
    }
}
