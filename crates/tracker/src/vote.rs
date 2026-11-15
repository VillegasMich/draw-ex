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

impl VoteWindow {
    /// An empty window holding the last `size` observations.
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
    /// The leader is the passcode with the highest summed score in the window. It wins
    /// when it has at least `min_votes` votes and its summed score is at least
    /// `min_margin` times the runner-up's (an absent runner-up counts as 0).
    pub(crate) fn winner(&self, min_votes: u32, min_margin: f32) -> Option<Passcode> {
        // (passcode, votes, summed score), in order of first appearance so ties break
        // deterministically. The window is a handful of slots, so a linear scan is fine.
        let mut tally: Vec<(Passcode, u32, f64)> = Vec::with_capacity(self.slots.len());
        for vote in self.slots.iter().flatten() {
            // Summing f32 scores in f64 keeps small sums exact, so a margin met exactly
            // (3 × 0.9 against 1.5 × 2 × 0.9) is not lost to rounding.
            let score = f64::from(vote.score);
            match tally.iter_mut().find(|(p, _, _)| *p == vote.passcode) {
                Some((_, votes, sum)) => {
                    *votes += 1;
                    *sum += score;
                }
                None => tally.push((vote.passcode, 1, score)),
            }
        }

        let mut leader: Option<(Passcode, u32, f64)> = None;
        let mut runner_up_sum = 0.0;
        for &(passcode, votes, sum) in &tally {
            match leader {
                Some((_, _, leader_sum)) if sum <= leader_sum => {
                    runner_up_sum = f64::max(runner_up_sum, sum);
                }
                _ => {
                    if let Some((_, _, previous)) = leader {
                        runner_up_sum = f64::max(runner_up_sum, previous);
                    }
                    leader = Some((passcode, votes, sum));
                }
            }
        }

        let (passcode, votes, sum) = leader?;
        (votes >= min_votes && sum >= f64::from(min_margin) * runner_up_sum).then_some(passcode)
    }
}
