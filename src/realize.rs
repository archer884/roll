use hashbrown::HashMap;
use smallvec::SmallVec;
use squirrel_rng::SquirrelRng;

pub use crate::expression::Realizer;

#[derive(Debug, Default)]
pub struct RandomRealizer {
    source: SquirrelRng,
}

impl RandomRealizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_logging(&'_ mut self) -> LogWrapper<'_, Self> {
        LogWrapper {
            realizer: self,
            log: HashMap::new(),
        }
    }
}

impl Realizer for RandomRealizer {
    fn next(&mut self, max: i32) -> i32 {
        assert!(max > 0, "die size must be positive");
        self.source.gen_range(1..(max as u32 + 1)) as i32
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
