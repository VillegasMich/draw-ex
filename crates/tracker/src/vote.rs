//! Owned by C1 (docs/ROADMAP.md): the weighted vote over a track's candidates.

use std::collections::VecDeque;

use domain::Passcode;

/// One observation's contribution to a track's vote: the passcode recognition settled
/// on, with its score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Vote {
    pub(crate) passcode: Passcode,
    pub(crate) score: f32,
}

/// A sliding window over a track's most recent observations. Each slot holds that
/// observation's vote, or `None` when it recognized nothing, so unreadable observations
/// still push old votes out.
#[derive(Debug, Clone)]
pub(crate) struct VoteWindow {
    slots: VecDeque<Option<Vote>>,
    size: usize,
}

/// One passcode's standing in a window.
#[derive(Debug, Clone, Copy)]
struct Tally {
    passcode: Passcode,
    votes: u32,
    /// Summed in f64 so small sums of f32 scores are exact, and a margin met exactly
    /// (3 × 0.9 against 1.5 × 2 × 0.9) is not lost to rounding.
    score: f64,
}

impl Tally {
    /// Whether this tally leads `other`: a higher summed score, then more votes.
    fn leads(&self, other: &Self) -> bool {
        (self.score, self.votes) > (other.score, other.votes)
    }
}

impl VoteWindow {
    /// An empty window holding the last `size` observations.
    #[must_use]
    pub(crate) fn new(size: usize) -> Self {
        Self {
            slots: VecDeque::with_capacity(size),
            size,
        }
    }

    /// Adds the newest observation's vote, dropping the oldest once the window is full.
    pub(crate) fn push(&mut self, vote: Option<Vote>) {
        if self.size == 0 {
            return;
        }
        if self.slots.len() == self.size {
            self.slots.pop_front();
        }
        self.slots.push_back(vote);
    }

    /// The passcode that wins the vote, if any.
    ///
    /// The leader is the passcode with the highest summed score in the window (then the
    /// most votes). It wins when it has at least `min_votes` votes and its summed score
    /// is at least `min_margin` times the runner-up's (an absent runner-up counts as 0).
    #[must_use]
    pub(crate) fn winner(&self, min_votes: u32, min_margin: f32) -> Option<Passcode> {
        // The window is a handful of slots, so a linear scan is fine.
        let mut tallies: Vec<Tally> = Vec::with_capacity(self.slots.len());
        for vote in self.slots.iter().flatten() {
            let score = f64::from(vote.score);
            match tallies.iter_mut().find(|t| t.passcode == vote.passcode) {
                Some(tally) => {
                    tally.votes += 1;
                    tally.score += score;
                }
                None => tallies.push(Tally {
                    passcode: vote.passcode,
                    votes: 1,
                    score,
                }),
            }
        }

        let leader = tallies
            .iter()
            .copied()
            .reduce(|best, tally| if tally.leads(&best) { tally } else { best })?;
        let runner_up_score = tallies
            .iter()
            .filter(|tally| tally.passcode != leader.passcode)
            .map(|tally| tally.score)
            .fold(0.0, f64::max);

        (leader.votes >= min_votes && leader.score >= f64::from(min_margin) * runner_up_score)
            .then_some(leader.passcode)
    }
}
