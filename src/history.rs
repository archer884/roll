use std::{io, path::Path};

use expr::{Expression, RealizedExpression};
use hashbrown::HashMap;

#[derive(Clone, Debug)]
pub struct History<'a> {
    path: &'a Path,
    history: HashMap<i32, Vec<i32>>,
}

impl<'a> History<'a> {
    pub fn new(path: &'a Path) -> Self {
        Self {
            path,
            history: HashMap::new(),
        }
    }

    pub fn insert(&mut self, expression: &Expression, realization: &RealizedExpression) {
        self.history
            .entry(expression.num_sides())
            .or_default()
            .extend(realization.results().map(|x| x.1));
    }

    pub fn write(mut self) -> io::Result<()> {
        todo!()
    }
}
