use shared::star_logic::{Ballot, Election};
use std::collections::HashMap;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;
use sqlx::{PgPool, postgres::PgQueryResult};
use shared::models::*;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid duration")]
    InvalidDuration,
    #[error("Invalid ballot")]
    InvalidBallot,
    #[error("Duration must be at least 1 minute")]
    DurationTooShort,
    #[error("Duration cannot exceed 6 days, 23 hours, 59 minutes")]
    DurationTooLong,
    #[error("Vote limit exceeded for user")]
    VoteLimitExceeded,
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Maximum active vote limit ({0}) reached")]
    ActiveVoteLimitExceeded(i64),
}

pub struct VoteProcessor;

impl VoteProcessor {
    pub async fn check_active_vote_limit(pool: &PgPool, limit: i64) -> Result<(), ValidationError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM active_votes.votes WHERE state = 'active'"
        )
        .fetch_one(pool)
        .await
        .map_err(|e| ValidationError::DatabaseError(e.to_string()))?
        .unwrap_or(0);
    
        if count >= limit {
            return Err(ValidationError::ActiveVoteLimitExceeded(limit));
        }
        Ok(())
    }

    pub fn create_vote(request: &CreateVoteRequest) -> Result<Vote, ValidationError> {
        if request.duration_hours == 0 && request.duration_minutes == 0 {
            return Err(ValidationError::DurationTooShort);
        }

        let days = request.duration_hours / 24;
        let hours = request.duration_hours % 24;
        
        if days > 6 || (days == 6 && (hours > 23 || request.duration_minutes > 59)) {
            return Err(ValidationError::DurationTooLong);
        }
        
        Ok(Vote {
            id: Uuid::new_v4(),
            title: request.title.clone(),
            description: request.description.clone(),
            options: request.options.clone(),
            voting_ends_at: OffsetDateTime::now_utc()
                + Duration::hours(request.duration_hours.into())
                + Duration::minutes(request.duration_minutes.into()),
            duration_hours: request.duration_hours,
            duration_minutes: request.duration_minutes,
            ballots: Vec::new(),
            user_fingerprint: request.user_fingerprint.clone(),
        })
    }

    pub async fn create_vote_db(pool: &PgPool, vote: &Vote) -> Result<PgQueryResult, ValidationError> {
        let active_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM active_votes.votes WHERE state = 'active'"
        )
        .fetch_one(pool)
        .await
        .map_err(|e| ValidationError::DatabaseError(e.to_string()))?
        .unwrap_or(0);
        
        if active_count >= 30 {
            return Err(ValidationError::ActiveVoteLimitExceeded(30));
        }
    
        sqlx::query!(
            "INSERT INTO active_votes.votes 
             (id, title, description, options, voting_ends_at, duration_hours, duration_minutes, user_fingerprint, state) 
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'active')",
            vote.id,
            vote.title,
            vote.description,
            &vote.options,
            vote.voting_ends_at,
            vote.duration_hours,
            vote.duration_minutes,
            vote.user_fingerprint,
        )
        .execute(pool)
        .await
        .map_err(|e| 
            if e.to_string().contains("check_user_vote_limit") {
                ValidationError::VoteLimitExceeded
            } else {
                ValidationError::DatabaseError(e.to_string())
            }
        )
    }

    pub async fn get_vote_db(pool: &PgPool, vote_id: Uuid) -> Result<Option<Vote>, ValidationError> {
        Self::fetch_vote_by_id(pool, vote_id).await
    }

    pub async fn fetch_vote_by_id(pool: &PgPool, vote_id: Uuid) -> Result<Option<Vote>, ValidationError> {
        let record = sqlx::query!(
            "SELECT id, title, description, options, voting_ends_at, duration_hours, duration_minutes, user_fingerprint 
             FROM active_votes.votes WHERE id = $1",
            vote_id
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| ValidationError::DatabaseError(e.to_string()))?;
    
        if let Some(vote) = record {
            let ballots = sqlx::query!(
                "SELECT scores, user_fingerprint FROM active_votes.ballots WHERE vote_id = $1",
                vote_id
            )
            .fetch_all(pool)
            .await
            .map_err(|e| ValidationError::DatabaseError(e.to_string()))?;
    
            let vote_ballots = ballots.into_iter()
                .map(|b| VoteBallot { 
                    scores: vote.options.iter().enumerate()
                        .map(|(i, opt)| (opt.clone(), b.scores[i] as i8))
                        .collect(),
                    csrf_token: String::new(),
                    captcha_token: String::new(),
                    user_fingerprint: b.user_fingerprint,
                })
                .collect();
    
            return Ok(Some(Vote {
                id: vote.id,
                title: vote.title,
                description: vote.description.unwrap_or_default(),
                options: vote.options,
                voting_ends_at: vote.voting_ends_at,
                ballots: vote_ballots,
                duration_hours: vote.duration_hours,
                duration_minutes: vote.duration_minutes,
                user_fingerprint: vote.user_fingerprint,
            }));
        }

        let archived = sqlx::query!(
            "SELECT id, title, description, options, voting_ends_at, duration_hours, duration_minutes, user_fingerprint 
             FROM archived_votes.votes WHERE id = $1",
            vote_id
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| ValidationError::DatabaseError(e.to_string()))?;
    
        if let Some(vote) = archived {
            let ballots = sqlx::query!(
                "SELECT scores, user_fingerprint FROM archived_votes.ballots WHERE vote_id = $1",
                vote_id
            )
            .fetch_all(pool)
            .await
            .map_err(|e| ValidationError::DatabaseError(e.to_string()))?;
    
            let vote_ballots = ballots.into_iter()
                .map(|b| VoteBallot { 
                    scores: vote.options.iter().enumerate()
                        .map(|(i, opt)| (opt.clone(), b.scores[i] as i8))
                        .collect(),
                    csrf_token: String::new(),
                    captcha_token: String::new(),
                    user_fingerprint: b.user_fingerprint,
                })
                .collect();
    
            return Ok(Some(Vote {
                id: vote.id,
                title: vote.title,
                description: vote.description.unwrap_or_default(),
                options: vote.options,
                voting_ends_at: vote.voting_ends_at,
                ballots: vote_ballots,
                duration_hours: vote.duration_hours,
                duration_minutes: vote.duration_minutes,
                user_fingerprint: vote.user_fingerprint,
            }));
        }
    
        Ok(None)
    }

    pub fn get_results(vote: &Vote) -> Result<VoteResult, String> {
        if OffsetDateTime::now_utc() <= vote.voting_ends_at {
            return Err("Vote is still ongoing".into());
        }
    
        let mut election = Election::new();
        for option in &vote.options {
            if let Err(e) = election.add_option(option.clone()) {
                return Err(format!("Failed to add option: {}", e));
            }
        }
    
        for ballot in &vote.ballots {
            if let Err(e) = election.cast_ballot(Ballot::new(ballot.scores.clone()).unwrap()) {
                return Err(e.to_string());
            }
        }
    
        match election.determine_winner() {
            Ok(result) => {
                Ok(VoteResult {
                    winner: Some(result.winner),
                    error: None,
                    stats: Self::calculate_stats(vote),
                    head_to_head: Some(HeadToHeadResult {
                        finalist1: result.finalist1,
                        finalist2: result.finalist2,
                        finalist1_votes: result.head_to_head.0,
                        finalist2_votes: result.head_to_head.1
                    }),
                    duration_hours: Some(i64::from(vote.duration_hours)),
                    duration_minutes: Some(i64::from(vote.duration_minutes)),
                })
            },
            Err(e) => Ok(VoteResult {
                winner: None,
                error: Some(e.to_string()),
                stats: Self::calculate_stats(vote),
                head_to_head: None,
                duration_hours: Some(i64::from(vote.duration_hours)),
                duration_minutes: Some(i64::from(vote.duration_minutes)),
            })
        }
    }    

    pub fn calculate_stats(vote: &Vote) -> VoteStats {
        let mut option_scores: HashMap<String, VoteOptionStats> = vote
            .options
            .iter()
            .map(|option| {
                let frequency = (0..=5).map(|score| (score as i8, 0)).collect();
                (
                    option.clone(),
                    VoteOptionStats {
                        total_score: 0,
                        average_score: 0.0,
                        frequency,
                        total_votes: 0,
                    },
                )
            })
            .collect();

        for ballot in &vote.ballots {
            for (option, &score) in &ballot.scores {
                if let Some(stats) = option_scores.get_mut(option) {
                    stats.total_score += i32::from(score);
                    *stats.frequency.entry(score).or_insert(0) += 1;
                    stats.total_votes += 1;
                }
            }
        }

        for stats in option_scores.values_mut() {
            if stats.total_votes > 0 {
                stats.average_score = stats.total_score as f64 / stats.total_votes as f64;
            }
        }

        VoteStats {
            option_scores,
            total_ballots: vote.ballots.len(),
        }
    }

    pub async fn fetch_all_votes(pool: &PgPool) -> Result<Vec<Vote>, ValidationError> {
        let active_records = sqlx::query!(
            "SELECT id, title, description, options, voting_ends_at, duration_hours, duration_minutes, user_fingerprint 
             FROM active_votes.votes 
             WHERE state IN ('active', 'concluded')
             ORDER BY created_at DESC"
        )
        .fetch_all(pool)
        .await
        .map_err(|e| ValidationError::DatabaseError(e.to_string()))?;
    
        let archived_records = sqlx::query!(
            "SELECT id, title, description, options, voting_ends_at, duration_hours, duration_minutes, user_fingerprint 
             FROM archived_votes.votes 
             ORDER BY archived_at DESC"
        )
        .fetch_all(pool)
        .await
        .map_err(|e| ValidationError::DatabaseError(e.to_string()))?;
    
        let mut votes = Vec::with_capacity(active_records.len() + archived_records.len());

        for vote in active_records {
            let ballots = sqlx::query!(
                "SELECT scores, user_fingerprint FROM active_votes.ballots WHERE vote_id = $1",
                vote.id
            )
            .fetch_all(pool)
            .await
            .map_err(|e| ValidationError::DatabaseError(e.to_string()))?;
    
            let vote_ballots = ballots.into_iter()
                .map(|b| VoteBallot {
                    scores: vote.options.iter().enumerate()
                        .map(|(i, opt)| (opt.clone(), b.scores[i] as i8))
                        .collect(),
                    csrf_token: String::new(),
                    captcha_token: String::new(),
                    user_fingerprint: b.user_fingerprint,
                })
                .collect();
    
            votes.push(Vote {
                id: vote.id,
                title: vote.title,
                description: vote.description.unwrap_or_default(),
                options: vote.options,
                voting_ends_at: vote.voting_ends_at,
                ballots: vote_ballots,
                duration_hours: vote.duration_hours,
                duration_minutes: vote.duration_minutes,
                user_fingerprint: vote.user_fingerprint,
            });
        }

        for vote in archived_records {
            let ballots = sqlx::query!(
                "SELECT scores, user_fingerprint FROM archived_votes.ballots WHERE vote_id = $1",
                vote.id
            )
            .fetch_all(pool)
            .await
            .map_err(|e| ValidationError::DatabaseError(e.to_string()))?;
    
            let vote_ballots = ballots.into_iter()
                .map(|b| VoteBallot {
                    scores: vote.options.iter().enumerate()
                        .map(|(i, opt)| (opt.clone(), b.scores[i] as i8))
                        .collect(),
                    csrf_token: String::new(),
                    captcha_token: String::new(),
                    user_fingerprint: b.user_fingerprint,
                })
                .collect();
    
            votes.push(Vote {
                id: vote.id,
                title: vote.title,
                description: vote.description.unwrap_or_default(),
                options: vote.options,
                voting_ends_at: vote.voting_ends_at,
                ballots: vote_ballots,
                duration_hours: vote.duration_hours,
                duration_minutes: vote.duration_minutes,
                user_fingerprint: vote.user_fingerprint,
            });
        }

        votes.sort_by(|a, b| {
            let now = OffsetDateTime::now_utc();
            let a_active = a.voting_ends_at > now;
            let b_active = b.voting_ends_at > now;
            
            match (a_active, b_active) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.voting_ends_at.cmp(&a.voting_ends_at)
            }
        });
    
        Ok(votes)
    }

    pub async fn archive_vote(pool: &PgPool, vote_id: Uuid) -> Result<(), ValidationError> {
        let vote = Self::fetch_vote_by_id(pool, vote_id).await?
            .ok_or_else(|| ValidationError::DatabaseError("Vote not found".into()))?;
    
        let result = Self::get_results(&vote).map_err(|e| ValidationError::DatabaseError(e))?;
        let stats = Self::calculate_stats(&vote);
    
        let mut tx = pool.begin().await
            .map_err(|e| ValidationError::DatabaseError(e.to_string()))?;
    
        sqlx::query!(
            "UPDATE active_votes.votes SET state = 'concluded', archived_at = NOW() WHERE id = $1",
            vote_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ValidationError::DatabaseError(e.to_string()))?;
    
        sqlx::query!(
            r#"
            INSERT INTO archived_votes.votes (
                id, user_fingerprint, title, description, created_at, voting_ends_at,
                archived_at, duration_hours, duration_minutes, options, final_stats,
                winner, head_to_head
            )
            SELECT 
                v.id, v.user_fingerprint, v.title, v.description, v.created_at, v.voting_ends_at,
                v.archived_at, v.duration_hours, v.duration_minutes, v.options, $2::jsonb,
                $3, $4::jsonb
            FROM active_votes.votes v WHERE v.id = $1
            "#,
            vote_id,
            serde_json::to_value(&stats).unwrap(),
            result.winner.unwrap_or_default(),
            serde_json::to_value(&result.head_to_head).unwrap()
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ValidationError::DatabaseError(e.to_string()))?;
    
        sqlx::query!(
            "INSERT INTO archived_votes.ballots (id, vote_id, user_fingerprint, scores, cast_at)
             SELECT id, vote_id, user_fingerprint, scores, cast_at
             FROM active_votes.ballots WHERE vote_id = $1",
            vote_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ValidationError::DatabaseError(e.to_string()))?;
    
        sqlx::query!(
            "DELETE FROM active_votes.votes WHERE id = $1",
            vote_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ValidationError::DatabaseError(e.to_string()))?;
    
        tx.commit().await
            .map_err(|e| ValidationError::DatabaseError(e.to_string()))?;
    
        Ok(())
    }
}