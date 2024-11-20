use std::collections::HashMap;
use std::sync::Mutex;
use time::{OffsetDateTime, Duration};
use tracing::{warn, error};
use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug)]
struct RateLimit {
    attempts: u32,
    first_attempt: OffsetDateTime,
    last_attempt: OffsetDateTime,
}

#[derive(Debug)]
pub struct RateLimiter {
    limits: Mutex<HashMap<String, RateLimit>>,
    max_attempts: u32,
    window: Duration,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            limits: Mutex::new(HashMap::new()),
            max_attempts: 5,
            window: Duration::minutes(15),
        }
    }
}

impl RateLimiter {
    pub fn new(max_attempts: u32, window_minutes: i64) -> Self {
        Self {
            limits: Mutex::new(HashMap::new()),
            max_attempts,
            window: Duration::minutes(window_minutes),
        }
    }

    pub fn check_rate_limit(&self, key: &str) -> Result<(), ErrorResponse> {
        let now = OffsetDateTime::now_utc();
        
        let result = {
            let mut limits = match self.limits.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    error!("Failed to acquire rate limit lock: {}", e);
                    return Err(ErrorResponse { error: "Internal rate limit error".into() });
                }
            };
            
            limits.retain(|_, limit| now - limit.first_attempt <= self.window * 2);
    
            match limits.get_mut(key) {
                Some(limit) => {
                    if now - limit.first_attempt <= self.window && limit.attempts >= self.max_attempts {
                        let minutes_to_wait = (limit.first_attempt + self.window - now).whole_minutes();
                        Err(ErrorResponse {
                            error: format!("Rate limit exceeded. Please try again in {} minutes.", minutes_to_wait.max(1)),
                        })
                    } else if now - limit.first_attempt > self.window {
                        *limit = RateLimit {
                            attempts: 1,
                            first_attempt: now,
                            last_attempt: now,
                        };
                        Ok(())
                    } else {
                        limit.attempts += 1;
                        limit.last_attempt = now;
                        Ok(())
                    }
                }
                None => {
                    limits.insert(key.to_string(), RateLimit {
                        attempts: 1,
                        first_attempt: now,
                        last_attempt: now,
                    });
                    Ok(())
                }
            }
        };
    
        if let Err(ref e) = result {
            warn!("Rate limit triggered for key {}: {}", key, e.error);
        }
    
        result
    }    

    pub fn get_remaining_attempts(&self, key: &str) -> Option<(u32, i64)> {
        let now = OffsetDateTime::now_utc();
        
        if let Ok(limits) = self.limits.lock() {
            if let Some(limit) = limits.get(key) {
                if now - limit.first_attempt <= self.window {
                    let remaining_attempts = self.max_attempts.saturating_sub(limit.attempts);
                    let minutes_remaining = (limit.first_attempt + self.window - now).whole_minutes();
                    Some((remaining_attempts, minutes_remaining))
                } else {
                    Some((self.max_attempts, 0))
                }
            } else {
                Some((self.max_attempts, 0))
            }
        } else {
            None
        }
    }
}