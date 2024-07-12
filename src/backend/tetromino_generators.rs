use std::num::NonZeroU32;

use rand::{
    self,
    distributions::{Distribution, Uniform, WeightedIndex},
    rngs::ThreadRng,
};

use crate::backend::game::Tetromino;

#[derive(Debug)]
pub struct RandomGen {
    rng: ThreadRng,
    uniform: Uniform<usize>,
}

impl RandomGen {
    #[allow(dead_code)]
    pub fn new() -> Self {
        RandomGen {
            rng: rand::thread_rng(),
            uniform: Uniform::from(0..=6),
        }
    }
}

impl Iterator for RandomGen {
    type Item = Tetromino;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: 0 <= uniform_sample <= 6.
        Some(self.uniform.sample(&mut self.rng).try_into().unwrap()) // Alternative random gen: `Some(rand::thread_rng().gen_range(0..=6).try_into().unwrap())`.
    }
}

#[derive(Debug)]
pub struct BagGen {
    // INVARIANT: self.leftover.iter().sum::<u32>() > 0.
    rng: ThreadRng,
    leftover: [u32; 7],
    bag_multiplicity: u32,
}

impl BagGen {
    #[allow(dead_code)]
    pub fn new(n: NonZeroU32) -> Self {
        BagGen {
            rng: rand::thread_rng(),
            leftover: [n.get(); 7],
            bag_multiplicity: n.get(),
        }
    }
}

impl Iterator for BagGen {
    type Item = Tetromino;

    fn next(&mut self) -> Option<Self::Item> {
        let weights = self.leftover.iter().map(|&c| if c > 0 { 1 } else { 0 });
        // SAFETY: Struct invariant.
        let idx = WeightedIndex::new(weights).unwrap().sample(&mut self.rng);
        // Update individual tetromino number and maybe replenish bag (ensuring invariant).
        self.leftover[idx] -= 1;
        if self.leftover.iter().sum::<u32>() == 0 {
            self.leftover = [self.bag_multiplicity; 7];
        }
        // SAFETY: 0 <= idx <= 6.
        Some(idx.try_into().unwrap())
    }
}

#[derive(Debug)]
pub struct TotalRelativeProbGen {
    rng: ThreadRng,
    relative_counts: [u32; 7],
}

impl TotalRelativeProbGen {
    #[allow(dead_code)]
    pub fn new() -> Self {
        TotalRelativeProbGen {
            rng: rand::thread_rng(),
            relative_counts: [0; 7],
        }
    }
}

impl Iterator for TotalRelativeProbGen {
    type Item = Tetromino;

    fn next(&mut self) -> Option<Self::Item> {
        let weighing = |&x| 1.0 / f64::from(x).exp(); // Alternative weighing function: `1.0 / (f64::from(x) + 1.0);`
        let weights = self.relative_counts.iter().map(weighing);
        // SAFETY: `weights` will always be non-zero due to `weighing`.
        let idx = WeightedIndex::new(weights).unwrap().sample(&mut self.rng);
        // Update individual tetromino counter and maybe rebalance all relative counts
        self.relative_counts[idx] += 1;
        // SAFETY: `self.relative_counts` always has a minimum.
        let min = *self.relative_counts.iter().min().unwrap();
        if min > 0 {
            for x in self.relative_counts.iter_mut() {
                *x -= min;
            }
        }
        // SAFETY: 0 <= idx <= 6.
        Some(idx.try_into().unwrap())
    }
}

#[derive(Debug)]
pub struct RecencyProbGen {
    rng: ThreadRng,
    last_played: [u32; 7],
}

impl RecencyProbGen {
    pub fn new() -> Self {
        RecencyProbGen {
            rng: rand::thread_rng(),
            last_played: [1; 7],
        }
    }
}

impl Iterator for RecencyProbGen {
    type Item = Tetromino;

    fn next(&mut self) -> Option<Self::Item> {
        /* Choice among these weighing functions:
         * `|x| x; // x -> x`
         * `|&x| f64::from(x).powf(1.5); // x -> x^1.5`
         * `|x| x * x; // x -> x^2`
         * `|&x| f64::from(x).exp() - 1.0; // x -> exp x - 1`
         */
        let weighing = |x| x * x;
        let weights = self.last_played.iter().map(weighing);
        // SAFETY: `weights` will always be non-zero due to `weighing`.
        let idx = WeightedIndex::new(weights).unwrap().sample(&mut self.rng);
        // Update all tetromino last_played values and maybe rebalance all relative counts..
        for x in self.last_played.iter_mut() {
            *x += 1;
        }
        self.last_played[idx] = 0;
        // SAFETY: 0 <= idx <= 6.
        Some(idx.try_into().unwrap())
    }
}

// TODO: Program [NESRandomGen](https://meatfighter.com/nintendotetrisai/#Spawning_Tetriminos).
