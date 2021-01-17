mod rng;

use std::fmt::Display;

use clap::{crate_authors, crate_version, Clap};
use colored::{ColoredString, Colorize};
use expr::{CompoundExpression, RealizedCompoundExpression, RealizedExpression};
use rng::BoundedRngProvider;

/// A dice roller.
///
/// Use expressions like 2d8+d6+5. Results are printed with the total first,
/// followed by each individual roll. Max rolls (crits) are highlighted in green,
/// while low rolls are highlighted in red.
#[derive(Clap, Clone, Debug)]
#[clap(author = crate_authors!(), version = crate_version!())]
struct Opts {
    expressions: Vec<CompoundExpression>,
}

struct ResultFormatter {
    realized: RealizedCompoundExpression,
}

impl ResultFormatter {
    fn new(result: RealizedCompoundExpression) -> Self {
        Self { realized: result }
    }
}

impl Display for ResultFormatter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn apply_highlight(result: &RealizedExpression) -> ColoredString {
            // If this is a constant modifier, it doesn't get highlighted.
            if result.max == result.min {
                let formatted = result.realized.to_string();
                return (&*formatted).clear();
            }

            match result.realized {
                x if x == result.max => x.to_string().bright_green(),
                x if x == result.min => x.to_string().bright_red(),
                _ => {
                    let formatted = result.realized.to_string();
                    (&*formatted).clear()
                }
            }
        }

        let sum = self.realized.sum();
        let mut results = self.realized.results();

        if let Some(result) = results.by_ref().next() {
            write!(f, "{} :: {}", sum, apply_highlight(result))?;
        } else {
            return f.write_str("Empty result");
        }

        for result in results {
            write!(f, " {}", apply_highlight(result))?;
        }

        Ok(())
    }
}

fn main() {
    let Opts { expressions } = Opts::parse();
    let mut provider = BoundedRngProvider::new();

    for exp in expressions {
        println!("{}", ResultFormatter::new(exp.realize(&mut provider)));
    }
}
