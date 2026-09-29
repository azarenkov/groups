pub mod github_connector;
pub mod google_verifier;
pub mod steam_connector;

pub(crate) fn urlencode(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
pub mod jwt_token_issuer;
pub mod redis_access_revoker;
pub mod altcha;
pub mod redis_captcha_store;
