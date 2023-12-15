use std::{
    borrow::Cow,
    io, iter,
    path::{Path, PathBuf},
};

use clap::Parser;
use directories::BaseDirs;
use either::Either;

use crate::Result;

#[derive(Clone, Debug, Parser)]
#[clap(author, about, version)]
pub struct Args {
    /// roll expressions
    ///
    /// Expressions should generally be of the form 2d6. Shorthand like 20 is valid for any
    /// expression 1d20, etc. Syntax extensions include:
    /// - 2d6r / 2d6r2: reroll 1s or 2s
    /// - 2d6!: explode (roll again and add to total) on max values
    /// - 2d6+2: add 2 to total
    expressions: Vec<String>,

    /// print average value of expressions
    ///
    /// In this mode, roll does not actually roll but prints the calculated average value of an
    /// expression.
    #[clap(short = 'a', long = "show-average")]
    show_average: bool,

    /// modify configuration
    ///
    /// Config for default and alternate profiles. (Pass profile name to modify named profile.)
    #[clap(short, long)]
    config: Option<String>,

    #[clap(subcommand)]
    subcmd: Option<SubCommand>,

    #[clap(short, long)]
    pub verbose: bool,
}

impl Args {
    pub fn parse() -> Self {
        Parser::parse()
    }

    pub fn candidates(&self) -> impl Iterator<Item = &str> {
        match self.subcmd {
            None => Either::Left(self.expressions.iter().map(AsRef::as_ref)),
            Some(SubCommand::AddAlias(AddAlias {
                ref candidate_expressions,
                ..
            })) => Either::Left(candidate_expressions.iter().map(AsRef::as_ref)),
            Some(SubCommand::RemAlias(_)) | Some(SubCommand::List) => Either::Right(iter::empty()),
        }
    }

    pub fn mode(&self) -> Mode {
        match self.subcmd {
            None => {
                if self.show_average {
                    Mode::Average
                } else {
                    Mode::Norm
                }
            }
            Some(SubCommand::AddAlias(ref add)) => Mode::Add(add),
            Some(SubCommand::RemAlias(ref rem)) => Mode::Rem(&rem.alias),
            Some(SubCommand::List) => Mode::List,
        }
    }

    pub fn path_config(&self) -> Result<PathConfig> {
        static CONFIG_BASE: &str = ".roll";
        static HISTORY: &str = ".roll.history";

        let dirs = BaseDirs::new()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No home directory"))?;

        let config = self
            .config
            .as_ref()
            .map(|config_extension| {
                let filename = CONFIG_BASE.to_string()
                    + "."
                    + &config_extension
                        .trim_matches(|c: char| !c.is_ascii_alphabetic())
                        .to_ascii_lowercase();
                Cow::from(filename)
            })
            .unwrap_or_else(|| Cow::from(CONFIG_BASE));

        Ok(PathConfig {
            config: dirs.home_dir().join(config.as_ref()),
            history: dirs.home_dir().join(HISTORY),
        })
    }
}

#[derive(Clone, Debug, Parser)]
enum SubCommand {
    #[clap(name = "add")]
    AddAlias(AddAlias),
    #[clap(name = "rm")]
    RemAlias(RemAlias),
    #[clap(name = "list")]
    List,
}

/// Store a set of expressions with an alias for easy reuse.
#[derive(Clone, Debug, Parser)]
pub struct AddAlias {
    /// An easily-remembered name for a set of expressions
    pub alias: String,
    /// A comment or explanation of the stored forumlae
    #[clap(short, long)]
    pub comment: Option<String>,
    /// The expressions to be evaluated when the alias is provided
    pub candidate_expressions: Vec<String>,
}

/// Remove a previously stored alias.
#[derive(Clone, Debug, Parser)]
struct RemAlias {
    /// Alias to be removed
    alias: String,
}

#[derive(Copy, Clone, Debug)]
pub enum Mode<'a> {
    Norm,
    Average,
    Add(&'a AddAlias),
    Rem(&'a str),
    List,
}

#[derive(Clone, Debug)]
pub struct PathConfig {
    config: PathBuf,
    history: PathBuf,
}

impl PathConfig {
    pub fn config(&self) -> &Path {
        &self.config
    }

    pub fn history(&self) -> &Path {
        &self.history
    }
}
