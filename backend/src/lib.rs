pub mod processor;
pub mod routes;
pub mod store;
pub mod cors;
pub mod error;
pub mod utils;
pub mod rate_limiter;
pub mod catchers;
pub mod captcha;
pub use shared::user_info;
pub use shared::{models::*, error::*, user_info::*};
pub use shared::star_logic::{Ballot, Election, Score, VotingError, HeadToHeadMatchup, RunoffResult};

#[cfg(test)]
mod tests;