mod bounded_rng;
mod error;
mod expression;

use bounded_rng::BoundedRngProvider;
use clap::{crate_authors, crate_description, crate_version, Clap};
use error::Error;
use expression::Expression;

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clap, Clone, Debug)]
#[clap(author = crate_authors!(), version = crate_version!(), about = crate_description!())]
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
