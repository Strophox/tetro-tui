/*!
This module handles random generation of [`Tetromino`]s.
*/

use std::num::NonZeroU32;

use rand::{
    self,
    distributions::{Distribution, WeightedIndex},
    //prelude::SliceRandom, // vec.shuffle(rng)...
    Rng,
};

use crate::Tetromino;

/// Handles the information of which pieces to spawn during a game.
///
/// To actually generate [`Tetromino`]s, the [`TetrominoSource::with_rng`] method needs to be used to yield a
/// [`TetrominoIterator`] that implements [`Iterator`].
#[derive(PartialEq, PartialOrd, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TetrominoSource {
    /// Uniformly random piece generator.
    Uniform,
    /// Standard 'bag' generator.
    ///
    /// Stock works by picking `n` copies of each [`Tetromino`] type, and then uniformly randomly
    /// handing them out until a lower stock threshold is reached and restocked with `n` copies.
    /// A multiplicity of `1` and restock threshold of `0` corresponds to the common 7-Bag.
    Stock {
        /// The number of each  piece type left in the bag.
        pieces_left: [u32; 7],
        /// How many of each piece type to refill with.
        multiplicity: NonZeroU32,
        /// Bag threshold upon which to restock.
        restock_threshold: u32,
    },
    /// Recency/history-based piece generator.
    ///
    /// This generator keeps track of the last time each [`Tetromino`] type has been seen.
    /// It picks pieces by weighing them by this information as given by the `snap` field, which is
    /// used as the exponent of the last time the piece was seen. Note that this makes it impossible
    /// for a piece that was just played (index `0`) to be played again.
    Recency {
        /// The last time a piece was seen.
        ///
        /// `0` here denotes that it was the most recent piece generated.
        last_generated: [u32; 7],
        /// Determines how strongly it weighs pieces not generated in a while.
        ///
        ///
        snap: f64,
    },
    /// Experimental generator based off of how many times each [`Tetromino`] type has been seen
    /// *in total so far*.
    BalanceRelative {
        /// The relative number of times each piece type has been seen more/less than the others.
        ///
        /// Note that this is normalized, i.e. all entries are decremented simultaneously until
        /// at least one is `0`.
        relative_counts: [u32; 7],
    },
    /// Debug generator which repeats a certain pattern of [`Tetromino`]s forever.
    Cycle {
        /// The sequence of pieces that is repeated.
        pattern: Vec<Tetromino>,
        /// Index to the piece that will be yielded next.
        index: usize,
    },
}

impl TetrominoSource {
    /// Initialize an instance of the [`TetrominoSource::Uniform`] variant.
    pub const fn uniform() -> Self {
        Self::Uniform
    }

    /// Initialize a 7-Bag instance of the [`TetrominoSource::Stock`] variant.
    pub const fn bag() -> Self {
        Self::Stock {
            pieces_left: [1; 7],
            multiplicity: NonZeroU32::MIN,
            restock_threshold: 0,
        }
    }

    /// Initialize a custom instance of the [`TetrominoSource::Stock`] variant.
    pub const fn stock(multiplicity: NonZeroU32, refill_threshold: u32) -> Option<Self> {
        if refill_threshold < multiplicity.get() * 7 {
            Some(Self::Stock {
                pieces_left: [multiplicity.get(); 7],
                multiplicity,
                restock_threshold: refill_threshold,
            })
        } else {
            None
        }
    }

    /// Initialize a default instance of the [`TetrominoSource::Recency`] variant.
    pub const fn recency() -> Self {
        Self::recency_with(2.5)
    }

    /// Initialize a custom instance of the [`TetrominoSource::Recency`] variant.
    pub const fn recency_with(snap: f64) -> Self {
        Self::Recency {
            last_generated: [1; 7],
            snap,
        }
    }

    /// Initialize an instance of the [`TetrominoSource::BalanceRelative`] variant.
    pub const fn balance_relative() -> Self {
        Self::BalanceRelative {
            relative_counts: [0; 7],
        }
    }

