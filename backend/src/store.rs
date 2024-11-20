use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;
use shared::models::*;

pub type VoteStore = Mutex<HashMap<Uuid, Vote>>;

#[derive(Debug)]
pub enum StoreError {
    NotFound,
    LockFailed,
    InvalidUuid,
}