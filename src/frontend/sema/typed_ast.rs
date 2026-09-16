use crate::frontend::{Span, Spanned};
use crate::frontend::ast::{Type, Visibility, BinOp, UnOp};

#[derive(Debug)]
pub struct TypedBlock {
    pub body: Vec<TypedStatement>
}

#[derive(Debug)]
pub struct TypedParam {
    pub name: String,
    pub ty: Spanned<Type>
}

#[derive(Debug)]
pub enum ImportKind {
    Item {
        name: String
    },
    Items,
    // All
}

#[derive(Debug)]
pub struct TypedProgram {
    pub items: Vec<TypedItem>
}

#[derive(Debug)]
pub enum TypedItem {
    Extern(TypedExtern),
    Function(TypedFunction)
}

#[derive(Debug)]
pub struct TypedExtern {
    pub name: String,
    pub mangled: String,
    pub params: Vec<Spanned<TypedParam>>,
    pub ret_type: Spanned<Type>,
    pub symbol: usize,
    pub visibility: Visibility
}

#[derive(Debug)]
pub struct TypedFunction {
    pub name: Spanned<String>,
    pub mangled: String,
    pub params: Vec<Spanned<TypedParam>>,
    pub ret_type: Spanned<Type>,
    pub body: TypedBlock,
    pub symbol: usize,
    pub visibility: Visibility,
    pub file: String,
    pub source: String
}

#[derive(Debug)]
pub enum TypedStatement {
    Expression(Spanned<TypedExpression>),
    Var {
        ident: String,
        ty: Spanned<Type>,
        val: Spanned<TypedExpression>,
        is_const: bool
    },
    Assign {
        ident: Spanned<String>,
        val: Spanned<TypedExpression>,
        symbol: usize,
        var_ty: Type
    },
    If {
        condition: Spanned<TypedExpression>,
        then_br: TypedBlock,
        else_br: Option<TypedBlock>
    },
    While {
        condition: Spanned<TypedExpression>,
        body: TypedBlock
    },
    /*For {
        ident: String,
        iterable: Spanned<TypedExpression>,
        body: TypedBlock
    },*/
    Continue(Span),
    Break(Span),
    Return(Option<Spanned<TypedExpression>>, Span)
}

#[derive(Debug)]
pub enum TypedExpression {
    Integer {
        val: i64,
        ty: Type
    },
    Float {
        val: f64,
        ty: Type
    },
    Bool(bool),
    Char(char),
    String(String),
    Identifier {
        name: String,
        ty: Type,
        symbol: usize
    },
    BinOp {
        left: Box<Spanned<TypedExpression>>,
        op: BinOp,
        right: Box<Spanned<TypedExpression>>,
        ty: Type
    },
    UnOp {
        op: UnOp,
        operand: Box<Spanned<TypedExpression>>,
        ty: Type
    },
    Call {
        ident: Spanned<String>,
        mangled: String,
        ty: Type,
        symbol: usize,
        args: Vec<Spanned<TypedExpression>>
    },
    As {
        expr: Box<Spanned<TypedExpression>>,
        ty: Spanned<Type>
    }
}

impl TypedExpression {
    pub fn infer_type(&self) -> Type {
        match self {
            Self::Integer { ty, .. } => *ty,
            Self::Float { ty, .. } => *ty,
            Self::Bool(_) => Type::Bool,
            Self::Char(_) => Type::Char,
            Self::String(_) => Type::Str,
            Self::Identifier { ty, .. } => *ty,
            Self::BinOp { ty, .. } => *ty,
            Self::UnOp { ty, .. } => *ty,
            Self::Call { ty, .. } => *ty,
            Self::As { ty, .. } => ty.node
        }
    }
}