    /// Initialize a custom instance of the [`TetrominoSource::Cycle`] variant.
    pub const fn cycle(pattern: Vec<Tetromino>) -> Self {
        Self::Cycle { pattern, index: 0 }
    }

    /// Method that allows `TetrominoSource` to be used as [`Iterator`].
    pub fn with_rng<'a, 'b, R: Rng>(&'a mut self, rng: &'b mut R) -> TetrominoIterator<'a, 'b, R> {
        TetrominoIterator {
            tetromino_generator: self,
            rng,
        }
    }
}

impl Clone for TetrominoSource {
    fn clone(&self) -> Self {
        match self {
            Self::Uniform => Self::uniform(),
            Self::Stock {
                pieces_left: _,
                multiplicity,
                restock_threshold,
            } => Self::stock(*multiplicity, *restock_threshold).unwrap(),
            Self::Recency {
                last_generated: _,
                snap,
            } => Self::recency_with(*snap),
            Self::BalanceRelative { relative_counts: _ } => Self::balance_relative(),
            Self::Cycle { pattern, index: _ } => Self::cycle(pattern.clone()),
        }
    }
}

/// Struct produced from [`TetrominoSource::with_rng`] which implements [`Iterator`].
pub struct TetrominoIterator<'a, 'b, R: Rng> {
    /// Selected tetromino generator to use as information source.
    pub tetromino_generator: &'a mut TetrominoSource,
    /// Thread random number generator for raw soure of randomness.
    pub rng: &'b mut R,
}

impl<'a, 'b, R: Rng> Iterator for TetrominoIterator<'a, 'b, R> {
    type Item = Tetromino;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.tetromino_generator {
            TetrominoSource::Uniform => Some(Tetromino::VARIANTS[self.rng.gen_range(0..=6)]),
            TetrominoSource::Stock {
                pieces_left,
                multiplicity,
                restock_threshold: refill_threshold,
            } => {
                let weights = pieces_left.iter();
                // SAFETY: Struct invariant.
                let idx = WeightedIndex::new(weights).unwrap().sample(&mut self.rng);
                // Update individual tetromino number and maybe replenish bag (ensuring invariant).
                pieces_left[idx] -= 1;
                if pieces_left.iter().sum::<u32>() == *refill_threshold {
                    for cnt in pieces_left {
                        *cnt += multiplicity.get();
                    }
                }
                // SAFETY: 0 <= idx <= 6.
                Some(Tetromino::VARIANTS[idx])
            }
            TetrominoSource::BalanceRelative { relative_counts } => {
                let weighing = |&x| 1.0 / f64::from(x).exp(); // Alternative weighing function: `1.0 / (f64::from(x) + 1.0);`
                let weights = relative_counts.iter().map(weighing);
                // SAFETY: `weights` will always be non-zero due to `weighing`.
                let idx = WeightedIndex::new(weights).unwrap().sample(&mut self.rng);
                // Update individual tetromino counter and maybe rebalance all relative counts
                relative_counts[idx] += 1;
                // SAFETY: `self.relative_counts` always has a minimum.
                let min = *relative_counts.iter().min().unwrap();
                if min > 0 {
                    for x in relative_counts.iter_mut() {
                        *x -= min;
                    }
                }
                // SAFETY: 0 <= idx <= 6.
                Some(Tetromino::VARIANTS[idx])
            }
            TetrominoSource::Recency {
                last_generated,
                snap,
            } => {
                let weighing = |&x| f64::from(x).powf(*snap);
                let weights = last_generated.iter().map(weighing);
                // SAFETY: `weights` will always be non-zero due to struct invarian.
                let idx = WeightedIndex::new(weights).unwrap().sample(&mut self.rng);
                // Update all tetromino last_played values and maybe rebalance all relative counts..
                for x in last_generated.iter_mut() {
                    *x += 1;
                }
                last_generated[idx] = 0;
                // SAFETY: 0 <= idx <= 6.
                Some(Tetromino::VARIANTS[idx])
            }
            TetrominoSource::Cycle { pattern, index } => {
                let tetromino = pattern[*index];
                *index += 1;
                if *index == pattern.len() {
                    *index = 0;
                }
                Some(tetromino)
            }
        }
    }
}
