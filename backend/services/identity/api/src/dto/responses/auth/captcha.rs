use identity::ports::captcha::CaptchaChallenge;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptchaChallengeResponse {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenge: Option<AltchaChallengeResponse>,
}

#[derive(Debug, Serialize)]
pub struct AltchaChallengeResponse {
    pub algorithm: String,
    pub challenge: String,
    pub maxnumber: u64,
    pub salt: String,
    pub signature: String,
}

impl From<Option<CaptchaChallenge>> for CaptchaChallengeResponse {
    fn from(value: Option<CaptchaChallenge>) -> Self {
        Self {
            enabled: value.is_some(),
            challenge: value.map(|c| AltchaChallengeResponse {
                algorithm: c.algorithm,
                challenge: c.challenge,
                maxnumber: c.max_number,
                salt: c.salt,
                signature: c.signature,
            }),
        }
    }
}
