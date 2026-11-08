//! The forward proxy that headless Chrome sends all of its traffic through.
//!
//! The DevTools request guard in `lib.rs` only sees ordinary page requests. WebSocket
//! handshakes, service workers and WebRTC skip it, and Chrome does its own DNS, so a name
//! can change between that check and the connection. Chrome therefore runs on a network
//! with no route out (`internal: true` in compose) and reaches the web only through this
//! proxy, which resolves every host itself, refuses private addresses and connects to
//! exactly the addresses it checked.
//!
//! It speaks plain HTTP/1.1 forward proxying: `CONNECT host:port` tunnels (HTTPS, and every
//! WebSocket) and absolute-form requests for `http://` URLs.

use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::{Bytes, Incoming};
use hyper::header::{HeaderValue, HOST};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode, Uri};
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tracing::{info, warn};

/// Connections served at once. Chrome opens a few per page, and two pages render at a time.
const MAX_CONNECTIONS: usize = 256;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// A screenshot is done within half a minute, so no tunnel needs to outlive this.
const MAX_TUNNEL_LIFETIME: Duration = Duration::from_secs(120);
const RESPONSE_HEAD_TIMEOUT: Duration = Duration::from_secs(30);

type Body = BoxBody<Bytes, hyper::Error>;

/// Turns `host` and `port` into the addresses to connect to, or `PermissionDenied`.
pub type Resolve = Arc<
    dyn Fn(String, u16) -> Pin<Box<dyn Future<Output = io::Result<Vec<SocketAddr>>> + Send>>
        + Send
        + Sync,
>;

/// The production resolver: public addresses only, see [`crate::net_guard`].
pub fn public_resolver() -> Resolve {
    Arc::new(|host, port| {
        Box::pin(async move { crate::net_guard::public_socket_addrs(&host, port).await })
    })
}

/// Ports a page may reach. Web servers live on 80, 443 and high ports; the other well-known
/// ports (mail, SSH, databases) are never something a rendered page needs.
fn allowed_port(port: u16) -> bool {
    port == 80 || port == 443 || port >= 1024
}

/// Serves the proxy on `listener` until the process exits.
pub async fn serve(listener: TcpListener, resolve: Resolve) {
    let slots = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    if let Ok(addr) = listener.local_addr() {
        info!(%addr, "browser egress proxy listening");
    }
    loop {
        let (stream, _) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                warn!("egress proxy accept: {e}");
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        let Ok(slot) = slots.clone().acquire_owned().await else {
            return;
        };
        let resolve = resolve.clone();
        tokio::spawn(async move {
            let service = service_fn(move |req| handle(req, resolve.clone()));
            let conn = hyper::server::conn::http1::Builder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(TokioIo::new(stream), service)
                .with_upgrades();
            let _ = conn.await;
            drop(slot);
        });
    }
}

async fn handle(req: Request<Incoming>, resolve: Resolve) -> Result<Response<Body>, hyper::Error> {
    if req.method() == Method::CONNECT {
        return Ok(tunnel(req, &resolve).await);
    }
    forward(req, &resolve).await
}

/// `CONNECT host:port`: connect, answer 200, then copy bytes both ways.
async fn tunnel(req: Request<Incoming>, resolve: &Resolve) -> Response<Body> {
    let Some((host, port)) = req
        .uri()
        .authority()
        .and_then(|a| a.port_u16().map(|p| (a.host().to_string(), p)))
    else {
        return status(StatusCode::BAD_REQUEST);
    };
    let upstream = match connect(&host, port, resolve).await {
        Ok(stream) => stream,
        Err(code) => return status(code),
    };
    tokio::spawn(async move {
        let client = match hyper::upgrade::on(req).await {
            Ok(upgraded) => upgraded,
            Err(e) => {
                warn!("egress proxy upgrade: {e}");
                return;
            }
        };
        let mut client = TokioIo::new(client);
        let mut upstream = upstream;
        let copy = tokio::io::copy_bidirectional(&mut client, &mut upstream);
        let _ = tokio::time::timeout(MAX_TUNNEL_LIFETIME, copy).await;
    });
    status(StatusCode::OK)
}

