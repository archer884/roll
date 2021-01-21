use std::{fmt::Display, fs, io, path::Path, process, slice};

mod error;
mod opts;

use colored::Colorize;
use either::Either;
use expr::{Expression, ExpressionParser, Highlight, RealizedExpression, Realizer};
use exprng::RandomRealizer;
use fs::File;
use hashbrown::HashMap;
use opts::{AddAlias, Mode, Opts};
use regex::Regex;
use serde::{Deserialize, Serialize};

type Result<T, E = error::Error> = std::result::Result<T, E>;

struct ResultFormatter<'a> {
    text: &'a str,
    realized: RealizedExpression,
}

impl<'a> ResultFormatter<'a> {
    fn new(text: &'a str, result: RealizedExpression) -> Self {
        Self {
            text,
            realized: result,
        }
    }
}

impl<'a> Display for ResultFormatter<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Print sum
        write!(f, "{:>2}  ::  {}  ::  ", self.realized.sum(), self.text)?;

        // Print rolled values with highlighting
        let mut results = self.realized.results();

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
        match self.realized.modifier() {
            0 => Ok(()),
            x if x.is_negative() => write!(f, " (-{})", x.abs()),
            x => write!(f, "  (+{})", x),
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
    let config_path = opts.config_path()?;

    match opts.mode() {
        Mode::Norm => execute_expressions(&config_path, opts.candidates()),
        Mode::Add(alias) => add_alias(alias, &config_path),
        Mode::Rem(alias) => rem_alias(alias, &config_path),
        Mode::List => list(&config_path),
    }
}

fn execute_expressions<'a, I>(config: &Path, candidates: I) -> Result<()>
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
    let aliases = read_config(config)?;
    let pattern = Regex::new(r#"\*(\d+)$"#).unwrap();
    let counted_expressions = candidates
        .into_iter()
        .map(|expr| count_expression(expr, &pattern));

    let mut realizer = RandomRealizer::new();

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
                    println!("  {}", ResultFormatter::new(&expression.text, result));
                }
            }
        } else {
            match parser.parse(expression.as_ref()) {
                Ok(compiled) => {
                    for _ in 0..count {
                        let result = realizer.realize(&compiled);
                        println!("{}", ResultFormatter::new(expression, result));
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
    Ok(())
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
