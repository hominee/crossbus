//! time module
//!
use core::{
    cmp::Eq,
    fmt::Debug,
    hash::Hash,
    ops::{Add, AddAssign, Sub, SubAssign},
    time::Duration,
};

/// An abstraction for to enable Actor known time
///
pub trait Timing
where
    Self: Add<Duration, Output = Self>
        + Sub<Duration, Output = Self>
        + AddAssign<Duration>
        + SubAssign<Duration>
        + Unpin
        //+ Copy
        + Clone
        + PartialEq
        + Eq
        + Ord
        + PartialOrd
        + Hash
        + Debug,
{
    /// Returns an instant corresponding to “now”.
    fn now() -> Self;

    /// Returns the amount of time elapsed from
    /// another instant to this one, or zero duration
    /// if that instant is later than this one.
    fn duration_since(&self, other: &Self) -> Duration;

    /// Returns the number of seconds contained as f64.
    #[cfg(all(feature = "time-metric", feature = "unstable"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "time-metric", feature = "unstable"))))]
    fn as_secs_f64(&self) -> f64;
}

#[cfg(feature = "std")]
pub use std::time::Instant as Clock;
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl Timing for Clock {
    fn now() -> Self {
        std::time::Instant::now()
    }

    fn duration_since(&self, other: &Self) -> Duration {
        self.duration_since(other.clone())
    }

    /// **Safety**: since std::intant have no method to
    /// provide the number of seconds from a fixed
    /// point. but its inner data gives that
    #[cfg(all(feature = "time-metric", feature = "unstable"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "time-metric", feature = "unstable"))))]
    fn as_secs_f64(&self) -> f64 {
        use core::mem::transmute;
        const NSEC_PER_SEC: f64 = 1_000_000_000.0;

        let (secs, frac) = unsafe { transmute::<Self, (i64, i64)>(*self) };
        secs as f64 + frac as f64 / NSEC_PER_SEC
    }
}
