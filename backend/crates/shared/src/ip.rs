use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use actix_web::{HttpRequest, dev::ServiceRequest, web};

#[derive(Debug, Clone, Default)]
pub struct TrustedProxyIps(pub Vec<IpAddr>);

impl TrustedProxyIps {
    pub fn contains(&self, ip: IpAddr) -> bool {
        self.0.contains(&ip)
    }
}

pub async fn resolve_trusted_proxy(hostname: Option<&str>) -> TrustedProxyIps {
    let Some(host) = hostname.filter(|h| !h.is_empty()) else {
        return TrustedProxyIps::default();
    };
    let ips: Vec<IpAddr> = tokio::net::lookup_host(format!("{host}:0"))
        .await
        .map(|iter| iter.map(|sa| sa.ip()).collect())
        .unwrap_or_default();
    TrustedProxyIps(ips)
}

pub fn real_ip_from_request(req: &HttpRequest) -> Option<IpAddr> {
    let trusted = req.app_data::<web::Data<TrustedProxyIps>>();
    let peer = req.peer_addr().map(|s| s.ip());
    let info = req.connection_info();
    let candidate = match (peer, trusted) {
        (Some(p), Some(t)) if t.get_ref().contains(p) => info.realip_remote_addr(),
        _ => info.peer_addr(),
    };
    candidate.and_then(parse_socket_or_ip)
}

pub fn real_ip_from_service_request(req: &ServiceRequest) -> Option<IpAddr> {
    let trusted = req.app_data::<web::Data<TrustedProxyIps>>();
    let peer = req.peer_addr().map(|s| s.ip());
    let info = req.connection_info();
    let candidate = match (peer, trusted) {
        (Some(p), Some(t)) if t.get_ref().contains(p) => info.realip_remote_addr(),
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
