#![allow(unused)]

use core::ops::{Add, AddAssign, Sub, SubAssign};
use core::{
    cmp::{Ord, Ordering, PartialOrd},
    hash::{Hash, Hasher},
    time::Duration,
};
use crossbus::time::Timing;

// wasm32
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Clock(js_sys::Date);

impl Hash for Clock {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.0.get_time() as u64).hash(state);
    }
}

impl PartialOrd for Clock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.get_time().partial_cmp(&other.0.get_time())
    }
}

impl Ord for Clock {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.0.get_time() as u64).cmp(&(other.0.get_time() as u64))
    }
}

impl Timing for Clock {
    fn now() -> Self {
        let date = js_sys::Date::new_0();
        Self(date)
    }

    fn duration_since(&self, other: &Self) -> Duration {
        let delta = self.0.get_time() - other.0.get_time();
        Duration::from_secs_f64(delta as f64 / 1000.0)
    }

    #[cfg(all(feature = "time-metric", feature = "unstable"))]
    fn as_secs_f64(&self) -> f64 {
        self.0.get_time() as f64 / 1000.0
    }
}

impl Add<Duration> for Clock {
    type Output = Clock;

    fn add(self, other: Duration) -> Self {
        let mut amt = self.0.get_time() as f64;
        let delta = other.as_secs_f64() * 1000.0;
        amt = amt + delta;
        self.0.set_time(amt as _);
        self
    }
}

impl AddAssign<Duration> for Clock {
    fn add_assign(&mut self, other: Duration) {
        let mut amt = self.0.get_time();
        let delta = other.as_secs_f64() * 1000.0;
        amt = amt + delta;
        self.0.set_time(amt as _);
    }
}

impl SubAssign<Duration> for Clock {
    fn sub_assign(&mut self, other: Duration) {
        let mut amt = self.0.get_time();
        let delta = other.as_secs_f64() * 1000.0;
        amt = amt - delta;
        self.0.set_time(amt as _);
    }
}

impl Sub<Duration> for Clock {
    type Output = Clock;

    fn sub(self, other: Duration) -> Self {
        let amt = self.0.get_time() as f64 - other.as_secs_f64() * 1000.0;
        self.0.set_time(amt as _);
        self
    }
}
