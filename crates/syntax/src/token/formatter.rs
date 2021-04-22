use std::fmt::Display;

use crate::Token;

macro_rules! write_match {
    ($self:expr, $f:expr, $($pat:pat => {$format:literal$(, $($params:expr),*)?})*) => {
        match $self{
            $(
                $pat => {
                    write!($f,$format $(, $($params),*)?)
                }
            )*
        }
    };
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_match! {
            self, f,
            Token::FnKw => {"fn"}
            Token::LetKw => {"let"}
            Token::ConstKw => {"const"}
            Token::AsKw => {"as"}
            Token::WhileKw => {"while"}
            Token::IfKw => {"if"}
            Token::ElseKw => {"else"}
            Token::ReturnKw => {"return"}
            Token::BreakKw => {"break"}
            Token::ContinueKw => {"continue"}
            Token::UIntLiteral(i) => {"uint {}",i}
            Token::FloatLiteral(i) => {"float {}", i}
            Token::CharLiteral(c) => {"char {}", c}
            Token::StringLiteral(s) => {"string {}", s}
            Token::Ident(id) => {"ident {}", id}
            Token::Plus => {"plus"}
            Token::Minus => {"minus"}
            Token::Mul => {"mul"}
            Token::Div => {"div"}
            Token::And => {"and"}
            Token::Assign => {"assign"}
            Token::Eq => {"eq"}
            Token::Neq => {"neq"}
            Token::Lt => {"lt"}
            Token::Gt => {"gt"}
            Token::Le => {"le"}
            Token::Ge => {"ge"}
            Token::LParen => {"lparen"}
            Token::RParen => {"rparen"}
            Token::LBrace => {"lbrace"}
            Token::RBrace => {"rbrace"}
            Token::Arrow => {"arrow"}
            Token::Comma => {"comma"}
            Token::Colon => {"colon"}
            Token::Semicolon => {"semicolon"}
            Token::Whitespace => {"WS"}
            Token::Comment => {"comment"}
            Token::Error => {"err"}
        }
    }
}
