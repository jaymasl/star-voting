use sqlx::{PgPool, postgres::PgQueryResult};
use uuid::Uuid;
use time::OffsetDateTime;
use crate::error::ApiError;
use std::collections::HashMap;
use shared::models::*;

pub struct Queries;

impl Queries {
    pub async fn get_active_vote(pool: &PgPool, id: Uuid) -> Result<Option<Vote>, ApiError> {
        let record = sqlx::query!(
            "SELECT id, title, description, options, end_time, duration_hours, duration_minutes 
             FROM active_votes.votes WHERE id = $1 AND state = 'active'",
            id
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        let Some(vote) = record else { return Ok(None) };

        let ballots = sqlx::query!(
            "SELECT scores, voter_fingerprint FROM active_votes.ballots WHERE vote_id = $1",
            id
        )
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        let vote_ballots = ballots.into_iter().map(|b| VoteBallot {
            scores: vote.options.iter().enumerate()
                .map(|(i, opt)| (opt.clone(), b.scores[i] as i8))
                .collect()
        }).collect();

        Ok(Some(Vote {
            id: vote.id,
            title: vote.title,
            description: vote.description,
            options: vote.options,
            end_time: vote.end_time,
            ballots: vote_ballots,
            duration_hours: vote.duration_hours,
            duration_minutes: vote.duration_minutes,
        }))
    }

    pub async fn get_archived_vote(pool: &PgPool, id: Uuid) -> Result<Option<Vote>, ApiError> {
        let record = sqlx::query!(
            "SELECT id, title, description, options, end_time, final_stats, duration_hours, duration_minutes
             FROM archived_votes.votes WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        let Some(vote) = record else { return Ok(None) };

        let ballots = sqlx::query!(
            "SELECT scores FROM archived_votes.ballots WHERE vote_id = $1",
            id
        )
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        let vote_ballots = ballots.into_iter().map(|b| VoteBallot {
            scores: vote.options.iter().enumerate()
                .map(|(i, opt)| (opt.clone(), b.scores[i] as i8))
                .collect()
        }).collect();

        Ok(Some(Vote {
            id: vote.id,
            title: vote.title,
            description: vote.description,
            options: vote.options,
            end_time: vote.end_time,
            ballots: vote_ballots,
            duration_hours: vote.duration_hours,
            duration_minutes: vote.duration_minutes,
        }))
    }

    pub async fn list_active_votes(pool: &PgPool) -> Result<Vec<Vote>, ApiError> {
        let records = sqlx::query!(
            "SELECT id, title, description, options, end_time, duration_hours, duration_minutes 
             FROM active_votes.votes WHERE state = 'active' ORDER BY created_at DESC"
        )
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        let mut votes = Vec::with_capacity(records.len());
        for vote in records {
            let ballots = sqlx::query!(
                "SELECT scores FROM active_votes.ballots WHERE vote_id = $1",
                vote.id
            )
            .fetch_all(pool)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

            let vote_ballots = ballots.into_iter().map(|b| VoteBallot {
                scores: vote.options.iter().enumerate()
                    .map(|(i, opt)| (opt.clone(), b.scores[i] as i8))
                    .collect()
            }).collect();

            votes.push(Vote {
                id: vote.id,
                title: vote.title,
                description: vote.description,
                options: vote.options,
                end_time: vote.end_time,
                ballots: vote_ballots,
                duration_hours: vote.duration_hours,
                duration_minutes: vote.duration_minutes,
            });
        }
        Ok(votes)
    }

    pub async fn create_vote(pool: &PgPool, vote: &Vote) -> Result<PgQueryResult, ApiError> {
        sqlx::query!(
            "INSERT INTO active_votes.votes 
             (id, title, description, options, end_time, duration_hours, duration_minutes)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            vote.id,
            vote.title,
            vote.description,
            &vote.options,
            vote.end_time,
            vote.duration_hours,
            vote.duration_minutes
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))
    }

    pub async fn cast_ballot(
        pool: &PgPool,
        vote_id: Uuid,
        scores: Vec<i32>,
        voter_fingerprint: String
    ) -> Result<PgQueryResult, ApiError> {
        sqlx::query!(
            "INSERT INTO active_votes.ballots (vote_id, scores, voter_fingerprint)
             VALUES ($1, $2, $3)",
            vote_id,
            &scores,
            voter_fingerprint
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))
    }

    pub async fn get_vote_stats(pool: &PgPool, vote_id: Uuid) -> Result<VoteStats, ApiError> {
        let stats = sqlx::query!(
            "SELECT total_ballots, option_scores FROM calculate_vote_stats($1)",
            vote_id
        )
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        let total_ballots = stats.total_ballots.unwrap_or(0);
        let option_scores: HashMap<String, VoteOptionStats> = 
            serde_json::from_value(stats.option_scores.unwrap_or_default())
                .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(VoteStats {
            option_scores,
            total_ballots: total_ballots as usize
        })
    }
}