/// An absolute-form `http://` request: send it to the origin in origin form, one request
/// per upstream connection, and pass the response back.
async fn forward(
    mut req: Request<Incoming>,
    resolve: &Resolve,
) -> Result<Response<Body>, hyper::Error> {
    let uri = req.uri().clone();
    if uri.scheme_str() != Some("http") {
        return Ok(status(StatusCode::BAD_REQUEST));
    }
    let Some(host) = uri.host().map(str::to_string) else {
        return Ok(status(StatusCode::BAD_REQUEST));
    };
    let port = uri.port_u16().unwrap_or(80);
    let upstream = match connect(&host, port, resolve).await {
        Ok(stream) => stream,
        Err(code) => return Ok(status(code)),
    };

    let path = uri
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or("/")
        .parse::<Uri>()
        .unwrap_or_else(|_| Uri::from_static("/"));
    *req.uri_mut() = path;
    let headers = req.headers_mut();
    for name in [
        "proxy-connection",
        "proxy-authorization",
        "connection",
        "keep-alive",
    ] {
        headers.remove(name);
    }
    if !headers.contains_key(HOST) {
        if let Some(authority) = uri
            .authority()
            .and_then(|a| HeaderValue::from_str(a.as_str()).ok())
        {
            headers.insert(HOST, authority);
        }
    }

    let (mut sender, conn) =
        match hyper::client::conn::http1::handshake(TokioIo::new(upstream)).await {
            Ok(pair) => pair,
            Err(e) => {
                warn!("egress proxy upstream handshake: {e}");
                return Ok(status(StatusCode::BAD_GATEWAY));
            }
        };
    tokio::spawn(async move {
        let _ = tokio::time::timeout(MAX_TUNNEL_LIFETIME, conn).await;
    });
    let resp = match tokio::time::timeout(RESPONSE_HEAD_TIMEOUT, sender.send_request(req)).await {
        Ok(Ok(resp)) => resp,
        Ok(Err(e)) => {
            warn!("egress proxy upstream request: {e}");
            return Ok(status(StatusCode::BAD_GATEWAY));
        }
        Err(_) => return Ok(status(StatusCode::GATEWAY_TIMEOUT)),
    };
    Ok(resp.map(|body| body.boxed()))
}

/// Checks the port and host, then connects to one of the checked addresses.
async fn connect(host: &str, port: u16, resolve: &Resolve) -> Result<TcpStream, StatusCode> {
    if !allowed_port(port) {
        warn!(port, "egress proxy refused a port");
        return Err(StatusCode::FORBIDDEN);
    }
    let addrs = match resolve(host.to_string(), port).await {
        Ok(addrs) if !addrs.is_empty() => addrs,
        Ok(_) => return Err(StatusCode::BAD_GATEWAY),
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            warn!(port, "egress proxy blocked a private address");
            return Err(StatusCode::FORBIDDEN);
        }
        Err(_) => return Err(StatusCode::BAD_GATEWAY),
    };
    match tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(&addrs[..])).await {
        Ok(Ok(stream)) => Ok(stream),
        Ok(Err(_)) => Err(StatusCode::BAD_GATEWAY),
        Err(_) => Err(StatusCode::GATEWAY_TIMEOUT),
    }
}

