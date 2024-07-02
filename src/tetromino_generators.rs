use crate::game_logic::Tetromino;

use rand::{
    self,
    distributions::{Distribution, Uniform, WeightedIndex},
    rngs::ThreadRng
};

// Uniformly random tetromino generation.
pub struct RandomGen {
    rng: ThreadRng,
    uniform: Uniform<usize>,
}

impl RandomGen {
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
        // Some(rand::thread_rng().gen_range(0..=6).try_into().unwrap()) // Safety: 0 <= n <= 6
        Some(self.uniform.sample(&mut self.rng).try_into().unwrap()) // Safety: 0 <= n <= 6
    }
}

// Bag-system for tetromino generation.
// All 7 tetrominos are put in a bag, shuffled, and handed out; repeat if empty.
// The bag multiplicity says how many copies of all 7 tetrominos are put in.
pub struct BagGen {
    // Invariants: self.leftover.iter().sum::<u32>() > 0
    rng: ThreadRng,
    leftover: [u32; 7],
    bag_multiplicity: u32,
}

impl BagGen {
    pub fn new(n: u32) -> Self {
        assert!(n != 0, "bag multiplicity must be > 0");
        BagGen {
            rng: rand::thread_rng(),
            leftover: [n; 7],
            bag_multiplicity: n,
        }
    }
}

impl Iterator for BagGen {
    type Item = Tetromino;

    fn next(&mut self) -> Option<Self::Item> {
        let weights = self.leftover.iter().map(|&c| if c > 0 {1} else {0});
        let i = WeightedIndex::new(weights).unwrap().sample(&mut self.rng); // Safety: (yes)
        // Adapt individual tetromino number and maybe replenish bag
        self.leftover[i] -= 1;
        if self.leftover.iter().sum::<u32>() == 0 {
            self.leftover = [self.bag_multiplicity; 7];
        }
        Some(i.try_into().unwrap()) // Safety: 0 <= n <= 6
    }
}

// A probabilistic generator that weighs the probabilities by
// how often a tetromino has appeared compared to the others. 
pub struct TotalRelativeProbGen {
    rng: ThreadRng,
    relative_counts: [u32; 7],
}

impl TotalRelativeProbGen {
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
        let weight = |&x| 1.0 / f64::from(x).exp(); // x -> 1 / exp x
        // let weight = |&x| 1.0 / (f64::from(x) + 1.0); // x -> 1 / (1 + x)
        let weights = self.relative_counts.iter().map(weight);
        let i = WeightedIndex::new(weights).unwrap().sample(&mut self.rng);
        // Adapt individual tetromino counter and maybe rebalance all relative counts
        self.relative_counts[i] += 1;
        let min = *self.relative_counts.iter().min().unwrap(); // Safety: minimum always exists
        if min > 0 {
            for x in self.relative_counts.iter_mut() {
                *x -= min;
            }
        }
        Some(i.try_into().unwrap()) // Safety: 0 <= n <= 6
    }
}

// A probabilistic generator that weighs the probabilities by
// how recently a tetromino has appeared. 
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
        // let weight = |x| x; // x -> x
        // let weight = |&x| f64::from(x).powf(1.5); // x -> x^1.5
        let weight = |x| x*x; // x -> x^2
        // let weight = |&x| f64::from(x).exp() - 1.0; // x -> exp x - 1
        let weights = self.last_played.iter().map(weight);
        let i = WeightedIndex::new(weights).unwrap().sample(&mut self.rng);
        // Adapt all tetromino last_played values and maybe rebalance all relative counts
        for x in self.last_played.iter_mut() {
            *x += 1;
        }
        self.last_played[i] = 0;
        Some(i.try_into().unwrap()) // Safety: 0 <= n <= 6
    }
}