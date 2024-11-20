use hcaptcha::{HcaptchaClient, HcaptchaRequest, HcaptchaCaptcha};
use tracing::warn;

pub struct CaptchaVerifier {
    secret: Option<String>,
}

impl CaptchaVerifier {
    pub fn new() -> Self {
        Self { secret: None }
    }

    pub fn new_with_secret(secret: impl Into<String>) -> Self {
        let secret = secret.into();
        if secret.trim().is_empty() {
            warn!("CaptchaVerifier created with empty secret - verification will be disabled");
            Self { secret: None }
        } else {
            Self { secret: Some(secret) }
        }
    }

    pub async fn verify(&self, token: &str, remote_ip: Option<&str>) -> bool {
        let Some(secret) = &self.secret else {
            warn!("Captcha verification skipped - HCAPTCHA_SECRET not configured");
            return true;
        };

        if token.trim().is_empty() {
            warn!("Empty captcha token received");
            return false;
        }

        let client = HcaptchaClient::new();

        let captcha = match HcaptchaCaptcha::new(token) {
            Ok(mut captcha) => {
                if let Some(ip) = remote_ip {
                    match captcha.set_remoteip(ip) {
                        Ok(c) => c,
                        Err(e) => {
                            warn!("Failed to set remote IP: {}", e);
                            captcha
                        }
                    }
                } else {
                    captcha
                }
            }
            Err(e) => {
                warn!("Failed to create hCaptcha captcha: {}", e);
                return false;
            }
        };

        let request = match HcaptchaRequest::new(secret, captcha) {
            Ok(req) => req,
            Err(e) => {
                warn!("Failed to create hCaptcha request: {}", e);
                return false;
            }
        };

        match client.verify_client_response(request).await {
            Ok(response) => {
                if !response.success() {
                    warn!("hCaptcha verification failed: {:?}", response.error_codes());
                }
                response.success()
            }
            Err(e) => {
                warn!("hCaptcha verification error: {}", e);
                false
            }
        }
    }
}