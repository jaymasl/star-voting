use crate::error::ApiError;
use shared::models::VoteStats;
use uuid::Uuid;
use time::OffsetDateTime;
use std::collections::HashMap;

pub fn parse_vote_id(id: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(id).map_err(|_| ApiError::InvalidId)
}

pub fn empty_stats() -> VoteStats {
    VoteStats {
        option_scores: HashMap::new(),
        total_ballots: 0,
    }
}

pub fn validate_vote_time(end_time: OffsetDateTime) -> Result<(), ApiError> {
    if OffsetDateTime::now_utc() > end_time {
        Err(ApiError::VoteEnded)
    } else {
        Ok(())
    }
}