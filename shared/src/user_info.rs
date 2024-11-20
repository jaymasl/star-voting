use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub user_fingerprint: String,
    pub ip: String,
}

// Frontend-specific code
#[cfg(target_arch = "wasm32")]
pub fn generate_browser_fingerprint() -> String {
    use web_sys::window;
    use base64::engine::general_purpose::URL_SAFE;
    use base64::Engine;
    use sha2::{Sha256, Digest};

    let window = window().unwrap();
    let navigator = window.navigator();
    let user_agent = navigator.user_agent().unwrap_or_default();
    let platform = navigator.platform().unwrap_or_default();
    let language = navigator.language().unwrap_or_default();

    let fingerprint = format!("{}-{}-{}", user_agent, platform, language);

    let mut hasher = Sha256::new();
    hasher.update(fingerprint.as_bytes());
    URL_SAFE.encode(hasher.finalize())
}

// Backend-specific code
#[cfg(not(target_arch = "wasm32"))]
pub fn generate_server_fingerprint(ip: &str, user_agent: Option<&str>) -> String {
    use base64::engine::general_purpose::URL_SAFE;
    use base64::Engine;
    use sha2::{Sha256, Digest};

    let mut hasher = Sha256::new();
    hasher.update(ip.as_bytes());
    if let Some(ua) = user_agent {
        hasher.update(ua.as_bytes());
    }
    URL_SAFE.encode(hasher.finalize())
}

// Backend-specific Rocket implementation
#[cfg(feature = "backend")]
mod backend_impl {
    use super::*;
    use rocket::request::{FromRequest, Outcome};
    use rocket::Request;

    #[rocket::async_trait]
    impl<'r> FromRequest<'r> for UserInfo {
        type Error = ();

        async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
            let headers = req.headers();
            let ip = headers.get_one("X-Real-IP")
                .or_else(|| headers.get_one("X-Forwarded-For"))
                .unwrap_or("0.0.0.0")
                .to_string();

            let user_agent = headers.get_one("User-Agent");
            let fingerprint = super::generate_server_fingerprint(&ip, user_agent);

            Outcome::Success(UserInfo {
                id: Uuid::new_v4(),
                user_fingerprint: fingerprint,
                ip,
            })
        }
    }
}