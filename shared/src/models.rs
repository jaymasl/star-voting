use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use time::OffsetDateTime;
use uuid::Uuid;

#[cfg(feature = "backend")]
#[derive(sqlx::Type)]
#[sqlx(type_name = "vote_state", rename_all = "lowercase")]
pub enum VoteState {
    Active,
    Concluded,
    PendingDeletion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Vote {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub options: Vec<String>,
    pub voting_ends_at: OffsetDateTime,
    pub ballots: Vec<VoteBallot>,
    pub duration_hours: i32,
    pub duration_minutes: i32,
    pub user_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VoteBallot {
    pub csrf_token: String,
    pub captcha_token: String,
    pub scores: HashMap<String, i8>,
    pub user_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BallotResponse {
    pub ballot_id: i64,
    pub vote_id: Uuid,
    pub cast_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteStats {
    pub total_ballots: usize,
    pub option_scores: HashMap<String, VoteOptionStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteOptionStats {
    pub total_score: i32,
    pub average_score: f64,
    pub frequency: HashMap<i8, usize>,
    pub total_votes: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateVoteRequest {
    pub csrf_token: String,
    pub captcha_token: String,
    pub title: String,
    pub description: String,
    pub options: Vec<String>,
    pub duration_hours: i32,
    pub duration_minutes: i32,
    pub user_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadToHeadResult {
    pub finalist1: String,
    pub finalist2: String,
    pub finalist1_votes: u32,
    pub finalist2_votes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteResult {
    pub winner: Option<String>,
    pub error: Option<String>,
    pub stats: VoteStats,
    pub head_to_head: Option<HeadToHeadResult>,
    pub duration_hours: Option<i64>,
    pub duration_minutes: Option<i64>,
}

impl Vote {
    pub fn is_ended(&self) -> bool {
        OffsetDateTime::now_utc() > self.voting_ends_at
    }

    pub fn end_now(&mut self) {
        self.voting_ends_at = OffsetDateTime::now_utc();
    }

    pub fn total_votes(&self) -> usize {
        self.ballots.len()
    }
}