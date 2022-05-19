pub use expr::Realizer;
use hashbrown::HashMap;
use rand::{
    distributions::{DistIter, Uniform},
    prelude::Distribution,
    Rng,
};
use smallvec::SmallVec;

pub trait InitializeBoundedRng: Rng + Sized {
    fn initialize(max: i32) -> BoundedRng<Self>;
}

impl<R: Default + Rng> InitializeBoundedRng for R {
    fn initialize(max: i32) -> BoundedRng<Self> {
        BoundedRng(Uniform::from(1..=max).sample_iter(R::default()))
    }
}

#[derive(Debug, Default)]
pub struct RandomRealizer<R> {
    source: HashMap<i32, BoundedRng<R>>,
}

impl<I: InitializeBoundedRng> RandomRealizer<I> {
    pub fn new() -> Self {
        Self {
            source: HashMap::new(),
        }
    }

    pub fn with_logging(&mut self) -> LogWrapper<Self> {
        LogWrapper {
            realizer: self,
            log: HashMap::new(),
        }
    }
}

impl<I: InitializeBoundedRng> Realizer for RandomRealizer<I> {
    fn next(&mut self, max: i32) -> i32 {
        self.source
            .entry(max)
            .or_insert_with(|| I::initialize(max))
            .next()
    }
}

pub struct LogWrapper<'r, R> {
    realizer: &'r mut R,
    log: HashMap<i32, SmallVec<[i32; 4]>>,
}

impl<R> LogWrapper<'_, R> {
    pub fn finalize(self) -> HashMap<i32, SmallVec<[i32; 4]>> {
        self.log
    }
}

impl<'r, R: Realizer> Realizer for LogWrapper<'r, R> {
    fn next(&mut self, max: i32) -> i32 {
        let result = self.realizer.next(max);
        self.log.entry(max).or_default().push(result);
        result
    }
}

#[derive(Debug)]
pub struct BoundedRng<R>(DistIter<Uniform<i32>, R, i32>);

impl<R: Rng> BoundedRng<R> {
    fn next(&mut self) -> i32 {
        self.0.next().unwrap()
    }
}
