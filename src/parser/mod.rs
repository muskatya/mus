pub mod ast;

use crate::errors::{Spanned, Error};
use crate::lexer::tokens::{Token, TokenKind};
use ast::*;

pub struct Parser {
    pub tokens: Vec<Token>,
    pub pos: usize
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Parser {
        Parser { tokens, pos: 0 }
    }

    fn is_eof(&self) -> bool {
        self.peek().map_or(true, |t| t.kind == TokenKind::EOF)
    }

    fn advance(&mut self) -> Result<&Token, Error> {
        let token = match self.tokens.get(self.pos) {
            Some(t) => Ok(t),
            None => Err(Error::UnexpectedEOF)
        };
        self.pos += 1;
        token
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn expect(&mut self, expected: TokenKind) -> Result<&Token, Error> {
        let token = self.advance()?;
        if token.kind != expected {
            return Err(Error::UnexpectedToken {
                expected: vec![expected],
                got: token.kind.clone(),
                span: token.span
            });
        }
        Ok(token)
    }

    fn expect_ident(&mut self) -> Result<Spanned<String>, Error> {
        let token = self.advance()?;
        match &token.kind {
            TokenKind::Identifier(i) => Ok(Spanned::new(i.clone(), token.span)),
            _ => Err(Error::UnexpectedToken {
                expected: vec![TokenKind::Identifier(String::new())],
                got: token.kind.clone(),
                span: token.span
            })
        }
    }

    fn check(&self, expected: TokenKind) -> bool {
        self.peek().map_or(false, |t| t.kind == expected)
    }

    fn expect_type(&mut self) -> Result<Spanned<Type>, Error> {
        let token = self.advance()?;
        let ty = match token.kind {
            TokenKind::I64 => Type::I64, TokenKind::I32 => Type::I32, TokenKind::I16 => Type::I16, TokenKind::I8 => Type::I8,
            TokenKind::U64 => Type::U64, TokenKind::U32 => Type::U32, TokenKind::U16 => Type::U16, TokenKind::U8 => Type::U8,
            TokenKind::F64 => Type::F64, TokenKind::F32 => Type::F32,
            TokenKind::Bool => Type::Bool,
            TokenKind::Str => Type::Str,
            TokenKind::Void => Type::Void,
            _ => return Err(Error::UnexpectedToken { expected: vec![
                TokenKind::I64, TokenKind::I32, TokenKind::I16, TokenKind::I8,
                TokenKind::U64, TokenKind::U32, TokenKind::U16, TokenKind::U8,
                TokenKind::F64, TokenKind::F32,
                TokenKind::Bool,
                TokenKind::Str,
                TokenKind::Void
            ], got: token.kind.clone(), span: token.span })
        };
        Ok(Spanned::new(ty, token.span))
    }

    fn is_type(&self) -> bool {
        self.peek().map_or(false, |t|
            matches!(t.kind,
                TokenKind::I64 | TokenKind::I32 | TokenKind::I16 | TokenKind::I8 |
                TokenKind::U64 | TokenKind::U32 | TokenKind::U16 | TokenKind::U8 |
                TokenKind::F64 | TokenKind::F32 |
                TokenKind::Bool |
                TokenKind::Str |
                TokenKind::Void
            )
        )
    }

    pub fn parse_program(&mut self) -> Result<Program, Error> {
        let mut functions = Vec::new();
        let mut externs = Vec::new();
        let mut imports = Vec::new();
        while !self.is_eof() {
            let token = self.peek().unwrap();
            match token.kind {
                TokenKind::Fn => functions.push(self.parse_function()?),
                TokenKind::Extern => externs.push(self.parse_extern()?),
                TokenKind::Import => imports.push(self.parse_import()?),
                _ => return Err(Error::UnexpectedToken { expected: vec![TokenKind::Fn, TokenKind::Extern, TokenKind::Import], got: token.kind.clone(), span: token.span })
            }
        }
        Ok(Program { functions, externs, imports })
    }

    fn parse_block(&mut self) -> Result<Block, Error> {
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(TokenKind::RBrace) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Block { stmts })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, Error> {
        let mut params = Vec::new();
        if self.check(TokenKind::RParen) {
            return Ok(params);
        }
        loop {
            if self.check(TokenKind::Ellipsis) {
                let span = self.advance()?.span;
                params.push(Param { name: "...".to_string(), ty: Spanned::new(Type::Ellipsis, span) });
                break;
            }
            let name = self.expect_ident()?.node;
            let ty = self.expect_type()?;
            params.push(Param { name, ty });
            if self.check(TokenKind::RParen) {
                break;
            }
            self.expect(TokenKind::Comma)?;
        }
        Ok(params)
    }

    fn parse_function(&mut self) -> Result<Function, Error> {
        self.advance()?;
        let name = self.expect_ident()?.node;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;
        let ret_type = self.expect_type()?;
        let body = self.parse_block()?;
        Ok(Function { name, params, ret_type, body })
    }

    fn parse_import(&mut self) -> Result<Import, Error> {
        self.advance()?;
        let mut path = String::new();
        let span = self.peek().ok_or_else(|| Error::UnexpectedEOF)?.span;
        while let Some(t) = self.peek() && matches!(t.kind, TokenKind::Identifier(_) | TokenKind::Dot) {
            match &t.kind {
                TokenKind::Identifier(i) => path.push_str(i),
                TokenKind::Dot => path.push('/'),
                _ => unreachable!()
            }
            self.advance()?;
        }
        /* let mut alias = None;
        if self.check(TokenKind::As) {
            self.advance();
            alias = Some(self.expect_ident()?);
        } */
        self.expect(TokenKind::Semicolon)?;
        Ok(Import { path: Spanned::new(path, span) /* , alias */ })
    }

    fn parse_extern(&mut self) -> Result<Extern, Error> {
        self.advance()?;
        self.expect(TokenKind::Fn)?;
        let name = self.expect_ident()?.node;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;
        let ret_type = self.expect_type()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(Extern { name, params, ret_type })
    }

    fn parse_stmt(&mut self) -> Result<Statement, Error> {
        match self.peek().map(|t| &t.kind) {
            Some(TokenKind::Var) => {
                self.advance()?;
                let ident = self.expect_ident()?.node;
                let ty = match self.is_type() {
                    true => Some(self.expect_type()?),
                    false => None
                };
                self.expect(TokenKind::Assign)?;
                let val = self.parse_expr(0)?;
                self.expect(TokenKind::Semicolon)?;
                Ok(Statement::Var { ident, ty, val })
            },
            Some(TokenKind::Identifier(_)) if self.tokens.get(self.pos + 1).map_or(false, |t| t.kind == TokenKind::Assign) => {
                let ident = self.expect_ident()?;
                self.expect(TokenKind::Assign)?;
                let val = self.parse_expr(0)?;
                self.expect(TokenKind::Semicolon)?;
                Ok(Statement::Assign { ident, val })
            },
            Some(TokenKind::Const) => {
                self.advance()?;
                let ident = self.expect_ident()?.node;
                let ty = if self.is_type() {
                    Some(self.expect_type()?)
                } else { None };
                self.expect(TokenKind::Assign)?;
                let val = self.parse_expr(0)?;
                self.expect(TokenKind::Semicolon)?;
                Ok(Statement::Const { ident, ty, val })
            },
            Some(TokenKind::If) => {
                self.advance()?;
                let condition = self.parse_expr(0)?;
                let then_br = self.parse_block()?;
                let mut else_br = None;
                if self.check(TokenKind::Else) {
                    self.advance()?;
                    else_br = Some(self.parse_block()?);
                }
                Ok(Statement::If { condition, then_br, else_br })
            },
            Some(TokenKind::While) => {
                self.advance()?;
                let condition = self.parse_expr(0)?;
                let body = self.parse_block()?;
                Ok(Statement::While { condition, body })
            },
            Some(TokenKind::For) => {
                self.advance()?;
                let var = self.expect_ident()?.node;
                self.expect(TokenKind::In)?;
                let iterable = self.parse_expr(0)?;
                let body = self.parse_block()?;
                Ok(Statement::For { var, iterable, body })
            },
            Some(TokenKind::Return) => {
                self.advance()?;
                if self.check(TokenKind::Semicolon) {
                    self.advance()?;
                    return Ok(Statement::Return(None))
                }
                let expr = self.parse_expr(0)?;
                self.expect(TokenKind::Semicolon)?;
                Ok(Statement::Return(Some(expr)))
            },
            Some(_) => {
                let expr = Statement::Expression(self.parse_expr(0)?);
                self.expect(TokenKind::Semicolon)?;
                Ok(expr)
            },
            None => return Err(Error::UnexpectedEOF)
        }
    }

    fn parse_args(&mut self) -> Result<Vec<Spanned<Expression>>, Error> {
        let mut args = Vec::new();
        if self.check(TokenKind::RParen) {
            return Ok(args)
        }
        loop {
            args.push(self.parse_expr(0)?);
            if self.check(TokenKind::RParen) {
                break;
            }
            self.expect(TokenKind::Comma)?;
        }
        Ok(args)
    }

    fn parse_expr(&mut self, min_bp: u8) -> Result<Spanned<Expression>, Error> {
        let token = self.advance()?;
        let span = token.span;
        let left = match &token.kind {
            TokenKind::Integer(i) => Expression::Integer(*i),
            TokenKind::Float(f) => Expression::Float(*f),
            TokenKind::True => Expression::Bool(true),
            TokenKind::False => Expression::Bool(false),
            TokenKind::String(s) => Expression::String(s.clone()),
            TokenKind::Identifier(i) => {
                let ident = i.clone();
                if self.check(TokenKind::LParen) {
                    self.advance()?;
                    let args = self.parse_args()?;
                    self.expect(TokenKind::RParen)?;
                    Expression::Call { ident: Spanned::new(ident, span), args }
                } else {
                    Expression::Identifier(ident)
                }
            },
            TokenKind::Minus => Expression::UnOp {
                op: UnOp::Negate,
                operand: Box::new(self.parse_expr(13)?)
            },
            TokenKind::Exclamation => Expression::UnOp {
                op: UnOp::Not,
                operand: Box::new(self.parse_expr(13)?)
            },
            TokenKind::LParen => {
                let expr = self.parse_expr(0)?;
                self.expect(TokenKind::RParen)?;
                expr.node
            },
            _ => {
                return Err(Error::UnexpectedToken { expected: vec![
                    TokenKind::Integer(0), TokenKind::Float(0.0), TokenKind::True, TokenKind::False,
                    TokenKind::String(String::new()), TokenKind::Identifier(String::new()), TokenKind::Minus,
                    TokenKind::Exclamation, TokenKind::LParen
                ], got: token.kind.clone(), span: token.span });
            }
        };
        let mut spanned_left = Spanned::new(left, span);
        loop {
            if self.check(TokenKind::As) {
                if 13 < min_bp {
                    break;
                }
                self.advance()?;
                let ty = self.expect_type()?;
                spanned_left = Spanned::new(
                    Expression::As { expr: Box::new(spanned_left), ty },
                    span
                );
                continue;
            }
            let op = match self.peek().map(|t| &t.kind) {
                Some(TokenKind::Or) => BinOp::Or,
                Some(TokenKind::And) => BinOp::And,
                Some(TokenKind::Eq) => BinOp::Eq,
                Some(TokenKind::NotEq) => BinOp::NotEq,
                Some(TokenKind::Greater) => BinOp::Greater,
                Some(TokenKind::GreaterEq) => BinOp::GreaterEq,
                Some(TokenKind::Lower) => BinOp::Lower,
                Some(TokenKind::LowerEq) => BinOp::LowerEq,
                Some(TokenKind::Plus) => BinOp::Plus,
                Some(TokenKind::Minus) => BinOp::Minus,
                Some(TokenKind::Asterisk) => BinOp::Multiply,
                Some(TokenKind::Slash) => BinOp::Divide,
                _ => break
            };
            let (lbp, rbp) = op.binding_power();
            if lbp < min_bp {
                break;
            }
            self.advance()?;
            let right = self.parse_expr(rbp)?;
            spanned_left = Spanned::new(
                Expression::BinOp { left: Box::new(spanned_left), op, right: Box::new(right) },
                span
            );
        }
        Ok(spanned_left)
    }
}
