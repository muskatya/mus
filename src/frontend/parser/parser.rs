use crate::frontend::{Error, ErrorKind, Span, Spanned};
use crate::frontend::{Token, TokenKind};
use crate::frontend::ast::*;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    file: String,
    source: String
}

impl Parser {
    pub fn new(tokens: Vec<Token>, file: String, source: String) -> Parser {
        Parser { tokens, pos: 0, file, source }
    }

    fn error(&self, span: Span, message: &str, note: Option<&str>) -> Error {
        Error::new(
            ErrorKind::Syntax,
            &self.file,
            &self.source,
            span,
            message,
            note
        )
    }

    fn eof_error(&self) -> Error {
        self.error(
            self.tokens.last().unwrap().span,
            "Unexpected end of file",
            None
        )
    }

    fn is_eof(&self) -> bool {
        self.peek().map_or(true, |t| t.kind == TokenKind::EOF)
    }

    fn advance(&mut self) -> Result<Token, Error> {
        let token = self.tokens.get(self.pos)
            .cloned()
            .ok_or_else(|| self.eof_error())?;
        self.pos += 1;
        Ok(token)
    }

    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.pos).cloned()
    }

    fn check(&self, expected: TokenKind) -> bool {
        self.peek().map_or(false, |t| t.kind == expected)
    }

    fn expect(&mut self, expected: TokenKind) -> Result<Token, Error> {
        let token = self.advance()?.clone();
        if token.kind != expected {
            return Err(self.error(
                token.span,
                &format!("Expected {}, found {}", expected, token.kind),
                None
            ));
        }
        Ok(token)
    }

    fn expect_ident(&mut self) -> Result<Spanned<String>, Error> {
        let token = self.advance()?;
        match &token.kind {
            TokenKind::Identifier(i) => Ok(Spanned::new(i.clone(), token.span)),
            _ => Err(self.error(
                token.span,
                &format!("Expected identifier, got '{}'", token.kind),
                None
            ))
        }
    }

    fn expect_type(&mut self) -> Result<Spanned<Type>, Error> {
        let token = self.advance()?;
        let span = token.span;
        let ty = match token.kind {
            TokenKind::I64 => Type::I64, TokenKind::U64 => Type::U64, TokenKind::F64 => Type::F64,
            TokenKind::I32 => Type::I32, TokenKind::U32 => Type::U32, TokenKind::F32 => Type::F32,
            TokenKind::I16 => Type::I16, TokenKind::U16 => Type::U16,
            TokenKind::I8 => Type::I8, TokenKind::U8 => Type::U8,
            TokenKind::Bool => Type::Bool,
            TokenKind::Char => Type::Char,
            TokenKind::Str => Type::Str,
            TokenKind::Void => Type::Void,
            _ => return Err(self.error(
                span,
                &format!("Expected type, got '{}'", token.kind),
                None
            ))
        };
        Ok(Spanned::new(ty, span))
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

    fn parse_params(&mut self) -> Result<Vec<Spanned<Param>>, Error> {
        let mut params = Vec::new();
        if self.check(TokenKind::RParen) {
            return Ok(params);
        }
        loop {
            if self.check(TokenKind::Ellipsis) {
                let span = self.advance()?.span;
                params.push(Spanned::new(Param {
                    name: "...".to_string(),
                    ty: Spanned::new(Type::Ellipsis, span)
                }, span));
                break;
            }
            let name = self.expect_ident()?;
            let mut span = name.span;
            let ty = self.expect_type()?;
            span.merge(ty.span);
            params.push(Spanned::new(Param { name: name.node, ty }, span));
            if self.check(TokenKind::RParen) {
                break;
            }
            self.expect(TokenKind::Comma)?;
        }
        Ok(params)
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

    fn parse_import_path(&mut self) -> Result<ImportPath, Error> {
        let mut path = Vec::new();
        let mut items = Vec::new();
        let mut alias = None;
        let mut span = self.peek()
            .ok_or_else(|| self.eof_error())?
            .span;
        while !self.is_eof() {
            if self.check(TokenKind::LParen) {
                self.advance()?;
                while !self.is_eof() && !self.check(TokenKind::RParen) {
                    items.push(self.parse_import_path()?);
                    if !self.check(TokenKind::RParen) {
                        self.expect(TokenKind::Comma)?;
                    }
                }
                self.advance()?;
                break;
            }
            let ident = self.expect_ident()?;
            path.push(ident.node);
            if !self.check(TokenKind::Dot) {
                span.merge(ident.span);
                break;
            }
            self.advance()?;
        }
        if items.is_empty() && self.check(TokenKind::As) {
            self.advance()?;
            alias = Some(self.expect_ident()?.node);
        }
        Ok(ImportPath { path: Spanned::new(path, span), items, alias })
    }

    pub fn parse_program(&mut self) -> Result<Program, Error> {
        let mut items = Vec::new();
        while !self.is_eof() {
            let visibility = match self.peek().unwrap().kind {
                TokenKind::Pub => {
                    self.advance()?;
                    Visibility::Public
                },
                _ => Visibility::Private
            };
            let token = self.peek()
                .ok_or_else(|| self.eof_error())?;
            let item = match token.kind {
                TokenKind::Import => self.parse_import(visibility)?,
                TokenKind::Extern => self.parse_extern(visibility)?,
                TokenKind::Fn => self.parse_function(visibility)?,
                _ => return Err(self.error(
                    token.span,
                    &format!("Expected import, extern or fn, got {}", token.kind),
                    None
                ))
            };
            items.push(item);
        }
        Ok(Program { items })
    }

    fn parse_import(&mut self, visibility: Visibility) -> Result<Item, Error> {
        self.advance()?;
        let path = self.parse_import_path()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(Item::Import(Import { path, visibility }))
    }

    fn parse_extern(&mut self, visibility: Visibility) -> Result<Item, Error> {
        self.advance()?;
        self.expect(TokenKind::Fn)?;
        let name = self.expect_ident()?.node;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;
        let ret_type = self.expect_type()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(Item::Extern(Extern { name, params, ret_type, visibility }))
    }

    fn parse_function(&mut self, visibility: Visibility) -> Result<Item, Error> {
        self.advance()?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;
        let ret_type = self.expect_type()?;
        let body = self.parse_block()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(Item::Function(Function { name, params, ret_type, body, visibility }))
    }

    fn parse_stmt(&mut self) -> Result<Statement, Error> {
        let stmt = match self.peek().map(|t| t.kind) {
            Some(TokenKind::Var) | Some(TokenKind::Const) => {
                let is_const = match self.advance()?.kind {
                    TokenKind::Var => false,
                    TokenKind::Const => true,
                    _ => unreachable!()
                };
                let ident = self.expect_ident()?.node;
                let ty = if self.peek().is_some_and(|t| t.kind.is_type()) {
                    Some(self.expect_type()?)
                } else {
                    None
                };
                self.expect(TokenKind::Assign)?;
                let val = self.parse_expr(0)?;
                self.expect(TokenKind::Semicolon)?;
                Statement::Var { ident, ty, val, is_const }
            },
            Some(TokenKind::Identifier(_)) if self.tokens.get(self.pos + 1).is_some_and(|t| t.kind == TokenKind::Assign) => {
                let ident = self.expect_ident()?;
                self.advance()?;
                let val = self.parse_expr(0)?;
                self.expect(TokenKind::Semicolon)?;
                Statement::Assign { ident, val }
            },
            Some(TokenKind::If) => self.parse_if(true)?,
            Some(TokenKind::While) => {
                self.advance()?;
                let condition = self.parse_expr(0)?;
                let body = self.parse_block()?;
                self.expect(TokenKind::Semicolon)?;
                Statement::While { condition, body }
            },
            Some(TokenKind::For) => {
                self.advance()?;
                let ident = self.expect_ident()?.node;
                self.expect(TokenKind::In)?;
                let iterable = self.parse_expr(0)?;
                let body = self.parse_block()?;
                self.expect(TokenKind::Semicolon)?;
                Statement::For { ident, iterable, body }
            },
            Some(TokenKind::Continue) => {
                let span = self.advance()?.span;
                self.expect(TokenKind::Semicolon)?;
                Statement::Continue(span)
            },
            Some(TokenKind::Break) => {
                let span = self.advance()?.span;
                self.expect(TokenKind::Semicolon)?;
                Statement::Break(span)
            },
            Some(TokenKind::Return) => {
                let mut span = self.advance()?.span;
                if self.check(TokenKind::Semicolon) {
                    self.advance()?;
                    return Ok(Statement::Return(None, span))
                }
                let expr = self.parse_expr(0)?;
                span.merge(expr.span);
                self.expect(TokenKind::Semicolon)?;
                Statement::Return(Some(expr), span)
            },
            Some(_) => {
                let expr = Statement::Expression(self.parse_expr(0)?);
                self.expect(TokenKind::Semicolon)?;
                expr
            },
            None => return Err(self.eof_error())
        };
        Ok(stmt)
    }

    fn parse_if(&mut self, expect_semi: bool) -> Result<Statement, Error> {
        self.expect(TokenKind::If)?;
        let condition = self.parse_expr(0)?;
        let then_br = self.parse_block()?;
        let else_br = if self.check(TokenKind::Else) {
            self.advance()?;
            if self.check(TokenKind::If) {
                Some(Block { stmts: vec![self.parse_if(false)?] })
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };
        if expect_semi {
            self.expect(TokenKind::Semicolon)?;
        }
        Ok(Statement::If { condition, then_br, else_br })
    }

    fn parse_expr(&mut self, min_bp: u8) -> Result<Spanned<Expression>, Error> {
        let mut left = self.parse_prefix()?;
        let mut span = left.span;
        loop {
            if self.check(TokenKind::As) {
                if 13 < min_bp {
                    break;
                }
                self.advance()?;
                let ty = self.expect_type()?;
                span.merge(ty.span);
                left = Spanned::new(
                    Expression::As { expr: Box::new(left), ty },
                    span
                );
                continue;
            }
            let op = match self.peek().map(|t| t.kind) {
                Some(TokenKind::Plus) => BinOp::Plus, Some(TokenKind::Minus) => BinOp::Minus, Some(TokenKind::Asterisk) => BinOp::Multiply, Some(TokenKind::Slash) => BinOp::Divide,
                Some(TokenKind::Greater) => BinOp::Greater, Some(TokenKind::Lower) => BinOp::Lower,
                Some(TokenKind::Eq) => BinOp::Eq, Some(TokenKind::NotEq) => BinOp::NotEq,
                Some(TokenKind::GreaterEq) => BinOp::GreaterEq, Some(TokenKind::LowerEq) => BinOp::LowerEq,
                Some(TokenKind::Or) => BinOp::Or, Some(TokenKind::And) => BinOp::And,
                _ => break
            };
            let (lbp, rbp) = op.binding_power();
            if lbp < min_bp {
                break;
            }
            self.advance()?;
            let right = self.parse_expr(rbp)?;
            span.merge(right.span);
            left = Spanned::new(
                Expression::BinOp { left: Box::new(left), op, right: Box::new(right) },
                span
            );
        }
        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Spanned<Expression>, Error> {
        let token = self.advance()?;
        let mut span = token.span;
        let expr = match &token.kind {
            TokenKind::Integer(int) => Expression::Integer(*int), TokenKind::Float(float) => Expression::Float(*float),
            TokenKind::True => Expression::Bool(true), TokenKind::False => Expression::Bool(false),
            TokenKind::Character(c) => Expression::Char(*c),
            TokenKind::String(string) => Expression::String(string.clone()),
            TokenKind::Identifier(i) => {
                let mut path = vec![i.clone()];
                while self.check(TokenKind::Dot) {
                    self.advance()?;
                    let next = self.expect_ident()?;
                    span.merge(next.span);
                    path.push(next.node);
                }
                let ident = path.join(".");
                if self.check(TokenKind::LParen) {
                    self.advance()?;
                    let args = self.parse_args()?;
                    let end = self.expect(TokenKind::RParen)?.span;
                    let call = Expression::Call {
                        ident: Spanned::new(ident, span),
                        args
                    };
                    span.merge(end);
                    call
                } else {
                    Expression::Identifier(ident)
                }
            },
            TokenKind::Minus => {
                let operand = self.parse_expr(13)?;
                span.merge(operand.span);
                Expression::UnOp {
                    op: UnOp::Negate,
                    operand: Box::new(operand)
                }
            },
            TokenKind::Exclamation => {
                let operand = self.parse_expr(13)?;
                span.merge(operand.span);
                Expression::UnOp {
                    op: UnOp::Not,
                    operand: Box::new(operand)
                }
            },
            TokenKind::LParen => {
                let expr = self.parse_expr(0)?;
                span.merge(self.expect(TokenKind::RParen)?.span);
                expr.node
            },
            _ => {
                return Err(self.error(
                    token.span,
                    &format!("Expected expr, got {}", token.kind),
                    None
                ));
            }
        };
        Ok(Spanned::new(expr, span))
    }
}
