/*!
A module that implements a minimalistic wrapper around [`Duration`], adding that it may be infinite.
*/

use std::{
    ops::{Add, AddAssign},
    time::Duration,
};

use crate::ExtNonNegF64;

/// A [`Duration`] that may also be infinite.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExtDuration {
    /// A finite duration.
    Finite(Duration),
    /// Infinite duration.
    Infinite,
}

impl Default for ExtDuration {
    fn default() -> Self {
        Self::Finite(Duration::default())
    }
}

impl From<Duration> for ExtDuration {
    fn from(value: Duration) -> Self {
        ExtDuration::Finite(value)
    }
}

impl Add for ExtDuration {
    type Output = ExtDuration;

    fn add(self, rhs: Self) -> Self::Output {
        // Saturating `ExtDuration` addition.
        // Computes `self + other`, returning `ExtDuration::Infinite` if result would overflow `ExtDuration::Finite(Duration::MAX)`.
        match (self, rhs) {
            (ExtDuration::Finite(dur0), ExtDuration::Finite(dur1))
                if dur0 <= Duration::MAX.saturating_sub(dur1) =>
            {
                ExtDuration::Finite(dur0.saturating_add(dur1))
            }
            _ => ExtDuration::Infinite,
        }
    }
}

impl AddAssign for ExtDuration {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl ExtDuration {
    /// An extended duration of zero time.
    pub const ZERO: Self = Self::Finite(Duration::ZERO);

    /// Returns the number of seconds contained by this `ExtDuration`, saturating to [`Duration::MAX`] if infinite.
    pub const fn saturating_duration(&self) -> Duration {
        match self {
            ExtDuration::Finite(dur) => *dur,
            ExtDuration::Infinite => Duration::MAX,
        }
    }

    /// Returns the number of seconds contained by this `ExtDuration` as `ExtNonNegF64`.
    pub const fn as_secs_ennf64(&self) -> ExtNonNegF64 {
        match self {
            ExtDuration::Finite(dur) => {
                // SAFETY: `+0.0 <= dur.as_secs_f64()`.
                unsafe { ExtNonNegF64::new_unchecked(dur.as_secs_f64()) }
            }
            ExtDuration::Infinite => ExtNonNegF64::MAX,
        }
    }

    /// Given a time delay a piece takes to fall one unit, returns how many units the piece falls per second ('1/s' or 'Hz').
    pub const fn as_hertz(&self) -> ExtNonNegF64 {
        self.as_secs_ennf64().recip()
    }

    /// Saturating `ExtDuration` multiplication.
    /// Computes `self * rhs`, returning `ExtDuration::Infinite` if result would overflow `ExtDuration::Finite(Duration::MAX)`.
    pub fn mul_ennf64(self, rhs: ExtNonNegF64) -> Self {
        match self {
            // Divide would (kind of) not overflow.
            ExtDuration::Finite(dur)
                if dur.as_secs_f64() * rhs.get() <= Duration::MAX.as_secs_f64() =>
            {
                ExtDuration::Finite(dur.mul_f64(rhs.get()))
            }

            _ => ExtDuration::Infinite,
        }
    }

    /// Saturating `ExtDuration` division.
    /// Computes `self / rhs`, returning `ExtDuration::Infinite` if result would overflow `ExtDuration::Finite(Duration::MAX)`.
    pub fn div_ennf64(self, rhs: ExtNonNegF64) -> Self {
        match self {
            // Divide would (kind of) not overflow.
            ExtDuration::Finite(dur)
                if dur.as_secs_f64() / rhs.get() <= Duration::MAX.as_secs_f64() =>
            {
                ExtDuration::Finite(dur.div_f64(rhs.get()))
            }

            _ => ExtDuration::Infinite,
        }
    }

    /// Saturating `ExtDuration` addition.
    /// Computes `self + other`, returning `ExtDuration::Infinite` if result would overflow `ExtDuration::Finite(Duration::MAX)`.
    pub fn saturating_sub(self, other: ExtDuration) -> Self {
        match (self, other) {
            (ExtDuration::Finite(dur0), ExtDuration::Finite(dur1)) => {
                dur0.saturating_sub(dur1).into()
            }
            (ExtDuration::Infinite, ExtDuration::Finite(_)) => ExtDuration::Infinite,
            (ExtDuration::Finite(_), ExtDuration::Infinite) => ExtDuration::ZERO,
            (ExtDuration::Infinite, ExtDuration::Infinite) => ExtDuration::ZERO, // Controversial?
        }
    }
}

/*#[cfg(test)] TODO
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let res = add((1,2),(3,4));
        assert_eq!(res, (4,6));
    }
}*/
