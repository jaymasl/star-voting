use std::collections::HashSet;
use crate::models::{CreateVoteRequest, VoteBallot};

pub const MAX_TITLE_LENGTH: usize = 100;
pub const MAX_DESCRIPTION_LENGTH: usize = 500;
pub const MAX_OPTION_LENGTH: usize = 40;
pub const MAX_OPTIONS: usize = 20;
pub const MAX_DURATION_DAYS: i32 = 6;
pub const MAX_DURATION_HOURS: i32 = 23;
pub const MAX_DURATION_MINUTES: i32 = 59;
pub const MIN_OPTIONS: usize = 2;
pub const MIN_SCORE: i8 = 0;
pub const MAX_SCORE: i8 = 5;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Title exceeds maximum length of {MAX_TITLE_LENGTH}")]
    TitleTooLong,
    #[error("Description exceeds maximum length of {MAX_DESCRIPTION_LENGTH}")]
    DescriptionTooLong,
    #[error("Option text exceeds maximum length of {MAX_OPTION_LENGTH}")]
    OptionTooLong,
    #[error("Too many options (maximum {MAX_OPTIONS})")]
    TooManyOptions,
    #[error("Too few options (minimum {MIN_OPTIONS})")]
    TooFewOptions,
    #[error("Duration exceeds maximum of {MAX_DURATION_DAYS} days, {MAX_DURATION_HOURS} hours, {MAX_DURATION_MINUTES} minutes")]
    DurationTooLong,
    #[error("Duration must be at least 1 minute")]
    DurationTooShort,
    #[error("Invalid score: {0} (must be {MIN_SCORE}-{MAX_SCORE})")]
    InvalidScore(i8),
    #[error("Duplicate option: {0}")]
    DuplicateOption(String),
    #[error("Empty option text")]
    EmptyOption,
    #[error("Invalid ballot: {0}")]
    InvalidBallot(String),
}

pub fn validate_vote_request(request: &CreateVoteRequest) -> Result<(), ValidationError> {
    if request.title.len() > MAX_TITLE_LENGTH { return Err(ValidationError::TitleTooLong); }
    if request.description.len() > MAX_DESCRIPTION_LENGTH { return Err(ValidationError::DescriptionTooLong); }
    if request.options.len() > MAX_OPTIONS { return Err(ValidationError::TooManyOptions); }
    if request.options.len() < MIN_OPTIONS { return Err(ValidationError::TooFewOptions); }

    let seen_options = request.options.iter()
        .map(|opt| opt.to_lowercase())
        .collect::<HashSet<_>>();

    if seen_options.len() != request.options.len() {
        return Err(ValidationError::DuplicateOption(
            request.options.iter()
                .find(|opt| 
                    request.options.iter()
                        .filter(|o| o.to_lowercase() == opt.to_lowercase())
                        .count() > 1
                )
                .unwrap()
                .clone()
        ));
    }

    if request.options.iter().any(|opt| opt.is_empty()) { return Err(ValidationError::EmptyOption); }
    if request.options.iter().any(|opt| opt.len() > MAX_OPTION_LENGTH) { return Err(ValidationError::OptionTooLong); }

    let (total_hours, total_minutes) = (request.duration_hours, request.duration_minutes);
    if total_hours == 0 && total_minutes == 0 { return Err(ValidationError::DurationTooShort); }

    let (days, hours) = (total_hours / 24, total_hours % 24);
    if days > MAX_DURATION_DAYS || 
       (days == MAX_DURATION_DAYS && (hours > MAX_DURATION_HOURS || 
        (hours == MAX_DURATION_HOURS && total_minutes > MAX_DURATION_MINUTES))) {
        return Err(ValidationError::DurationTooLong);
    }

    Ok(())
}

pub fn validate_ballot(ballot: &VoteBallot, options: &[String]) -> Result<(), ValidationError> {
    if let Some(&invalid_score) = ballot.scores.values().find(|&&score| score < MIN_SCORE || score > MAX_SCORE) {
        return Err(ValidationError::InvalidScore(invalid_score));
    }

    let invalid_option = ballot.scores.keys().find(|opt| !options.contains(opt));
    if let Some(opt) = invalid_option {
        return Err(ValidationError::InvalidBallot(format!("Invalid option: {}", opt)));
    }

    let missing_option = options.iter().find(|opt| !ballot.scores.contains_key(*opt));
    if let Some(opt) = missing_option {
        return Err(ValidationError::InvalidBallot(format!("Missing score for option: {}", opt)));
    }

    Ok(())
}