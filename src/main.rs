mod rng;

use std::{fmt::Display, process};

use clap::{crate_authors, crate_version, Clap};
use colored::Colorize;
use either::Either;
use expr::{ExpressionParser, Highlight, RealizedExpression};
use rng::RngSource;

/// A dice roller.
///
/// Use expressions like 2d8+d6+5. Results are printed with the total first,
/// followed by each individual roll. Max rolls (crits) are highlighted in green,
/// while low rolls are highlighted in red.
#[derive(Clap, Clone, Debug)]
#[clap(author = crate_authors!(), version = crate_version!())]
struct Opts {
    candidate_expressions: Vec<String>,
}

struct ResultFormatter {
    realized: RealizedExpression,
}

impl ResultFormatter {
    fn new(result: RealizedExpression) -> Self {
        Self { realized: result }
    }
}

impl Display for ResultFormatter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Print sum
        write!(f, "{} ::", self.realized.sum())?;

        // Print rolled values with highlighting
        let mut results = self.realized.results();

        if let Some((highlight, value)) = results.by_ref().next() {
            let item = match highlight {
                Highlight::High => Either::Left(value.to_string().bright_green()),
                Highlight::Low => Either::Left(value.to_string().bright_red()),
                _ => Either::Right(value),
            };
            write!(f, " {}", item)?;
        }
        
        for (highlight, value) in results {
            let item = match highlight {
                Highlight::High => Either::Left(value.to_string().bright_green()),
                Highlight::Low => Either::Left(value.to_string().bright_red()),
                _ => Either::Right(value),
            };
            write!(f, " + {}", item)?;
        }

        // Print static modifier
        match self.realized.modifier() {
            x if x.is_negative() => write!(f, " - {}", x.abs()),
            x => write!(f, " + {}", x),
        }
    }
}

fn main() {
    let Opts {
        candidate_expressions,
    } = Opts::parse();

    let parser = ExpressionParser::new();
    let mut source = RngSource::new();

    for expression in candidate_expressions {
        match parser.parse(&expression) {
            Ok(expression) => {
                let result = expression.realize(&mut source);
                println!("{}", ResultFormatter::new(result));
            }

            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    }
}
