use std::iter;

use clap::{crate_authors, crate_description, crate_version, Clap};
use either::Either;

#[derive(Clap, Clone, Debug)]
#[clap(author = crate_authors!(), about = crate_description!(), version = crate_version!())]
pub struct Opts {
    candidate_expressions: Vec<String>,
    #[clap(subcommand)]
    subcmd: Option<SubCommand>,
}

impl Opts {
    pub fn parse() -> Self {
        Clap::parse()
    }

    pub fn candidates(&self) -> impl Iterator<Item = &str> {
        match self.subcmd {
            None => Either::Left(self.candidate_expressions.iter().map(AsRef::as_ref)),
            Some(SubCommand::AddAlias(AddAlias {
                ref candidate_expressions,
                ..
            })) => Either::Left(candidate_expressions.iter().map(AsRef::as_ref)),
            Some(SubCommand::RemAlias(_)) | Some(SubCommand::List) => Either::Right(iter::empty()),
        }
    }

    pub fn mode(&self) -> Mode {
        match self.subcmd {
            None => Mode::Norm,
            Some(SubCommand::AddAlias(ref add)) => Mode::Add(&add.alias),
            Some(SubCommand::RemAlias(ref rem)) => Mode::Rem(&rem.alias),
            Some(SubCommand::List) => Mode::List,
        }
    }
}

#[derive(Clap, Clone, Debug)]
enum SubCommand {
    #[clap(name = "add")]
    AddAlias(AddAlias),
    #[clap(name = "rm")]
    RemAlias(RemAlias),
    #[clap(name = "list")]
    List,
}

/// Store a set of expressions with an alias for easy reuse.
#[derive(Clap, Clone, Debug)]
struct AddAlias {
    /// An easily-remembered name for a set of expressions.
    alias: String,
    /// The expressions to be evaluated when the alias is provided.
    candidate_expressions: Vec<String>,
}

/// Remove a previously stored alias.
#[derive(Clap, Clone, Debug)]
struct RemAlias {
    /// Alias to be removed
    alias: String,
}

#[derive(Copy, Clone, Debug)]
pub enum Mode<'a> {
    Norm,
    Add(&'a str),
    Rem(&'a str),
    List,
}
