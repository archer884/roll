mod args;
mod history;

use std::{fmt::Display, fs, io, iter, path::Path, process, slice};

use args::{AddAlias, Args, Mode, PathConfig};
use expr::{Expression, ExpressionParser, Highlight, RealizedExpression};
use exprng::{Realizer, RandomRealizer};
use fs::File;
use hashbrown::{HashMap, HashSet};
use history::History;
use owo_colors::OwoColorize;
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
        match self.result {
            result if result.is_critical() => {
                write!(
                    f,
                    "{:>2}  ::  {}  ::  ",
                    result.sum().bright_green(),
                    self.text
                )?;
            }
            result if result.sum() == 1 => {
                write!(
                    f,
                    "{:>2}  ::  {}  ::  ",
                    result.sum().bright_red(),
                    self.text
                )?;
            }
            result => {
                write!(f, "{:>2}  ::  {}  ::  ", result.sum(), self.text)?;
            }
        }

        // Print rolled values with highlighting
        let mut results = self.result.results();

        if let Some((highlight, value)) = results.by_ref().next() {
            match highlight {
                Highlight::High => {
                    write!(f, "{:>2}", value.bright_green())?;
                }
                Highlight::Low => {
                    write!(f, "{:>2}", value.bright_red())?;
                }
                Highlight::Normal => {
                    write!(f, "{:>2}", value)?;
                }
            }
        }

        for (highlight, value) in results {
            write_with_highlight(f, value, highlight)?;
        }

        // Print static modifier
        match self.result.modifier() {
            0 => Ok(()),
            x if x.is_negative() => write!(f, " (-{})", x.abs()),
            x => write!(f, "   +{}", x),
        }
    }
}

#[inline(always)]
fn write_with_highlight(
    f: &mut std::fmt::Formatter,
    value: i32,
    highlight: Highlight,
) -> std::fmt::Result {
    match highlight {
        Highlight::High => {
            write!(f, ", {:>2}", value.bright_green())
        }
        Highlight::Low => {
            write!(f, ", {:>2}", value.bright_red())
        }
        Highlight::Normal => {
            write!(f, ", {:>2}", value)
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
    let opts = Args::parse();
    let paths = opts.path_config()?;

    match opts.mode() {
        Mode::Norm => execute_expressions(&paths, opts.candidates()),
        Mode::Average => print_averages(&paths, opts.candidates()),
        Mode::Add(alias) => add_alias(alias, paths.config()),
        Mode::Rem(alias) => rem_alias(alias, paths.config()),
        Mode::List => list(paths.config()),
    }
}

/// Expands "counted" expressions
///
/// An expression of the form 2d6*2 expands *two instances of* the expression 2d6. (Note that on
/// Linux systems it is necessary to either quote '2d6*2' or use 2d6x2 instead.) This function
/// transforms a counted expression into one or more expressions of the same value.
fn expand_expressions<'a, I>(candidates: I) -> impl Iterator<Item = &'a str>
where
    I: IntoIterator<Item = &'a str>,
{
    candidates
        .into_iter()
        .map(
            |candidate| match candidate.split_once(|u: char| u == '*' || u == 'x' || u == 'X') {
                Some((expr, count)) => (count.parse().unwrap_or(1usize), expr),
                None => (1, candidate),
            },
        )
        .flat_map(|(count, expr)| iter::repeat(expr).take(count))
}

fn print_averages<'a, I>(path: &PathConfig, candidates: I) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut unique_filter = HashSet::new();
    let aliases = read_config(path.config())?;

    println!();

    for expression in expand_expressions(candidates) {
        if let Some(formula) = aliases.get(expression) {
            for expression in formula.expressions.iter().map(|x| &x.expression) {
                if !unique_filter.contains(expression) {
                    let average = expression.average_result();
                    println!("{average:.02}");
                    unique_filter.insert(expression.clone());
                }
            }
        } else {

        }
    }

    println!();

    Ok(())
}

fn execute_expressions<'a, I>(paths: &PathConfig, candidates: I) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let parser = ExpressionParser::new();
    let aliases = read_config(paths.config())?;

    let mut realizer: RandomRealizer<SquirrelRng> = RandomRealizer::new();
    let mut realizer = realizer.with_logging();
    let mut history = History::new(paths.history());

    println!();

    for expression in expand_expressions(candidates) {
        if let Some(formula) = aliases.get(expression) {
            println!("# {}", expression);
            if let Some(comment) = &formula.comment {
                println!("# {}", comment);
            }
            for expression in &formula.expressions {
                let result = realizer.realize(&expression.expression);
                println!("  {}", ResultFormatter::new(&expression.text, &result));
            }
        } else {
            match parser.parse(expression.as_ref()) {
                Ok(compiled) => {
                    let result = realizer.realize(&compiled);
                    println!("  {}", ResultFormatter::new(expression, &result));
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
    write_config(config, &aliases)?;
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
