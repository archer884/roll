mod args;
mod history;

use std::{borrow::Cow, fs, io, iter, path::Path, slice};

use args::{AddAlias, Args, Mode, PathConfig};
use comfy_table::Table;
use expr::{Expression, ExpressionParser};
use exprng::{RandomRealizer, Realizer};
use fs::File;
use hashbrown::{HashMap, HashSet};
use history::History;
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

fn main() {
    let args = Args::parse();

    if let Err(e) = run(&args) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run(args: &Args) -> Result<()> {
    let paths = args.path_config()?;

    match args.mode() {
        Mode::Norm => execute_expressions(&paths, args),
        Mode::Average => print_averages(&paths, args.candidates()),
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
    let parser = ExpressionParser::new();
    let aliases = read_config(path.config())?;
    let mut unique_filter = HashSet::new();
    let mut table = configure_table();

    for expression in expand_expressions(candidates) {
        if let Some(formula) = aliases.get(expression) {
            for expression in formula.expressions.iter() {
                if !unique_filter.contains(&expression.text) {
                    let average = expression.expression.average_result();
                    table.add_row(&[Cow::from(&expression.text), format!("{average:.02}").into()]);
                    unique_filter.insert(expression.text.clone());
                }
            }
        } else if !unique_filter.contains(expression) {
            let compiled = parser.parse(expression)?;
            let average = compiled.average_result();
            table.add_row(&[Cow::from(expression), format!("{average:.02}").into()]);
        }
    }

    println!("{table}");

    Ok(())
}

fn execute_expressions(paths: &PathConfig, args: &Args) -> Result<()> {
    let parser = ExpressionParser::new();
    let aliases = read_config(paths.config())?;

    let mut realizer: RandomRealizer<SquirrelRng> = RandomRealizer::new();
    let mut realizer = realizer.with_logging();
    let mut history = History::new(paths.history());
    let mut table = configure_table();

    // FIXME: Add verbose mode that prints expressions, but don't bother
    // printing expressions under normal circumstances.

    for expression in expand_expressions(args.candidates()) {
        if let Some(formula) = aliases.get(expression) {
            table.add_row([expression, formula.comment.as_deref().unwrap_or("")]);

            for expression in &formula.expressions {
                let result = realizer.realize(&expression.expression);
                table.add_row(&[
                    Cow::from(result.sum().to_string()),
                    Cow::from(&expression.text),
                ]);
            }
        } else {
            let compiled = parser.parse(expression)?;
            let result = realizer.realize(&compiled);

            if args.verbose {
                table.add_row(&[Cow::from(result.sum().to_string()), Cow::from(expression)]);
            } else {
                table.add_row(&[Cow::from(result.sum().to_string()), Cow::from("")]);
            }
        }
    }

    table
        .column_mut(0)
        .expect("table has two columns")
        .set_cell_alignment(comfy_table::CellAlignment::Right);

    println!("{table}");
    history.append_log(realizer.finalize());
    Ok(history.write()?)
}

fn configure_table() -> Table {
    let mut table = Table::new();
    table.load_preset(comfy_table::presets::NOTHING);
    table
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
