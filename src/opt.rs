use std::{path::PathBuf, str::FromStr};

use clap::Clap;


/// Options
#[derive(Clap, Debug)]
pub struct Opt {
    file: PathBuf,
    #[clap(short = 'd', long = "do", long = "action")]
    action: Action,
}

#[derive(Debug)]
pub enum Action {
    Lex,
    Parse,
    Run,
    Interpret,
}

impl FromStr for Action {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "lex" => Self::Lex,
            "parse" => Self::Parse,
            "run" => Self::Run,
            "interpret" => Self::Interpret,
            _ => return Err(format!("Expected lex, parse, run, got {}", s)),
        })
    }
}
