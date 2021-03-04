mod implementation;
use combine::stream::position::DefaultPositioned;
pub use implementation::*;

use crate::Program;

pub fn parse_program_from_string(
    input: &str,
) -> Result<
    Program,
    easy_parse::ParseError<
        combine::stream::position::Stream<&str, <&str as DefaultPositioned>::Positioner>,
    >,
> {
    let stream = combine::stream::position::Stream::new(input);
    parse_program().easy_parse(stream).map(|(res, _input)| res)
}
