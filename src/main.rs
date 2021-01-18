use std::{
    fmt::Display,
    fs, io,
    path::{Path, PathBuf},
    process,
};

mod error;
mod opts;

use colored::Colorize;
use directories::BaseDirs;
use either::Either;
use expr::{Expression, ExpressionParser, Highlight, RealizedExpression, Realizer};
use exprng::RandomRealizer;
use fs::File;
use hashbrown::HashMap;
use opts::{Mode, Opts};
use serde::{Deserialize, Serialize};

type Result<T, E = error::Error> = std::result::Result<T, E>;

static CONFIG: &str = ".roll";

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
            0 => Ok(()),
            x if x.is_negative() => write!(f, " - {}", x.abs()),
            x => write!(f, " + {}", x),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StoredExpression {
    text: String,
    expression: Expression,
}

impl StoredExpression {
    fn new(text: &str, expression: Expression) -> Self {
        Self {
            text: text.into(),
            expression,
        }
    }
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    match opts.mode() {
        Mode::Norm => execute_expressions(opts.candidates()),
        Mode::Add(alias) => add_alias(alias, opts.candidates()),
        Mode::Rem(alias) => rem_alias(alias),
        Mode::List => list(),
    }
}

fn execute_expressions<'a, I>(candidates: I) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let parser = ExpressionParser::new();
    let aliases = read_config(&get_config_path()?)?;
    let mut realizer = RandomRealizer::new();

    for expression in candidates {
        if let Some(stored_expressions) = aliases.get(expression) {
            for expression in stored_expressions.iter().map(|x| &x.expression) {
                let result = realizer.realize(expression);
                println!("{}", ResultFormatter::new(result));
            }
        } else {
            match parser.parse(expression.as_ref()) {
                Ok(expression) => {
                    let result = realizer.realize(&expression);
                    println!("{}", ResultFormatter::new(result));
                }

                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            }
        }
    }

    Ok(())
}

fn add_alias<'a, I>(alias: &str, candidates: I) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let parser = ExpressionParser::new();
    let expressions: expr::Result<Vec<StoredExpression>> = candidates
        .into_iter()
        .map(|x| {
            parser
                .parse(x.as_ref())
                .map(|expression| StoredExpression::new(x, expression))
        })
        .collect();
    let expressions = expressions?;

    let path = get_config_path()?;
    let mut aliases = read_config(&path)?;
    aliases.insert(alias.into(), expressions);
    write_config(&path, &aliases)?;
    Ok(())
}

fn rem_alias(alias: &str) -> Result<()> {
    let path = get_config_path()?;
    let mut aliases = read_config(&path)?;
    aliases.remove(alias);
    write_config(&path, &aliases)?;
    Ok(())
}

fn list() -> Result<()> {
    let aliases = read_config(&get_config_path()?)?;
    for (alias, expressions) in aliases {
        println!("> {}", alias);
        for expression in expressions {
            println!("  {}", expression.text);
        }
    }
    Ok(())
}

fn read_config(path: &Path) -> io::Result<HashMap<String, Vec<StoredExpression>>> {
    if !path.exists() {
        return Ok(Default::default());
    }

    let map = serde_json::from_reader(File::open(path)?)?;
    Ok(map)
}

fn write_config(path: &Path, aliases: &HashMap<String, Vec<StoredExpression>>) -> io::Result<()> {
    serde_json::to_writer_pretty(File::create(path)?, aliases)?;
    Ok(())
}

fn get_config_path() -> io::Result<PathBuf> {
    let dirs = BaseDirs::new()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No home directory"))?;
    Ok(dirs.home_dir().join(CONFIG))
}
