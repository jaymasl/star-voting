#[cfg(test)]
mod tests {
    use std::{fmt::Debug, hash::Hash};
    use crate::star_logic::{Ballot, Election, Score, VotingError, RunoffResult};

    fn ballot<T: Clone + Eq + Hash + Debug>(scores: &[(T, i8)]) -> Ballot<T> {
        Ballot::new(scores.iter().cloned().collect()).unwrap()
    }

    fn election<T: Clone + Eq + Hash + Ord + Debug>(options: &[T]) -> Election<T> {
        let mut e = Election::new();
        options.iter().for_each(|opt| e.add_option(opt.clone()).unwrap());
        e
    }

    #[test]
    fn test_basic_validation() {
        assert!(Score::try_from(0).is_ok());
        assert!(Score::try_from(6).is_err());

        assert!(Ballot::<&str>::new([("A", 5)].iter().cloned().collect()).is_ok());
        assert!(matches!(
            Ballot::<&str>::new([("A", 6)].iter().cloned().collect()),
            Err(VotingError::InvalidScore(6))
        ));

        let mut e = election(&["A"]);
        assert!(matches!(e.add_option("A"), Err(VotingError::DuplicateOption("A"))));
    }

    #[test]
    fn test_ties() {
        let mut e = election(&["A", "B"]);
        e.cast_ballot(ballot(&[("A", 5), ("B", 5)])).unwrap();
        e.cast_ballot(ballot(&[("A", 5), ("B", 5)])).unwrap();
        assert!(matches!(e.determine_winner(), Err(VotingError::FirstPlaceTie)));

        let mut e = election(&["A", "B", "C"]);
        let ballots = [
            ballot(&[("A", 5), ("B", 3), ("C", 3)]),
            ballot(&[("A", 4), ("B", 3), ("C", 3)]),
            ballot(&[("A", 4), ("B", 3), ("C", 3)]),
            ballot(&[("A", 0), ("B", 3), ("C", 3)]),
        ];
        for b in ballots {
            e.cast_ballot(b).unwrap();
        }
        let result = e.determine_winner().unwrap();
        assert_eq!(result.winner, "A");
        assert!(result.head_to_head.0 > result.head_to_head.1);
    }

    #[test]
    fn test_winner_determination() {
        let mut e = election(&["A", "B", "C"]);
        let ballots = [
            ballot(&[("A", 5), ("B", 4), ("C", 0)]),
            ballot(&[("A", 4), ("B", 3), ("C", 0)]),
            ballot(&[("A", 4), ("B", 3), ("C", 0)]),
            ballot(&[("A", 3), ("B", 2), ("C", 0)]),
        ];
        ballots.into_iter().for_each(|b| e.cast_ballot(b).unwrap());

        let RunoffResult { winner, finalist1, head_to_head, .. } = e.determine_winner().unwrap();
        assert_eq!(winner, "A");
        assert_eq!(finalist1, "A");
        assert_eq!(head_to_head.0 > head_to_head.1, true);
    }

    #[test]
    fn test_edge_cases() {
        let mut e = Election::<&str>::new();
        assert!(matches!(e.determine_winner(), Err(VotingError::InsufficientOptions)));

        e.add_option("A").unwrap();
        assert!(matches!(e.determine_winner(), Err(VotingError::InsufficientOptions)));

        e.add_option("B").unwrap();
        e.cast_ballot(ballot(&[("A", 5), ("B", 0)])).unwrap();
        assert_eq!(e.determine_winner().unwrap().winner, "A");
    }

    #[test]
    fn test_multiple_ballots_varied_scores() {
        let mut e = election(&["A", "B", "C"]);
        [
            ballot(&[("A", 5), ("B", 3), ("C", 1)]),
            ballot(&[("A", 3), ("B", 5), ("C", 2)]),
            ballot(&[("A", 2), ("B", 4), ("C", 5)]),
            ballot(&[("A", 5), ("B", 2), ("C", 3)]),
        ]
        .iter()
        .for_each(|b| e.cast_ballot(b.clone()).unwrap());
        assert_eq!(e.determine_winner().unwrap().winner, "A");
    }

