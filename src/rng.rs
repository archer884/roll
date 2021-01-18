use expr::RollSource;
use hashbrown::HashMap;
use rand::{
    distributions::{DistIter, Distribution, Uniform},
    rngs::ThreadRng,
};

#[derive(Default)]
pub struct RngSource {
    providers: HashMap<i32, BoundedRng>,
}

impl RngSource {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn next(&mut self, max: i32) -> i32 {
        self.providers
            .entry(max)
            .or_insert_with(|| BoundedRng::new(max))
            .next()
    }
}

impl RollSource for RngSource {
    fn next(&mut self, max: i32) -> i32 {
        RngSource::next(self, max)
    }
}

struct BoundedRng(DistIter<Uniform<i32>, ThreadRng, i32>);

impl BoundedRng {
    fn new(max: i32) -> Self {
        BoundedRng(Uniform::from(1..=max).sample_iter(rand::thread_rng()))
    }

    fn next(&mut self) -> i32 {
        self.0.next().unwrap()
    }
}

struct BoundedRngIter<'a>(&'a mut BoundedRng);

impl Iterator for BoundedRngIter<'_> {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.0.next())
    }
}
