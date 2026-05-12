use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
    time::Duration,
};

use actix_web::{HttpRequest, dev::ServiceRequest, http::header::HeaderMap, web};

const RESOLVE_ATTEMPTS: u32 = 5;
const RESOLVE_RETRY_DELAY: Duration = Duration::from_secs(1);
const X_FORWARDED_FOR: &str = "X-Forwarded-For";

#[derive(Debug, Clone, Default)]
pub struct TrustedProxyIps(pub Vec<IpAddr>);

impl TrustedProxyIps {
    pub fn contains(&self, ip: IpAddr) -> bool {
        self.0.contains(&ip)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveTrustedProxyError {
    #[error("DNS lookup for trusted proxy {host} failed after {attempts} attempts: {source}")]
    LookupFailed {
        host: String,
        attempts: u32,
        #[source]
        source: std::io::Error,
    },
    #[error("DNS lookup for trusted proxy {host} returned no addresses after {attempts} attempts")]
    NoAddresses { host: String, attempts: u32 },
}

pub async fn resolve_trusted_proxy(
    hostname: Option<&str>,
) -> Result<TrustedProxyIps, ResolveTrustedProxyError> {
    let Some(host) = hostname.filter(|h| !h.is_empty()) else {
        return Ok(TrustedProxyIps::default());
    };

    let mut last_io_err: Option<std::io::Error> = None;
    for attempt in 1..=RESOLVE_ATTEMPTS {
        match tokio::net::lookup_host(format!("{host}:0")).await {
            Ok(iter) => {
                let ips: Vec<IpAddr> = iter.map(|sa| sa.ip()).collect();
                if !ips.is_empty() {
                    return Ok(TrustedProxyIps(ips));
                }
                tracing::warn!(host, attempt, "Trusted proxy DNS returned no addresses");
            }
            Err(e) => {
                tracing::warn!(host, attempt, error = %e, "Trusted proxy DNS lookup failed");
                last_io_err = Some(e);
            }
        }
        if attempt < RESOLVE_ATTEMPTS {
            tokio::time::sleep(RESOLVE_RETRY_DELAY).await;
        }
    }

    Err(match last_io_err {
        Some(source) => ResolveTrustedProxyError::LookupFailed {
            host: host.to_string(),
            attempts: RESOLVE_ATTEMPTS,
            source,
        },
        None => ResolveTrustedProxyError::NoAddresses {
            host: host.to_string(),
            attempts: RESOLVE_ATTEMPTS,
        },
    })
}

pub fn real_ip_from_request(req: &HttpRequest) -> Option<IpAddr> {
    let trusted = req.app_data::<web::Data<TrustedProxyIps>>().map(|d| d.get_ref());
    let peer = req.peer_addr().map(|s| s.ip());
    resolve_real_ip(peer, trusted, req.headers())
}

pub fn real_ip_from_service_request(req: &ServiceRequest) -> Option<IpAddr> {
    let trusted = req.app_data::<web::Data<TrustedProxyIps>>().map(|d| d.get_ref());
    let peer = req.peer_addr().map(|s| s.ip());
    resolve_real_ip(peer, trusted, req.headers())
}

fn resolve_real_ip(
    peer: Option<IpAddr>,
    trusted: Option<&TrustedProxyIps>,
    headers: &HeaderMap,
) -> Option<IpAddr> {
    if let (Some(p), Some(t)) = (peer, trusted)
        && t.contains(p)
        && let Some(xff) = headers.get(X_FORWARDED_FOR).and_then(|v| v.to_str().ok())
        && let Some(client) = rightmost_untrusted(xff, t)
    {
        return Some(client);
    }

    peer
}

fn rightmost_untrusted(header_value: &str, trusted: &TrustedProxyIps) -> Option<IpAddr> {
    for raw in header_value.rsplit(',') {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let ip = parse_socket_or_ip(trimmed)?;
        if !trusted.contains(ip) {
            return Some(ip);
        }
    }
    None
}

fn parse_socket_or_ip(s: &str) -> Option<IpAddr> {
    SocketAddr::from_str(s)
        .map(|sa| sa.ip())
        .or_else(|_| IpAddr::from_str(s))
        .ok()
}
