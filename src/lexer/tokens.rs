use crate::errors::Span;

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Plus, Minus, Asterisk, Slash, Assign, LParen, RParen, LSquare, RSquare, LBrace, RBrace, Exclamation, Comma, Dot, Ellipsis, Semicolon,
    Greater, Lower, Eq, GreaterEq, LowerEq, NotEq,
    Integer(i64), Float(f64), String(String), Identifier(String),
    I64, I32, I16, I8, U64, U32, U16, U8, F64, F32, Bool, Str, Void,
    Fn, Import, Extern, Var, Const, Return, If, Else, Or, And, As, While, For, In, True, False,
    EOF
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let format = match self {
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Asterisk => "*",
            TokenKind::Slash => "/",
            TokenKind::Assign => "=",
            TokenKind::LParen => "(",
            TokenKind::RParen => ")",
            TokenKind::LSquare => "[",
            TokenKind::RSquare => "]",
            TokenKind::LBrace => "{",
            TokenKind::RBrace => "}",
            TokenKind::Exclamation => "!",
            TokenKind::Comma => ",",
            TokenKind::Dot => ".",
            TokenKind::Ellipsis => "...",
            TokenKind::Semicolon => ";",
            TokenKind::Greater => ">",
            TokenKind::Lower => "<",
            TokenKind::Eq => "==",
            TokenKind::GreaterEq => ">=",
            TokenKind::LowerEq => "<=",
            TokenKind::NotEq => "!=",
            TokenKind::Integer(_) => "int literal",
            TokenKind::Float(_) => "float literal",
            TokenKind::String(_) => "string literal",
            TokenKind::Identifier(_) => "identifier",
            TokenKind::I64 => "i64",
            TokenKind::I32 => "i32",
            TokenKind::I16 => "i16",
            TokenKind::I8 => "i8",
            TokenKind::U64 => "u64",
            TokenKind::U32 => "u32",
            TokenKind::U16 => "u16",
            TokenKind::U8 => "u8",
            TokenKind::F64 => "f64",
            TokenKind::F32 => "f32",
            TokenKind::Bool => "bool",
            TokenKind::Str => "str",
            TokenKind::Void => "void",
            TokenKind::Fn => "fn",
            TokenKind::Extern => "extern",
            TokenKind::Import => "import",
            TokenKind::Var => "var",
            TokenKind::Const => "const",
            TokenKind::Return => "return",
            TokenKind::If => "if",
            TokenKind::Else => "else",
            TokenKind::Or => "or",
            TokenKind::And => "and",
            TokenKind::As => "as",
            TokenKind::While => "while",
            TokenKind::For => "for",
            TokenKind::In => "in",
            TokenKind::True => "true",
            TokenKind::False => "false",
            TokenKind::EOF => "eof"
        };
        write!(f, "{}", format)
    }
}

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Token {
        Token { kind, span }
    }
}
