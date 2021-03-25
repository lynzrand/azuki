use std::{path::PathBuf, str::FromStr};

use clap::Clap;

/// Options
#[derive(Clap, Debug)]
pub struct Opt {
    /// The file to compile
    pub file: PathBuf,

    /// Output file. Omit for default file (compile) or stdout (other actions).
    #[clap(short, long = "out")]
    pub out_file: Option<PathBuf>,

    /// The action to perform. Accepts: lex, parse, compile, run
    #[clap(
        short = 'd',
        long = "do",
        long = "action",
        default_value = "compile",
        env = "AZUKI_ACTION"
    )]
    pub action: Action,

    /// The optimization passes to perform.
    #[clap(long = "opt", env = "AZUKI_OPT")]
    pub optimization: Option<Vec<String>>,

    #[clap(long)]
    pub entry_point: Option<String>,

    #[clap(long)]
    pub params: Vec<i64>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Lex,
    Parse,
    Run,
    Compile,
}

impl FromStr for Action {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "lex" => Self::Lex,
            "parse" => Self::Parse,
            "run" => Self::Run,
            "compile" => Self::Compile,
            _ => return Err(format!("Expected lex, parse, run, got {}", s)),
        })
    }
}
