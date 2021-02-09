use std::{fmt::Display, fs, io, path::Path, process, slice};

mod history;
mod opts;

use colored::{ColoredString, Colorize};
use either::Either;
use expr::{Expression, ExpressionParser, Highlight, RealizedExpression, Realizer};
use exprng::RandomRealizer;
use fs::File;
use hashbrown::HashMap;
use history::History;
use opts::{AddAlias, Mode, Opts, PathConfig};
use regex::Regex;
use serde::{Deserialize, Serialize};
use squirrel_rng::SquirrelRng;

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unable to parse expression: {0}")]
    Expr(#[from] expr::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
}

struct ResultFormatter<'a> {
    text: &'a str,
    result: &'a RealizedExpression,
}

impl<'a> ResultFormatter<'a> {
    fn new(text: &'a str, result: &'a RealizedExpression) -> Self {
        Self { text, result }
    }
}

impl<'a> Display for ResultFormatter<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Print sum
        let sum = match self.result {
            result if result.is_critical() => Either::Left(result.sum().to_string().bright_green()),
            result if result.sum() == 1 => Either::Left(result.sum().to_string().bright_red()),
            result => Either::Right(result.sum()),
        };
        write!(f, "{:>2}  ::  {}  ::  ", sum, self.text)?;

        // Print rolled values with highlighting
        let mut results = self.result.results();

        if let Some((highlight, value)) = results.by_ref().next() {
            let item = match highlight {
                Highlight::High => Either::Left(value.to_string().bright_green()),
                Highlight::Low => Either::Left(value.to_string().bright_red()),
                _ => Either::Right(value),
            };
            write!(f, "{:>2}", item)?;
        }

        for (highlight, value) in results {
            let item = match highlight {
                Highlight::High => Either::Left(value.to_string().bright_green()),
                Highlight::Low => Either::Left(value.to_string().bright_red()),
                _ => Either::Right(value),
            };
            write!(f, "  {:>2}", item)?;
        }

        // Print static modifier
        match self.result.modifier() {
            0 => Ok(()),
            x if x.is_negative() => write!(f, " (-{})", x.abs()),
            x => write!(f, "   +{}", x),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Formula {
    comment: Option<String>,
    expressions: Vec<StoredExpression>,
}

impl<'a> IntoIterator for &'a Formula {
    type Item = &'a StoredExpression;

    type IntoIter = slice::Iter<'a, StoredExpression>;

    fn into_iter(self) -> Self::IntoIter {
        self.expressions.iter()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StoredExpression {
    text: String,
    expression: Expression,
}

impl StoredExpression {
    fn new(text: impl Into<String>, expression: Expression) -> Self {
        Self {
            text: text.into(),
            expression,
        }
    }
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    let paths = opts.path_config()?;

    match opts.mode() {
        Mode::Norm(show_average) => execute_expressions(&paths, opts.candidates(), show_average),
        Mode::Add(alias) => add_alias(alias, paths.config()),
        Mode::Rem(alias) => rem_alias(alias, paths.config()),
        Mode::List => list(paths.config()),
    }
}

fn execute_expressions<'a, I>(paths: &PathConfig, candidates: I, show_average: bool) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    /// Extracts a count suffix from the string, returning the suffix parsed as
    /// an integer and the non-suffixed string.
    fn count_expression<'a>(expr: &'a str, pattern: &Regex) -> (usize, &'a str) {
        match pattern.captures(expr) {
            Some(suffix) => {
                let expr = &expr[..suffix.get(0).unwrap().start()];
                let count = suffix.get(1).unwrap().as_str().parse().unwrap_or(1);
                (count, expr)
            }
            None => (1, expr),
        }
    }

    let parser = ExpressionParser::new();
    let aliases = read_config(paths.config())?;
    let pattern = Regex::new(r#"\*(\d+)$"#).unwrap();
    let counted_expressions = candidates
        .into_iter()
        .map(|expr| count_expression(expr, &pattern));

    let mut realizer: RandomRealizer<SquirrelRng> = RandomRealizer::new();
    let mut realizer = realizer.with_logging();
    let mut history = History::new(paths.history());

    println!();
    for (count, expression) in counted_expressions {
        if let Some(formula) = aliases.get(expression) {
            for _ in 0..count {
                println!("# {}", expression);
                if let Some(comment) = &formula.comment {
                    println!("# {}", comment);
                }
                for expression in &formula.expressions {
                    let result = realizer.realize(&expression.expression);
                    if show_average {
                        println!(
                            "  {}   {:>4}",
                            ResultFormatter::new(&expression.text, &result),
                            compare_to_average(
                                result.sum(),
                                expression.expression.average_result()
                            )
                        );
                    } else {
                        println!("  {}", ResultFormatter::new(&expression.text, &result));
                    }
                }
            }
        } else {
            match parser.parse(expression.as_ref()) {
                Ok(compiled) => {
                    for _ in 0..count {
                        let result = realizer.realize(&compiled);
                        if show_average {
                            println!(
                                "  {}   {:>4}",
                                ResultFormatter::new(expression, &result),
                                compare_to_average(result.sum(), compiled.average_result())
                            );
                        } else {
                            println!("  {}", ResultFormatter::new(expression, &result));
                        }
                    }
                }

                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            }
        }
    }

    println!();
    history.append_log(realizer.finalize());
    Ok(history.write()?)
}

fn add_alias(add: &AddAlias, config: &Path) -> Result<()> {
    let parser = ExpressionParser::new();
    let expressions: expr::Result<Vec<StoredExpression>> = add
        .candidate_expressions
        .iter()
        .map(|text| {
            parser
                .parse(text.as_ref())
                .map(|expression| StoredExpression::new(text, expression))
        })
        .collect();

    let mut aliases = read_config(config)?;
    aliases.insert(
        add.alias.clone(),
        Formula {
            comment: add.comment.clone(),
            expressions: expressions?,
        },
    );
    write_config(config, &aliases)?;
    Ok(())
}

fn rem_alias(alias: &str, config: &Path) -> Result<()> {
    let mut aliases = read_config(config)?;
    aliases.remove(alias);
    write_config(&config, &aliases)?;
    Ok(())
}

fn list(config: &Path) -> Result<()> {
    let aliases = read_config(config)?;
    for (alias, formula) in aliases {
        println!("# {}", alias);
        if let Some(comment) = &formula.comment {
            println!("# {}", comment);
        }
        for expression in &formula {
            println!("  {}", expression.text);
        }
    }
    Ok(())
}

fn compare_to_average(realized: i32, average: f64) -> Either<String, ColoredString> {
    match realized as f64 / average * 100.0 {
        n if n >= 150.0 => Either::Right(format!("{:.0}%", n).bright_green()),
        n if n <= 50.0 => Either::Right(format!("{:.0}%", n).bright_red()),
        n => Either::Left(format!("{:.0}%", n)),
    }
}

fn read_config(path: &Path) -> io::Result<HashMap<String, Formula>> {
    if !path.exists() {
        return Ok(Default::default());
    }

    let map = serde_json::from_reader(File::open(path)?)?;
    Ok(map)
}

fn write_config(path: &Path, aliases: &HashMap<String, Formula>) -> io::Result<()> {
    serde_json::to_writer_pretty(File::create(path)?, aliases)?;
    Ok(())
}