fn status(code: StatusCode) -> Response<Body> {
    let body = if code == StatusCode::OK {
        Empty::<Bytes>::new()
            .map_err(|never| match never {})
            .boxed()
    } else {
        Full::new(Bytes::from(code.canonical_reason().unwrap_or("error")))
            .map_err(|never| match never {})
            .boxed()
    };
    let mut resp = Response::new(body);
    *resp.status_mut() = code;
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn start(resolve: Resolve) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(serve(listener, resolve));
        addr
    }

    /// Resolves `upstream.test` to `target` and refuses everything else, like the public
    /// resolver refuses private addresses. Lets tests reach a loopback upstream.
    fn test_resolver(target: SocketAddr) -> Resolve {
        Arc::new(move |host, _port| {
            Box::pin(async move {
                if host == "upstream.test" {
                    Ok(vec![target])
                } else {
                    Err(io::Error::new(io::ErrorKind::PermissionDenied, "blocked"))
                }
            })
        })
    }

    /// Sends raw bytes and returns everything read until the proxy closes or `stop` appears.
    async fn exchange(proxy: SocketAddr, request: &str, stop: &str) -> String {
        let mut conn = TcpStream::connect(proxy).await.unwrap();
        conn.write_all(request.as_bytes()).await.unwrap();
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let read = tokio::time::timeout(Duration::from_secs(5), conn.read(&mut buf)).await;
            match read {
                Ok(Ok(0)) | Err(_) => break,
                Ok(Ok(n)) => out.extend_from_slice(&buf[..n]),
                Ok(Err(_)) => break,
            }
            if String::from_utf8_lossy(&out).contains(stop) {
                break;
            }
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    fn status_line(response: &str) -> &str {
        response.lines().next().unwrap_or_default()
    }

    #[tokio::test]
    async fn connect_to_private_addresses_is_refused() {
        let proxy = start(public_resolver()).await;
        for target in [
            "127.0.0.1:443",
            "localhost:443",
            "10.0.0.1:443",
            "192.168.1.1:443",
            "169.254.169.254:80",
            "100.64.0.1:443",
            "[::1]:443",
            "[::ffff:127.0.0.1]:443",
            "[fd00::1]:443",
            "db:5432",
            "api:8080",
            "metadata.google.internal:80",
        ] {
            let req = format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n\r\n");
            let resp = exchange(proxy, &req, "\r\n\r\n").await;
            assert!(status_line(&resp).contains(" 403 "), "{target}: {resp}");
        }
    }

    #[tokio::test]
    async fn plain_requests_to_private_addresses_are_refused() {
        let proxy = start(public_resolver()).await;
        for url in [
            "http://127.0.0.1/",
            "http://localhost:8080/",
            "http://10.89.11.2:9222/json/version",
            "http://[::1]/",
        ] {
            let req = format!("GET {url} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
            let resp = exchange(proxy, &req, "\r\n\r\n").await;
            assert!(status_line(&resp).contains(" 403 "), "{url}: {resp}");
        }
    }

    #[tokio::test]
    async fn low_ports_other_than_web_ones_are_refused() {
        let echo = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy = start(test_resolver(echo.local_addr().unwrap())).await;
        for port in [22, 25, 465, 587, 1023] {
            let req =
                format!("CONNECT upstream.test:{port} HTTP/1.1\r\nHost: upstream.test\r\n\r\n");
            let resp = exchange(proxy, &req, "\r\n\r\n").await;
            assert!(status_line(&resp).contains(" 403 "), "{port}: {resp}");
        }
    }

    #[tokio::test]
    async fn connect_tunnels_bytes_to_an_allowed_host() {
        let echo = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let echo_addr = echo.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut s, _) = echo.accept().await.unwrap();
            let mut buf = [0u8; 64];
            let n = s.read(&mut buf).await.unwrap();
            s.write_all(&buf[..n]).await.unwrap();
        });
        let proxy = start(test_resolver(echo_addr)).await;

        let mut conn = TcpStream::connect(proxy).await.unwrap();
        let req = format!(
            "CONNECT upstream.test:{0} HTTP/1.1\r\nHost: upstream.test:{0}\r\n\r\n",
            echo_addr.port()
        );
        conn.write_all(req.as_bytes()).await.unwrap();
        let mut head = vec![0u8; 256];
        let n = conn.read(&mut head).await.unwrap();
        let head = String::from_utf8_lossy(&head[..n]).into_owned();
        assert!(status_line(&head).contains(" 200 "), "{head}");

        conn.write_all(b"ping through the tunnel").await.unwrap();
        let mut back = vec![0u8; 64];
        let n = conn.read(&mut back).await.unwrap();
        assert_eq!(&back[..n], b"ping through the tunnel");
    }

    #[tokio::test]
    async fn plain_requests_are_sent_in_origin_form_without_proxy_headers() {
        let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin_addr = origin.local_addr().unwrap();
        let (seen_tx, seen_rx) = tokio::sync::oneshot::channel::<String>();
        tokio::spawn(async move {
            let (mut s, _) = origin.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let mut got = Vec::new();
            while !String::from_utf8_lossy(&got).contains("\r\n\r\n") {
                let n = s.read(&mut buf).await.unwrap();
                got.extend_from_slice(&buf[..n]);
            }
            seen_tx
                .send(String::from_utf8_lossy(&got).into_owned())
                .unwrap();
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello")
                .await
                .unwrap();
        });
        let proxy = start(test_resolver(origin_addr)).await;

        let req = format!(
            "GET http://upstream.test:{0}/page?q=1 HTTP/1.1\r\nHost: upstream.test:{0}\r\nProxy-Connection: keep-alive\r\nProxy-Authorization: Basic eDp5\r\n\r\n",
            origin_addr.port()
        );
        let resp = exchange(proxy, &req, "hello").await;
        assert!(status_line(&resp).contains(" 200 "), "{resp}");
        assert!(resp.ends_with("hello"), "{resp}");

        let seen = seen_rx.await.unwrap();
        assert!(seen.starts_with("GET /page?q=1 HTTP/1.1\r\n"), "{seen}");
        let lower = seen.to_ascii_lowercase();
        assert!(!lower.contains("proxy-connection"), "{seen}");
        assert!(!lower.contains("proxy-authorization"), "{seen}");
        assert!(
            lower.contains(&format!("host: upstream.test:{}", origin_addr.port())),
            "{seen}"
        );
    }

    #[tokio::test]
    async fn https_without_connect_is_refused() {
        let proxy = start(public_resolver()).await;
        let req =
            "GET https://example.com/ HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n";
        let resp = exchange(proxy, req, "\r\n\r\n").await;
        assert!(status_line(&resp).contains(" 400 "), "{resp}");
    }
}
