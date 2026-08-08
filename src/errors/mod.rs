use crate::lexer::tokens::TokenKind;
use crate::parser::ast::Type;

use std::fmt;

#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub line: usize,
    pub col: usize
}

impl Span {
    pub fn new(line: usize, col: usize) -> Span {
        Span { line, col }
    }
}

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

pub enum Error {
    IllegalChar {
        ch: char,
        span: Span
    },
    TooManyPoints {
        span: Span
    },
    InvalidEscape {
        escape: String,
        span: Span
    },
    UnterminatedString {
        span: Span
    },
    UnterminatedComment {
        span: Span
    },
    UnexpectedToken {
        expected: Vec<TokenKind>,
        got: TokenKind,
        span: Span
    },
    UnexpectedEOF,
    ModuleNotFound {
        path: String,
        span: Span
    },
    UnexpectedType {
        expected: Vec<Type>,
        got: Type,
        span: Span
    },
    UndefinedVariable {
        ident: String,
        span: Span
    },
    UndefinedFunction {
        ident: String,
        span: Span
    },
    InvalidAssignment {
        ident: String,
        span: Span
    },
    LLVMError {
        error: String
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IllegalChar { ch, span } => write!(f, "Illegal character '{}' at {}:{}", ch, span.line, span.col),
            Error::TooManyPoints { span } => write!(f, "Too many points at {}:{}", span.line, span.col),
            Error::InvalidEscape { escape, span } => write!(f, "Invalid escape '{}' at {}:{}", escape, span.line, span.col),
            Error::UnterminatedString { span } => write!(f, "Unterminated string at {}:{}", span.line, span.col),
            Error::UnterminatedComment { span } => write!(f, "Unterminated comment at {}:{}", span.line, span.col),
            Error::UnexpectedToken { expected, got, span } => write!(f, "Expected {}, got {} at {}:{}", expected.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", "), got, span.line, span.col),
            Error::UnexpectedEOF => write!(f, "Unexpected end of file"),
            Error::ModuleNotFound { path, span } => write!(f, "Module '{}' not found at {}:{}", path, span.line, span.col),
            Error::UnexpectedType { expected, got, span } => write!(f, "Expected {} type, got {} at {}:{}", expected.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", "), got, span.line, span.col),
            Error::UndefinedVariable { ident, span } => write!(f, "Undefined variable '{}' at {}:{}", ident, span.line, span.col),
            Error::UndefinedFunction { ident, span } => write!(f, "Undefined function '{}' at {}:{}", ident, span.line, span.col),
            Error::InvalidAssignment { ident, span } => write!(f, "Invalid assignment to '{}' at {}:{}", ident, span.line, span.col),
            Error::LLVMError { error } => write!(f, "LLVM error: {}", error),
        }
    }
}