    #[test]
    fn test_large_tie_scenario() {
        let mut e = election(&["A", "B", "C", "D"]);
        let ballots = [
            ballot(&[("A", 5), ("B", 5), ("C", 5), ("D", 5)]),
            ballot(&[("A", 5), ("B", 5), ("C", 5), ("D", 5)]),
        ];
        for b in ballots { e.cast_ballot(b).unwrap(); }
        assert!(matches!(e.determine_winner(), Err(VotingError::FirstPlaceTie)));
    }

    #[test]
    fn test_close_runoff() {
        let mut e = election(&["A", "B", "C"]);
        [
            ballot(&[("A", 4), ("B", 4), ("C", 0)]),
            ballot(&[("A", 4), ("B", 3), ("C", 0)]),
            ballot(&[("A", 3), ("B", 5), ("C", 1)]),
        ]
        .iter()
        .for_each(|b| e.cast_ballot(b.clone()).unwrap());
        let result = e.determine_winner().unwrap();
        assert_eq!(result.winner, "B");
        assert_eq!(result.finalist1, "B");
    }

    #[test]
    fn test_different_ordering() {
        let mut e = election(&["X", "Y", "Z"]);
        [
            ballot(&[("X", 5), ("Y", 4), ("Z", 1)]),
            ballot(&[("X", 3), ("Y", 5), ("Z", 2)]),
            ballot(&[("X", 4), ("Y", 2), ("Z", 5)]),
        ]
        .iter()
        .for_each(|b| e.cast_ballot(b.clone()).unwrap());
        let result = e.determine_winner().unwrap();
        assert_eq!(result.winner, "X");
    }

    #[test]
    fn test_invalid_ballot_scores() {
        assert!(matches!(
            Ballot::<&str>::new([("A", 6)].iter().cloned().collect()),
            Err(VotingError::InvalidScore(6))
        ));
        assert!(matches!(
            Ballot::<&str>::new([("A", -1)].iter().cloned().collect()),
            Err(VotingError::InvalidScore(-1))
        ));
    }

    #[test]
    fn test_empty_ballot() {
        let empty_ballot = Ballot::<&str>::new([].iter().cloned().collect());
        assert!(matches!(empty_ballot, Ok(_)), "Empty ballot should be allowed");
    }

    #[test]
    fn test_nonexistent_option_in_ballot() {
        let mut e = election(&["A", "B"]);
        let ballot = ballot(&[("C", 5)]);
        assert!(matches!(e.cast_ballot(ballot), Err(VotingError::InvalidOption(_))));
    }

    #[test]
    fn test_all_zero_scores() {
        let mut e = election(&["A", "B", "C"]);
        let zero_ballot = ballot(&[("A", 0), ("B", 0), ("C", 0)]);
        assert!(e.cast_ballot(zero_ballot).is_ok(), "All-zero scores should be accepted");
        let result = e.determine_winner();
        assert!(result.is_ok() || matches!(result, Err(VotingError::FirstPlaceTie)), "All zero scores should be valid but may result in a tie");
    }

    #[test]
    fn test_multiple_identical_ballots() {
        let mut e = election(&["A", "B"]);
        let ballots = [
            ballot(&[("A", 3), ("B", 2)]),
            ballot(&[("A", 3), ("B", 2)]),
            ballot(&[("A", 3), ("B", 2)]),
        ];
        for b in ballots {
            e.cast_ballot(b).unwrap();
        }
        let result = e.determine_winner().unwrap();
        assert_eq!(result.winner, "A");
    }

    #[test]
    fn test_insufficient_options() {
        let mut e = Election::<&str>::new();
        e.add_option("A").unwrap();
        assert!(matches!(e.determine_winner(), Err(VotingError::InsufficientOptions)));
    }

}