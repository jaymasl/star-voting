pub mod error;
pub mod models;
pub mod validation;
pub mod user_info;
pub mod star_logic;

pub use error::{Error, ErrorCode, Result, ErrorResponse};
pub use models::*;
pub use validation::*;
pub use user_info::*;
pub use star_logic::{Ballot, Election, Score, VotingError, HeadToHeadMatchup, RunoffResult};

#[cfg(test)]
mod tests;