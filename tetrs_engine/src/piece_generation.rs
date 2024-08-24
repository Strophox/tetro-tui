/*!
This module handles random generation of [`Tetromino`]s.
*/

use std::num::NonZeroU32;

use rand::{
    self,
    distributions::{Distribution, WeightedIndex},
    prelude::SliceRandom,
    rngs::ThreadRng,
    Rng,
};

use crate::Tetromino;

/// Handles the information of which pieces to spawn during a game.
///
/// To actually generate [`Tetromino`]s, the [`TetrominoGenerator::with_rng`] method needs to be used to yield a
/// [`TetrominoIterator`] that implements [`Iterator`].
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TetrominoSource {
    /// Uniformly random piece generator.
    Uniform,
    /// Standard 'bag' generator.
    ///
    /// It is not limited to '7-bag' and allows for other multiplicities `n` as well.
    /// It works by picking `n` copies of each [`Tetromino`] type, and then uniformly random
    /// handing them out until the bag is empty and is refilled again.
    Bag {
        /// The number of each  piece type left in the bag.
        pieces_left: [u32; 7],
        /// How many of each piece type to refill with.
        multiplicity: NonZeroU32,
    },
    /// Recency/history-based piece generator.
    ///
    /// This generator keeps track of the last time each [`Tetromino`] type has been seen.
    /// It picks pieces by weighing them by this information, such that it is impossible to choose a
    /// piece picked last time, and a bit more than quadratically (`x.powf(2.5)`) more likely to
    /// choose the index of another piece.
    Recency {
        /// The last time a piece was seen.
        ///
        /// `0` here denotes that it was the most recent piece generated.
        last_generated: [u32; 7],
    },
    /// Experimental generator based off of how many times each [`Tetromino`] type has been seen
    /// *in total so far*.
    TotalRelative {
        /// The relative number of times each piece type has been seen more/less than the others.
        ///
        /// Note that this is normalized, i.e. all entries are decremented simultaneously until
        /// at least one is `0`.
        relative_counts: [u32; 7],
    },
}

impl TetrominoSource {
    /// Initialize a new instance of the [`TetrominoGenerator::Uniform`] variant.
    pub fn uniform() -> Self {
        Self::Uniform
    }

    /// Initialize a new instance of the [`TetrominoGenerator::Bag`] variant with some multiplicity.
    pub fn bag(multiplicity: NonZeroU32) -> Self {
        Self::Bag {
            pieces_left: [multiplicity.get(); 7],
            multiplicity,
        }
    }

    /// Initialize a new instance of the [`TetrominoGenerator::Recency`] variant.
    pub fn recency() -> Self {
        let mut last_generated = [0, 1, 2, 3, 4, 5, 6];
        last_generated.shuffle(&mut rand::thread_rng());
        Self::Recency { last_generated }
    }

    /// Initialize a new instance of the [`TetrominoGenerator::TotalRelative`] variant.
    pub fn total_relative() -> Self {
        Self::TotalRelative {
            relative_counts: [0; 7],
        }
    }

    /// Method that allows `TetrominoGenerator` to be used as an [`Iterator`].
    pub fn with_rng<'a, 'b>(&'a mut self, rng: &'b mut ThreadRng) -> TetrominoIterator<'a, 'b> {
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
            Self::Bag { multiplicity, .. } => Self::bag(*multiplicity),
            Self::Recency { .. } => Self::recency(),
            Self::TotalRelative { .. } => Self::total_relative(),
        }
    }
}

/// Struct produced from [`TetrominoGenerator::with_rng`] which implements [`Iterator`].
pub struct TetrominoIterator<'a, 'b> {
    /// Selected tetromino generator to use as information source.
    pub tetromino_generator: &'a mut TetrominoSource,
    /// Thread random number generator for raw soure of randomness.
    pub rng: &'b mut ThreadRng,
}

impl<'a, 'b> Iterator for TetrominoIterator<'a, 'b> {
    type Item = Tetromino;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.tetromino_generator {
            TetrominoSource::Uniform => Some(self.rng.gen_range(0..=6).try_into().unwrap()),
            TetrominoSource::Bag {
                pieces_left,
                multiplicity,
            } => {
                let weights = pieces_left.iter().map(|&c| if c > 0 { 1 } else { 0 });
                // SAFETY: Struct invariant.
                let idx = WeightedIndex::new(weights).unwrap().sample(&mut self.rng);
                // Update individual tetromino number and maybe replenish bag (ensuring invariant).
                pieces_left[idx] -= 1;
                if pieces_left.iter().sum::<u32>() == 0 {
                    *pieces_left = [multiplicity.get(); 7];
                }
                // SAFETY: 0 <= idx <= 6.
                Some(idx.try_into().unwrap())
            }
            TetrominoSource::TotalRelative { relative_counts } => {
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
                Some(idx.try_into().unwrap())
            }
            TetrominoSource::Recency { last_generated } => {
                let weighing = |&x| f64::from(x).powf(2.5);
                let weights = last_generated.iter().map(weighing);
                // SAFETY: `weights` will always be non-zero due to `weighing`.
                let idx = WeightedIndex::new(weights).unwrap().sample(&mut self.rng);
                // Update all tetromino last_played values and maybe rebalance all relative counts..
                for x in last_generated.iter_mut() {
                    *x += 1;
                }
                last_generated[idx] = 0;
                // SAFETY: 0 <= idx <= 6.
                Some(idx.try_into().unwrap())
            }
        }
    }
}
