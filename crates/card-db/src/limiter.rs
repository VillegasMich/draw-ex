//! Owned by B1 (docs/ROADMAP.md): the single rate limiter shared by every YGOPRODeck request.
//!
//! YGOPRODeck blocks an IP for an hour above 20 requests per second. The limiter allows
//! 10 per second with no burst, so requests are spaced at least 100 ms apart and no
//! one-second window can come near the ban limit.

use std::num::NonZeroU32;

use governor::{DefaultDirectRateLimiter, Quota};

/// YGOPRODeck requests allowed per second: half the ban limit.
const YGOPRODECK_PER_SECOND: NonZeroU32 = NonZeroU32::new(10).expect("10 is not zero");

/// Spaces requests evenly at a fixed rate, with no burst. Shared through an `Arc` by
/// everything that talks to YGOPRODeck (the card data client and the image cache).
#[derive(Debug)]
pub(crate) struct RateLimiter(DefaultDirectRateLimiter);

impl RateLimiter {
    /// The limiter for YGOPRODeck: 10 requests per second.
    pub(crate) fn ygoprodeck() -> Self {
        Self::per_second(YGOPRODECK_PER_SECOND)
    }

    /// A limiter allowing `rate` requests per second, one at a time.
    fn per_second(rate: NonZeroU32) -> Self {
        let one = NonZeroU32::MIN;
        Self(DefaultDirectRateLimiter::direct(
            Quota::per_second(rate).allow_burst(one),
        ))
    }

    /// Waits until one more request is allowed. Call it right before each request.
    pub(crate) async fn until_ready(&self) {
        self.0.until_ready().await;
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    #[tokio::test]
    async fn requests_are_spaced_by_the_rate_with_no_burst() {
        // 100 per second: 6 requests need at least 5 gaps of 10 ms.
        let limiter = RateLimiter::per_second(NonZeroU32::new(100).unwrap());
        let start = Instant::now();

        for _ in 0..6 {
            limiter.until_ready().await;
        }

        assert!(
            start.elapsed() >= Duration::from_millis(50),
            "6 requests took only {:?}",
            start.elapsed()
        );
    }
}
