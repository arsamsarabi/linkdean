//! Keeps server-side fetches away from private networks. Bookmark URLs and everything a
//! fetched page points at are untrusted, and the responses are stored and shown to users.

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::redirect;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use url::{Host, Url};

const MAX_REDIRECTS: usize = 8;

/// An HTTP client that can only reach public addresses.
///
/// Domains are filtered in the resolver, so the address that gets checked is the one
/// that gets connected to, which rules out DNS rebinding. IP literals skip the resolver,
/// so the first URL goes through [`allowed_literal`] and redirects are checked here.
pub fn public_client(timeout: Duration, user_agent: &str) -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(timeout)
        .user_agent(user_agent)
        .no_proxy()
        .dns_resolver(Arc::new(PublicResolver))
        .redirect(redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= MAX_REDIRECTS {
                attempt.stop()
            } else if allowed_literal(attempt.url()) {
                attempt.follow()
            } else {
                attempt.error("redirect to a blocked address")
            }
        }))
        .build()
}

struct PublicResolver;

impl Resolve for PublicResolver {
    fn resolve(&self, name: Name) -> Resolving {
        Box::pin(async move {
            let addrs = public_addrs(name.as_str()).await?;
            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

/// True when the scheme is http(s) and the host is not a blocked name or address.
/// Does not resolve DNS: use [`resolves_public`] or [`public_client`] for that.
pub fn allowed_literal(url: &Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    match url.host() {
        Some(Host::Domain(domain)) => !is_blocked_name(domain),
        Some(Host::Ipv4(ip)) => !is_blocked_ip(IpAddr::V4(ip)),
        Some(Host::Ipv6(ip)) => !is_blocked_ip(IpAddr::V6(ip)),
        None => false,
    }
}

/// True when the URL passes [`allowed_literal`] and every address of its host is public.
pub async fn resolves_public(url: &Url) -> bool {
    if !allowed_literal(url) {
        return false;
    }
    match url.host() {
        Some(Host::Domain(domain)) => public_addrs(domain).await.is_ok(),
        _ => true,
    }
}

/// The addresses to connect to for `host:port`, or `PermissionDenied` when the host is a
/// blocked name or any of its addresses is private. `host` may be a name or an IP literal,
/// with or without IPv6 brackets. Connect to exactly these, so the check cannot be raced.
pub async fn public_socket_addrs(host: &str, port: u16) -> std::io::Result<Vec<SocketAddr>> {
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = bare.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            return Err(blocked());
        }
        return Ok(vec![SocketAddr::new(ip, port)]);
    }
    let mut addrs = public_addrs(bare).await?;
    for addr in &mut addrs {
        addr.set_port(port);
    }
    Ok(addrs)
}

/// All-or-nothing, so a name with one public and one private address is refused.
async fn public_addrs(host: &str) -> Result<Vec<SocketAddr>, std::io::Error> {
    if is_blocked_name(host) {
        return Err(blocked());
    }
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host, 0)).await?.collect();
    if addrs.is_empty() || addrs.iter().any(|a| is_blocked_ip(a.ip())) {
        return Err(blocked());
    }
    Ok(addrs)
}

fn blocked() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::PermissionDenied, "blocked address")
}

pub fn is_blocked_name(host: &str) -> bool {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    // A name with no dot only resolves inside a private network, such as a compose service.
    !host.contains('.')
        || matches!(
            host.as_str(),
            "localhost.localdomain" | "metadata.google.internal"
        )
        || [".localhost", ".local", ".internal", ".lan", ".home.arpa"]
            .iter()
            .any(|suffix| host.ends_with(suffix))
}

pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_blocked_v4(ip),
        IpAddr::V6(ip) => is_blocked_v6(ip),
    }
}

fn is_blocked_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_documentation()
        || a == 0
        || a >= 240
        // Carrier-grade NAT, which Tailscale also uses.
        || (a == 100 && (64..128).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 198 && (18..20).contains(&b))
}

fn is_blocked_v6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = embedded_v4(ip) {
        return is_blocked_v4(v4);
    }
    let first = ip.segments()[0];
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        // Unique local, link-local and the old site-local range.
        || first & 0xfe00 == 0xfc00
        || first & 0xffc0 == 0xfe80
        || first & 0xffc0 == 0xfec0
        // Documentation.
        || (first == 0x2001 && ip.segments()[1] == 0x0db8)
}

/// The IPv4 address carried by mapped, compatible, NAT64 and 6to4 forms, which reach
/// the same host as the plain address.
fn embedded_v4(ip: Ipv6Addr) -> Option<Ipv4Addr> {
    let o = ip.octets();
    let s = ip.segments();
    if let Some(v4) = ip.to_ipv4_mapped() {
        return Some(v4);
    }
    if s[..6] == [0, 0, 0, 0, 0, 0] && !ip.is_loopback() && !ip.is_unspecified() {
        return Some(Ipv4Addr::new(o[12], o[13], o[14], o[15]));
    }
    if s[..6] == [0x64, 0xff9b, 0, 0, 0, 0] {
        return Some(Ipv4Addr::new(o[12], o[13], o[14], o[15]));
    }
    if s[0] == 0x2002 {
        return Some(Ipv4Addr::new(o[2], o[3], o[4], o[5]));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(url: &str) -> bool {
        allowed_literal(&Url::parse(url).unwrap())
    }

    #[test]
    fn allows_public_web_addresses() {
        assert!(allowed("https://example.com/path?q=1"));
        assert!(allowed("http://93.184.216.34/"));
        assert!(allowed("http://[2606:2800:220:1:248:1893:25c8:1946]/"));
    }

    #[test]
    fn refuses_other_schemes() {
        for url in [
            "file:///etc/passwd",
            "ftp://example.com/",
            "data:text/html,hi",
            "chrome://version",
            "javascript:alert(1)",
        ] {
            assert!(!allowed(url), "{url}");
        }
    }

    #[test]
    fn refuses_private_addresses_in_every_spelling() {
        for url in [
            "http://127.0.0.1/",
            "http://2130706433/",
            "http://0x7f.1/",
            "http://0.0.0.0/",
            "http://10.0.0.1/",
            "http://172.16.0.1/",
            "http://192.168.1.1/",
            "http://169.254.169.254/latest/meta-data/",
            "http://100.64.0.1/",
            "http://[::1]/",
            "http://[::ffff:127.0.0.1]/",
            "http://[::ffff:169.254.169.254]/",
            "http://[::127.0.0.1]/",
            "http://[64:ff9b::7f00:1]/",
            "http://[2002:7f00:1::]/",
            "http://[fd00::1]/",
            "http://[fe80::1]/",
        ] {
            assert!(!allowed(url), "{url}");
        }
    }

    #[test]
    fn refuses_internal_names() {
        for url in [
            "http://localhost:8080/",
            "http://foo.localhost/",
            "http://db:5432/",
            "http://meilisearch:7700/",
            "http://nas.local/",
            "http://metadata.google.internal/",
            "http://LOCALHOST./",
        ] {
            assert!(!allowed(url), "{url}");
        }
    }

    #[tokio::test]
    async fn client_refuses_a_loopback_name() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let internal = listener.local_addr().unwrap();
        let client = public_client(Duration::from_secs(2), "test").unwrap();
        let err = client
            .get(format!("http://localhost.:{}/", internal.port()))
            .send()
            .await
            .unwrap_err();
        assert!(err.is_connect() || err.is_request(), "{err}");
    }
}
