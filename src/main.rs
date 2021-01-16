mod bounded_rng;
mod error;
mod expression;

use bounded_rng::BoundedRngProvider;
use clap::{crate_authors, crate_version, Clap};
use error::Error;
use expression::Expression;

type Result<T, E = Error> = std::result::Result<T, E>;

/// A dice roller.
///
/// Use expressions like 2d8+d6+5. Results are printed with the total first,
/// followed by each individual roll. Max rolls (crits) are highlighted in green,
/// while low rolls are highlighted in red.
#[derive(Clap, Clone, Debug)]
#[clap(author = crate_authors!(), version = crate_version!())]
struct Opts {
    expressions: Vec<Expression>,
}

fn main() {
    let Opts { expressions } = Opts::parse();
    let mut provider = BoundedRngProvider::new();

    for exp in expressions {
        println!("{}", exp.realize(&mut provider));
    }
}
