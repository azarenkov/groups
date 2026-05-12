use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use actix_web::{HttpRequest, dev::ServiceRequest, web};

#[derive(Debug, Clone, Copy)]
pub struct TrustedProxyIp(pub Option<IpAddr>);

pub async fn resolve_trusted_proxy(hostname: Option<&str>) -> TrustedProxyIp {
    let Some(host) = hostname.filter(|h| !h.is_empty()) else {
        return TrustedProxyIp(None);
    };
    let resolved = tokio::net::lookup_host(format!("{host}:0"))
        .await
        .ok()
        .and_then(|mut iter| iter.next())
        .map(|sa| sa.ip());
    TrustedProxyIp(resolved)
}

pub fn real_ip_from_request(req: &HttpRequest) -> Option<IpAddr> {
    let trusted = req
        .app_data::<web::Data<TrustedProxyIp>>()
        .and_then(|t| t.get_ref().0);
    let peer = req.peer_addr().map(|s| s.ip());
    let info = req.connection_info();
    match (peer, trusted) {
        (Some(p), Some(t)) if p == t => info.realip_remote_addr().and_then(parse_socket_or_ip),
        _ => info.peer_addr().and_then(parse_socket_or_ip),
    }
}

pub fn real_ip_from_service_request(req: &ServiceRequest) -> Option<IpAddr> {
    let trusted = req
        .app_data::<web::Data<TrustedProxyIp>>()
        .and_then(|t| t.get_ref().0);
    let peer = req.peer_addr().map(|s| s.ip());
    let info = req.connection_info();
    let candidate = match (peer, trusted) {
        (Some(p), Some(t)) if p == t => info.realip_remote_addr(),
        _ => info.peer_addr(),
    };
    candidate.and_then(parse_socket_or_ip)
}

fn parse_socket_or_ip(s: &str) -> Option<IpAddr> {
    SocketAddr::from_str(s)
        .map(|sa| sa.ip())
        .or_else(|_| IpAddr::from_str(s))
        .ok()
}
