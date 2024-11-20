use std::collections::HashMap;
use std::hash::Hash;
use std::cmp::Ordering;
use thiserror::Error;
use serde::{Serialize, Deserialize};

#[derive(Error, Debug, Clone)]
pub enum VotingError<T> {
    #[error("Invalid score {0}. Must be 0-5")] InvalidScore(i8),
    #[error("Invalid option: {0:?}")] InvalidOption(T),
    #[error("Duplicate option: {0:?}")] DuplicateOption(T),
    #[error("Need at least 2 options")] InsufficientOptions,
    #[error("Perfect tie for first")] FirstPlaceTie,
    #[error("Tie for second")] SecondPlaceTie,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Score { Zero, One, Two, Three, Four, Five }

impl Score {
    pub const fn as_i8(self) -> i8 { 
        self as i8 
    }
}

impl TryFrom<i8> for Score {
    type Error = i8;
    
    fn try_from(v: i8) -> Result<Self, i8> {
        match v {
            0 => Ok(Score::Zero),
            1 => Ok(Score::One),
            2 => Ok(Score::Two),
            3 => Ok(Score::Three),
            4 => Ok(Score::Four),
            5 => Ok(Score::Five),
            n => Err(n),
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Ballot<T: Clone + Eq + Hash> {
    scores: HashMap<T, Score>
}

impl<T: Clone + Eq + Hash> Ballot<T> {
    pub fn new(scores_map: HashMap<T, i8>) -> Result<Self, VotingError<T>> {
        scores_map.into_iter()
            .map(|(k, v)| Score::try_from(v)
                .map(|score| (k, score))
                .map_err(VotingError::InvalidScore))
            .collect::<Result<HashMap<_, _>, _>>()
            .map(|scores| Self { scores })
    }
    
    pub fn scores(&self) -> &HashMap<T, Score> { &self.scores }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreMetrics {
    total: i32,
    by_value: [u32; 6]
}

impl ScoreMetrics {
    fn record(&mut self, score: Score) {
        self.total += i32::from(score.as_i8());
        self.by_value[score.as_i8() as usize] += 1;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VotingOption<T> {
    value: T,
    metrics: ScoreMetrics,
    order: u64
}

impl<T: Clone + Eq + Hash> VotingOption<T> {
    fn new(value: T, order: u64) -> Self {
        Self { value, metrics: ScoreMetrics::default(), order }
    }
    pub fn value(&self) -> &T { &self.value }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadToHeadMatchup<T> {
    pub candidate1: T,
    pub candidate2: T,
    pub votes1: u32,
    pub votes2: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunoffResult<T> {
    pub winner: T,
    pub finalist1: T,
    pub finalist2: T,
    pub head_to_head: (u32, u32),
}

#[derive(Debug)]
struct SortedOption<'a, T> {
    option: &'a VotingOption<T>,
    idx: usize,
}

#[derive(Debug)]
pub struct Election<T: Clone + Eq + Hash + Ord> {
    options: HashMap<T, VotingOption<T>>,
    ballots: Vec<Ballot<T>>,
    option_order: u64,
}

impl<T: Clone + Eq + Hash + Ord> Election<T> {
    pub fn new() -> Self {
        Self { options: HashMap::new(), ballots: Vec::new(), option_order: 0 }
    }

    pub fn add_option(&mut self, option: T) -> Result<(), VotingError<T>> {
        if self.options.contains_key(&option) {
            return Err(VotingError::DuplicateOption(option));
        }
        self.options.insert(option.clone(), VotingOption::new(option, self.option_order));
        self.option_order += 1;
        Ok(())
    }

    pub fn cast_ballot(&mut self, ballot: Ballot<T>) -> Result<(), VotingError<T>> {
        for (option, score) in ballot.scores() {
            self.options.get_mut(option)
                .ok_or_else(|| VotingError::InvalidOption(option.clone()))?
                .metrics.record(*score);
        }
        self.ballots.push(ballot);
        Ok(())
    }

    fn get_head_to_head_votes(&self, option1: &T, option2: &T) -> (u32, u32) {
        self.ballots.iter().fold((0, 0), |(v1, v2), ballot| {
            match (ballot.scores().get(option1), ballot.scores().get(option2)) {
                (Some(&s1), Some(&s2)) => match s1.as_i8().cmp(&s2.as_i8()) {
                    Ordering::Greater => (v1 + 1, v2),
                    Ordering::Less => (v1, v2 + 1),
                    Ordering::Equal => (v1, v2),
                },
                _ => (v1, v2),
            }
        })
    }

    fn sort_options_by_score(&self) -> Result<Vec<SortedOption<T>>, VotingError<T>> {
        if self.options.is_empty() {
            return Err(VotingError::InsufficientOptions);
        }
    
        let mut sorted: Vec<_> = self.options.values()
            .enumerate()
            .map(|(i, opt)| SortedOption { option: opt, idx: i })
            .collect();
            
        sorted.sort_unstable_by(|a, b| {
            b.option.metrics.total.cmp(&a.option.metrics.total)
                .then_with(|| {
                    let b_nonzero: u32 = b.option.metrics.by_value[1..].iter().sum();
                    let a_nonzero: u32 = a.option.metrics.by_value[1..].iter().sum();
                    b_nonzero.cmp(&a_nonzero)
                })
                .then_with(|| b.option.metrics.by_value[5].cmp(&a.option.metrics.by_value[5]))
                .then_with(|| b.option.metrics.by_value[4].cmp(&a.option.metrics.by_value[4]))
                .then_with(|| a.option.metrics.by_value[0].cmp(&b.option.metrics.by_value[0]))
                .then_with(|| a.option.metrics.by_value[1].cmp(&b.option.metrics.by_value[1]))
                .then_with(|| a.idx.cmp(&b.idx))
        });
        Ok(sorted)
    }

    fn is_perfect_tie(candidates: &[SortedOption<T>]) -> bool {
        candidates.windows(2).next().map_or(false, |w| {
            w[0].option.metrics.by_value == w[1].option.metrics.by_value &&
            w[0].option.metrics.by_value[1..].iter().sum::<u32>() ==
            w[1].option.metrics.by_value[1..].iter().sum::<u32>()
        })
    }

    fn select_finalists(&self) -> Result<(&VotingOption<T>, &VotingOption<T>), VotingError<T>> {
        let sorted = self.sort_options_by_score()?;
        if sorted.len() < 2 { return Err(VotingError::InsufficientOptions); }
        
        let first_ties = sorted.windows(2)
            .take_while(|w| w[0].option.metrics.total == w[1].option.metrics.total)
            .count() + 1;
     
        if first_ties > 1 {
            let tied = &sorted[..first_ties];
            if Self::is_perfect_tie(tied) {
                return Err(VotingError::FirstPlaceTie);
            }
            return Ok((tied[0].option, tied[1].option));
        }
        
        let second_ties = sorted.windows(2).skip(1)
            .take_while(|w| w[0].option.metrics.total == w[1].option.metrics.total)
            .count() + 1;
    
        if second_ties > 1 {
            let tied = &sorted[1..=second_ties];
            if Self::is_perfect_tie(tied) {
                if tied.iter().all(|c| {
                    let (w, t) = self.get_head_to_head_votes(sorted[0].option.value(), c.option.value());
                    w > t
                }) {
                    return Ok((sorted[0].option, tied[0].option));
                }
                return Err(VotingError::SecondPlaceTie);
            }
            return Ok((sorted[0].option, tied[0].option));
        }
        Ok((&sorted[0].option, &sorted[1].option))
    }

    pub fn determine_winner(&self) -> Result<RunoffResult<T>, VotingError<T>> {
        let sorted = self.sort_options_by_score()?;
        if sorted.len() < 2 { return Err(VotingError::InsufficientOptions); }
     
        let (f1, f2) = self.select_finalists()?;
        let (p1, p2) = self.get_head_to_head_votes(f1.value(), f2.value());
        let winner = if p1 >= p2 { f1.value() } else { f2.value() };

        let mut additional = Vec::new();
        for runner_up in sorted.iter().skip(1).take(3) {
            if runner_up.option.value() != f2.value() {
                let (w, r) = self.get_head_to_head_votes(winner, runner_up.option.value());
                additional.push(HeadToHeadMatchup {
                    candidate1: winner.clone(),
                    candidate2: runner_up.option.value().clone(),
                    votes1: w,
                    votes2: r,
                });
            }
        }
     
        Ok(RunoffResult {
            winner: winner.clone(),
            finalist1: f1.value().clone(),
            finalist2: f2.value().clone(),
            head_to_head: (p1, p2),
        })
    }
}