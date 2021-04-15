mod implementation;
use std::{collections::HashMap};

pub use implementation::*;

use crate::Program;

pub fn parse_program_from_string(input: &str) -> Result<Program, ParseError> {
    let mut parser = lexpr::Parser::from_str_custom(
        input,
        lexpr::parse::Options::new().with_nil_symbol(lexpr::parse::NilSymbol::EmptyList),
    );
    let val = parser.datum_iter();
    let res = val.map(|x| x.map(|x| parse_function(x.as_ref())));
    let mut program = Program {
        functions: HashMap::new(),
    };
    for x in res {
        let x = x??;
        program.functions.insert(x.name.clone(), x);
    }
    Ok(program)
}
