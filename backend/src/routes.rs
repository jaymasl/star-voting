use rocket::{State, get, post, http::Status, serde::json::Json};
use tracing::{error, debug, instrument};
use std::sync::Mutex;
use std::collections::HashSet;
use ring::rand::{SecureRandom, SystemRandom};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rustrict::CensorStr;
use sqlx::PgPool;
use shared::{models::*, user_info::UserInfo};
use crate::{
   processor::{VoteProcessor, ValidationError},
   utils::parse_vote_id,
   rate_limiter::{RateLimiter, ErrorResponse},
   captcha::CaptchaVerifier
};

const CREATE_VOTE_WINDOW_MINUTES: i64 = 60;
const CAST_BALLOT_WINDOW_MINUTES: i64 = 1;
const MAX_TOKENS: usize = 10000;

pub struct CsrfGuard {
    tokens: Mutex<HashSet<String>>,
    rng: SystemRandom,
}

impl CsrfGuard {
    fn new() -> Self {
        Self {
            tokens: Mutex::new(HashSet::new()),
            rng: SystemRandom::new(),
        }
    }

    fn cleanup_old_tokens(&self) {
        if let Ok(mut tokens) = self.tokens.lock() {
            if tokens.len() > MAX_TOKENS {
                tokens.clear();
            }
        }
    }

    fn generate_token(&self) -> Result<String, Status> {
        self.cleanup_old_tokens();
        let mut bytes = [0u8; 32];
        self.rng.fill(&mut bytes).map_err(|_| Status::InternalServerError)?;
        let token = URL_SAFE_NO_PAD.encode(bytes);
        if let Ok(mut tokens) = self.tokens.lock() {
            tokens.insert(token.clone());
            debug!("Generated new CSRF token");
            Ok(token)
        } else {
            error!("Failed to acquire lock for token storage");
            Err(Status::InternalServerError)
        }
    }

    fn verify_token(&self, token: &str) -> Result<(), Status> {
        let mut tokens = self.tokens.lock().map_err(|_| Status::InternalServerError)?;
        if !tokens.remove(token) {
            debug!("CSRF token validation failed. Token not found or already used.");
            return Err(Status::Forbidden);
        }
        debug!("CSRF token validated successfully");
        Ok(())
    }
}

pub struct AppState {
    pub vote_limiter: RateLimiter,
    pub ballot_limiter: RateLimiter,
    pub csrf: CsrfGuard,
    pub captcha: CaptchaVerifier,
    pub db: PgPool,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        Self {
            vote_limiter: RateLimiter::new(1, CREATE_VOTE_WINDOW_MINUTES),
            ballot_limiter: RateLimiter::new(1, CAST_BALLOT_WINDOW_MINUTES),
            csrf: CsrfGuard::new(),
            captcha: CaptchaVerifier::new(),
            db: pool,
        }
    }

    pub fn new_with_captcha(pool: PgPool, captcha_secret: impl Into<String>) -> Self {
        Self {
            vote_limiter: RateLimiter::new(1, CREATE_VOTE_WINDOW_MINUTES),
            ballot_limiter: RateLimiter::new(1, CAST_BALLOT_WINDOW_MINUTES),
            csrf: CsrfGuard::new(),
            captcha: CaptchaVerifier::new_with_secret(captcha_secret),
            db: pool,
        }
    }
}

fn check_combined_options_for_profanity(options: &[String]) -> Result<(), String> {
    for option in options {
        if option.is_inappropriate() {
            return Err(format!("Possible profanity detected in option: {}", option));
        }
    }

    for window_size in 2..=options.len() {
        for window in options.windows(window_size) {
            let combined = window.join("");
            if combined.is_inappropriate() {
                return Err(format!(
                    "Inappropriate content detected across options: {}",
                    window.join(", ")
                ));
            }
        }
    }

    Ok(())
}

#[get("/csrf-token")]
pub async fn get_csrf_token(state: &State<AppState>) -> Result<String, Status> {
    state.csrf.generate_token()
}

#[get("/votes")]
pub async fn list_votes(state: &State<AppState>) -> Result<Json<Vec<Vote>>, Status> {
    VoteProcessor::fetch_all_votes(&state.db)
        .await
        .map(Json)
        .map_err(|_| Status::InternalServerError)
}

#[rocket::options("/<_..>")]
pub async fn all_options() -> Status {
    Status::Ok
}

