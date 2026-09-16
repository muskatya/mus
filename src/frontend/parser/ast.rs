use crate::frontend::{Span, Spanned};
use std::fmt;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Type {
    I64, I32, I16, I8,
    U64, U32, U16, U8,
    F64, F32,
    Bool,
    Char,
    Str,
    Void,
    Ellipsis
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::I64 => write!(f, "i64"),
            Type::I32 => write!(f, "i32"),
            Type::I16 => write!(f, "i16"),
            Type::I8 => write!(f, "i8"),
            Type::U64 => write!(f, "u64"),
            Type::U32 => write!(f, "u32"),
            Type::U16 => write!(f, "u16"),
            Type::U8 => write!(f, "u8"),
            Type::F64 => write!(f, "f64"),
            Type::F32 => write!(f, "f32"),
            Type::Bool => write!(f, "bool"),
            Type::Char => write!(f, "char"),
            Type::Str => write!(f, "str"),
            Type::Void => write!(f, "void"),
            Type::Ellipsis => write!(f, "...")
        }
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Statement>
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Visibility {
    Public,
    Private
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Spanned<Type>
}

#[derive(Debug, Clone)]
pub struct ImportPath {
    pub path: Spanned<Vec<String>>,
    pub items: Vec<ImportPath>,
    pub alias: Option<String>
}

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>
}

#[derive(Debug, Clone)]
pub enum Item {
    Function(Function),
    Import(Import),
    Extern(Extern)
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: Spanned<String>,
    pub params: Vec<Spanned<Param>>,
    pub ret_type: Spanned<Type>,
    pub body: Block,
    pub visibility: Visibility
}

#[derive(Debug, Clone)]
pub struct Import {
    pub path: ImportPath,
    pub visibility: Visibility
}

#[derive(Debug, Clone)]
pub struct Extern {
    pub name: String,
    pub params: Vec<Spanned<Param>>,
    pub ret_type: Spanned<Type>,
    pub visibility: Visibility
}

#[derive(Debug, Clone)]
pub enum Statement {
    Expression(Spanned<Expression>),
    Var {
        ident: String,
        ty: Option<Spanned<Type>>,
        val: Spanned<Expression>,
        is_const: bool
    },
    Assign {
        ident: Spanned<String>,
        val: Spanned<Expression>
    },
    If {
        condition: Spanned<Expression>,
        then_br: Block,
        else_br: Option<Block>
    },
    While {
        condition: Spanned<Expression>,
        body: Block
    },
    For {
        ident: String,
        iterable: Spanned<Expression>,
        body: Block
    },
    Continue(Span),
    Break(Span),
    Return(Option<Spanned<Expression>>, Span)
}

#[derive(Debug, Clone)]
pub enum Expression {
    Integer(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
    Identifier(String),
    BinOp {
        left: Box<Spanned<Expression>>,
        op: BinOp,
        right: Box<Spanned<Expression>>
    },
    UnOp {
        op: UnOp,
        operand: Box<Spanned<Expression>>
    },
    Call {
        ident: Spanned<String>,
        args: Vec<Spanned<Expression>>
    },
    As {
        expr: Box<Spanned<Expression>>,
        ty: Spanned<Type>
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Plus, Minus, Multiply, Divide,
    Eq, Greater, Lower, GreaterEq, LowerEq, NotEq,
    And, Or
}

impl BinOp {
    pub fn binding_power(&self) -> (u8, u8) {
        match self {
            BinOp::Or => (1, 2),
            BinOp::And => (3, 4),
            BinOp::Eq | BinOp::NotEq => (5, 6),
            BinOp::Greater | BinOp::GreaterEq |
            BinOp::Lower | BinOp::LowerEq => (7, 8),
            BinOp::Plus | BinOp::Minus => (9, 10),
            BinOp::Multiply | BinOp::Divide => (11, 12)
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum UnOp {
    Negate, Not
}
