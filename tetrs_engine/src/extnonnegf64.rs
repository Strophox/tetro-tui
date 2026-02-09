/*!
A module that implements a minimalistic wrapper around `f64`, asserting that it is in the range `+0.0 ≤ f ≤ +∞`.
*/

/// An [`f64`] that is known to be non-negative or positive infinity, but not `NaN`, `+0.0 ≤ value ≤ +∞`.
///
/// In precise terms, an extended non-negative `f64` consists of all `value: f64`s that fulfil `0.0f64.total_cmp(&value).is_le() && !value.is_nan()`.
///
/// Unlike `f64`, `ExNonNegF64` does implement [`Eq`], [`Ord`], [`std::hash::Hash`].
#[derive(PartialEq, Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExtNonNegF64(f64);

impl Eq for ExtNonNegF64 {}

impl PartialOrd for ExtNonNegF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExtNonNegF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl std::hash::Hash for ExtNonNegF64 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.to_bits());
    }
}

impl ExtNonNegF64 {
    /// Zero (+0.0).
    pub const ZERO: Self = Self(0f64);

    /// One (+1.0).
    pub const ONE: Self = Self(1f64);

    /// Infinity (+∞)
    pub const INFINITY: Self = Self(f64::INFINITY);

    /// Creates an extended non-negative `f64` if `0.0 <= value`.
    pub fn new(value: f64) -> Option<Self> {
        if 0.0f64.total_cmp(&value).is_le() && !value.is_nan() {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Creates an extended non-negative `f64` without checking whether `0.0 <= value`. This results in undefined behavior if the value is negative and nonzero, negative infinity or NaN.
    ///
    /// # Safety
    /// The value must fulfil `0.0 <= value`.
    pub const unsafe fn new_unchecked(value: f64) -> Self {
        Self(value)
    }

    /// Returns the contained value as `f64`.
    pub const fn get(&self) -> f64 {
        self.0
    }

    /// Subtracts an extended non-negative `f64` from another.
    pub const fn saturating_sub(self, other: Self) -> Self {
        let result = self.0 - other.0;
        if result.is_sign_positive() {
            Self(result)
        } else {
            Self::ZERO
        }
    }

    /// Takes the reciprocal (inverse) of a number, `1/x`.
    pub const fn recip(self) -> Self {
        Self(self.0.recip())
    }
}

impl std::ops::Add for ExtNonNegF64 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign for ExtNonNegF64 {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

/*#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let res = add((1,2),(3,4));
        assert_eq!(res, (4,6));
    }
}*/