#[instrument(skip(state, request), fields(vote_id))]
#[post("/vote", format = "json", data = "<request>")]
pub async fn create_vote(
    state: &State<AppState>,
    request: Json<CreateVoteRequest>,
    user_info: UserInfo,
) -> Result<Json<Vote>, (Status, Json<ErrorResponse>)> {
    let mut request_data = request.into_inner();
    
    debug!("Validating CSRF token for vote creation: length={}", request_data.csrf_token.len());
    match state.csrf.verify_token(&request_data.csrf_token) {
        Ok(_) => (),
        Err(_) => {
            match state.csrf.generate_token() {
                Ok(new_token) => {
                    return Err((Status::Forbidden, Json(ErrorResponse {
                        error: format!("CSRF token expired, please use new token: {}", new_token)
                    })));
                }
                Err(status) => {
                    return Err((status, Json(ErrorResponse {
                        error: "Failed to generate new CSRF token".into()
                    })));
                }
            }
        }
    }

    if request_data.title.is_inappropriate() {
        return Err((Status::BadRequest, Json(ErrorResponse {
            error: format!("Possible profanity detected in title: {}", request_data.title)
        })));
    }

    if request_data.description.is_inappropriate() {
        return Err((Status::BadRequest, Json(ErrorResponse {
            error: format!("Possible profanity detected in description: {}", request_data.description)
        })));
    }

    if let Err(error) = check_combined_options_for_profanity(&request_data.options) {
        return Err((Status::BadRequest, Json(ErrorResponse { error })));
    }

    if !state.captcha.verify(&request_data.captcha_token, Some(&user_info.ip)).await {
        return Err((Status::BadRequest, Json(ErrorResponse {
            error: "Invalid captcha".into()
        })));
    }

    request_data.user_fingerprint = user_info.user_fingerprint.clone();
    
    let vote = match VoteProcessor::create_vote(&request_data) {
        Ok(v) => v,
        Err(e) => return Err((Status::BadRequest, Json(ErrorResponse { error: e.to_string() })))
    };

    let rate_limit_key = format!("create_vote:{}", user_info.user_fingerprint);
    if let Err(e) = state.vote_limiter.check_rate_limit(&rate_limit_key) {
        return Err((Status::TooManyRequests, Json(e)));
    }

    match VoteProcessor::create_vote_db(&state.db, &vote).await {
        Ok(_) => Ok(Json(vote)),
        Err(e) => match e {
            ValidationError::ActiveVoteLimitExceeded(limit) =>
                Err((Status::BadRequest, Json(ErrorResponse {
                    error: format!("Maximum active vote limit ({}) exceeded", limit)
                }))),
            _ => Err((Status::InternalServerError, Json(ErrorResponse {
                error: "Failed to create vote".into()
            })))
        }
    }
}

#[instrument(skip(state, ballot), fields(vote_id = %id))]
#[post("/vote/<id>/ballot", format = "json", data = "<ballot>")]
pub async fn cast_ballot(
    state: &State<AppState>,
    id: &str,
    ballot: Json<VoteBallot>,
    user_info: UserInfo
) -> Result<Json<BallotResponse>, (Status, Json<ErrorResponse>)> {
    let ballot_data = ballot.into_inner();
    let uuid = parse_vote_id(id).map_err(|_| (
        Status::BadRequest, 
        Json(ErrorResponse { error: "Invalid vote ID".into() })
    ))?;
    
    debug!("Validating CSRF token for ballot: length={}", ballot_data.csrf_token.len());
    match state.csrf.verify_token(&ballot_data.csrf_token) {
        Ok(_) => (),
        Err(_) => {
            match state.csrf.generate_token() {
                Ok(new_token) => {
                    return Err((Status::Forbidden, Json(ErrorResponse {
                        error: format!("CSRF token expired, please use new token: {}", new_token)
                    })));
                }
                Err(status) => {
                    return Err((status, Json(ErrorResponse {
                        error: "Failed to generate new CSRF token".into()
                    })));
                }
            }
        }
    }

    if !state.captcha.verify(&ballot_data.captcha_token, Some(&user_info.ip)).await {
        return Err((
            Status::BadRequest,
            Json(ErrorResponse { error: "Invalid captcha".into() })
        ));
    }

    let rate_limit_key = format!("cast_ballot:{}:{}", user_info.user_fingerprint, id);
    if let Err(e) = state.ballot_limiter.check_rate_limit(&rate_limit_key) {
        return Err((Status::TooManyRequests, Json(e)));
    }

    let scores: Vec<_> = ballot_data.scores.values().map(|&s| s as i32).collect();

    let result = sqlx::query!(
        "INSERT INTO active_votes.ballots (vote_id, user_fingerprint, scores) 
         VALUES ($1, $2, $3) 
         RETURNING id as ballot_id, cast_at",
        uuid,
        user_info.user_fingerprint,
        &scores
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        let (status, msg) = match e.to_string().contains("unique_voter") {
            true => (Status::Forbidden, "Already voted"),
            false => (Status::InternalServerError, "Database error"),
        };
        (status, Json(ErrorResponse { error: msg.into() }))
    })?;

    Ok(Json(BallotResponse {
        ballot_id: result.ballot_id,
        vote_id: uuid,
        cast_at: result.cast_at,
    }))
}

#[get("/vote/<id>/result")]
pub async fn get_result(state: &State<AppState>, id: &str) -> Result<Json<VoteResult>, Status> {
    let uuid = parse_vote_id(id).map_err(|_| Status::BadRequest)?;
    
    if let Some(vote) = VoteProcessor::get_vote_db(&state.db, uuid).await.map_err(|_| Status::InternalServerError)? {
        VoteProcessor::get_results(&vote)
            .map(Json)
            .map_err(|_| Status::InternalServerError)
    } else {
        Err(Status::NotFound)
    }
}

#[get("/vote/<id>")]
pub async fn get_vote(state: &State<AppState>, id: &str) -> Result<Json<Option<Vote>>, Status> {
    let uuid = parse_vote_id(id).map_err(|_| Status::BadRequest)?;
    VoteProcessor::fetch_vote_by_id(&state.db, uuid)
        .await
        .map(Json)
        .map_err(|_| Status::InternalServerError)
}