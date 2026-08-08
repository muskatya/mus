use crate::errors::Spanned;

use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    I64, I32, I16, I8,
    U64, U32, U16, U8,
    F64, F32,
    Bool,
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
            Type::Str => write!(f, "str"),
            Type::Void => write!(f, "void"),
            Type::Ellipsis => write!(f, "...")
        }
    }
}

#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
    pub externs: Vec<Extern>,
    pub imports: Vec<Import>
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_type: Spanned<Type>,
    pub body: Block
}

#[derive(Debug)]
pub struct Param {
    pub name: String,
    pub ty: Spanned<Type>
}

#[derive(Debug)]
pub struct Block {
    pub stmts: Vec<Statement>
}

#[derive(Debug)]
pub struct Import {
    pub path: Spanned<String>,
    // pub alias: Option<String>
}

#[derive(Debug)]
pub struct Extern {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_type: Spanned<Type>,
}

#[derive(Debug)]
pub enum Statement {
    Expression(Spanned<Expression>),
    Var {
        ident: String,
        ty: Option<Spanned<Type>>,
        val: Spanned<Expression>
    },
    Assign {
        ident: Spanned<String>,
        val: Spanned<Expression>
    },
    Const {
        ident: String,
        ty: Option<Spanned<Type>>,
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
        var: String,
        iterable: Spanned<Expression>,
        body: Block
    },
    Return(Option<Spanned<Expression>>)
}

#[derive(Debug)]
pub enum Expression {
    Integer(i64),
    Float(f64),
    Bool(bool),
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

#[derive(Debug)]
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

#[derive(Debug)]
pub enum UnOp {
    Negate, Not
}
