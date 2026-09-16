use crate::frontend::{Error, ErrorKind, Span, Spanned};
use crate::frontend::ast::{Type, BinOp, UnOp};
use crate::frontend::sema::typed_ast::*;

pub struct TypeChecker {
    ret_type: Option<Type>,
    current_file: String,
    current_source: String
}

impl TypeChecker {
    pub fn new(file: String, source: String) -> Self {
        Self { ret_type: None, current_file: file, current_source: source }
    }

    fn error(&self, span: Span, message: &str, note: Option<&str>) -> Error {
        Error::new(
            ErrorKind::Semantic,
            &self.current_file,
            &self.current_source,
            span,
            message,
            note
        )
    }

    fn is_noreturn(stmt: &TypedStatement) -> bool {
        match stmt {
            TypedStatement::Return(..) => true,
            TypedStatement::If { then_br, else_br, .. } => Self::is_block_noreturn(&then_br) && else_br.as_ref().map_or(
                false,
                |b| Self::is_block_noreturn(b)
            ),
            _ => false
        }
    }

    fn is_block_noreturn(block: &TypedBlock) -> bool {
        block.body.iter().any(Self::is_noreturn)
    }

    pub fn check(&mut self, program: &TypedProgram) -> Result<(), Error> {
        for item in &program.items {
            match &item {
                TypedItem::Function(func) => {
                    self.current_file = func.file.clone();
                    self.current_source = func.source.clone();
                    self.ret_type = Some(func.ret_type.node);
                    for param in &func.params {
                        if param.node.ty.node == Type::Void {
                            return Err(self.error(
                                param.node.ty.span,
                                &format!("Parameter '{}' can't be void", param.node.name),
                                Some(&format!("Remove parameter '{}'", param.node.name))
                            ));
                        }
                    }
                    if func.ret_type.node != Type::Void && !Self::is_block_noreturn(&func.body) {
                        return Err(self.error(
                            func.ret_type.span,
                            &format!("Missing return in non-void function '{}'", func.name.node),
                            None
                        ));
                    }
                    self.check_block(&func.body)?;
                    self.ret_type = None;
                },
                _ => ()
            }
        }
        Ok(())
    }

    fn check_block(&self, block: &TypedBlock) -> Result<(), Error> {
        for stmt in &block.body {
            self.check_stmt(&stmt)?;
        }
        Ok(())
    }

    fn check_stmt(&self, stmt: &TypedStatement) -> Result<(), Error> {
        match stmt {
            TypedStatement::Expression(expr) => self.check_expr(&expr)?,
            TypedStatement::Var { ident, ty, val, .. } => {
                let val_ty = val.node.infer_type();
                if val_ty != ty.node {
                    return Err(self.error(
                        val.span,
                        &format!("Expected type {}, got {}", ty.node, val_ty),
                        None
                    ));
                }
                if val_ty == Type::Void {
                    return Err(self.error(
                        val.span,
                        &format!("Variable '{}' can't be void", ident),
                        if matches!(val.node, TypedExpression::Call { .. }) {
                            Some("Just call void function without defining a variable")
                        } else {
                            None
                        }
                    ))
                }
                self.check_expr(&val)?;
            },
            TypedStatement::Assign { val, var_ty, .. } => {
                let ty = val.node.infer_type();
                if ty != *var_ty {
                    return Err(self.error(
                        val.span,
                        &format!("Expected type {}, got {}", var_ty, ty),
                        None
                    ));
                }
                self.check_expr(&val)?;
            },
            TypedStatement::If { condition, then_br, else_br } => {
                self.check_expr(&condition)?;
                let ty = condition.node.infer_type();
                if ty == Type::Str || ty == Type::Void {
                    return Err(self.error(
                        condition.span,
                        &format!("If condition can't be {}", ty),
                        None
                    ))
                }
                self.check_block(&then_br)?;
                if let Some(e) = else_br {
                    self.check_block(&e)?;
                }
            },
            TypedStatement::While { condition, body } => {
                self.check_expr(&condition)?;
                let ty = condition.node.infer_type();
                if ty == Type::Str || ty == Type::Void {
                    return Err(self.error(
                        condition.span,
                        &format!("While condition can't be {}", ty),
                        None
                    ))
                }
                self.check_block(&body)?;
            },
            TypedStatement::Return(expr, span) => {
                let ty = if let Some(e) = expr {
                    self.check_expr(&e)?;
                    e.node.infer_type()
                } else {
                    Type::Void
                };
                let ret_type = self.ret_type.unwrap();
                if ty != ret_type {
                    return Err(self.error(
                        *span,
                        &format!("Expected type {}, got {}", ret_type, ty),
                        None
                    ));
                }
            },
            _ => ()
        }
        Ok(())
    }

    fn check_expr(&self, expr: &Spanned<TypedExpression>) -> Result<(), Error> {
        match &expr.node {
            TypedExpression::BinOp { left, op, right, .. } => {
                let left_ty = left.node.infer_type();
                let right_ty = right.node.infer_type();
                if left_ty != right_ty {
                    return Err(self.error(
                        right.span,
                        &format!("Expected type {}, got {}", left_ty, right_ty),
                        None
                    ));
                }
                if left_ty == Type::Str {
                    return Err(self.error(
                        left.span,
                        "Can't append binary operation to str",
                        Some("You can append binary operation to any type literal except str")
                    ));
                }
                if matches!(op, BinOp::Plus | BinOp::Minus | BinOp::Multiply | BinOp::Divide) && !matches!(
                    left_ty,
                    Type::I64 | Type::U64 | Type::F64 |
                    Type::I32 | Type::U32 | Type::F32 |
                    Type::I16 | Type::U16 |
                    Type::I8 | Type::U8
                ) {
                    return Err(self.error(
                        left.span,
                        &format!("Can't append arithmetic operation to {}", left_ty),
                        Some("You can append arithmetic operation only to i64, u64, f64, i32, u32, f32, i16, u16, i8 and u8 literals")
                    ));
                }
                self.check_expr(&left)?;
                self.check_expr(&right)?;
            },
            TypedExpression::UnOp { op, operand, .. } => {
                let operand_ty = operand.node.infer_type();
                if *op == UnOp::Negate && !matches!(
                    operand_ty,
                    Type::I64 | Type::F64 |
                    Type::I32 | Type::F32 |
                    Type::I16 |
                    Type::I8
                ) {
                    return Err(self.error(
                        operand.span,
                        &format!("Can't negate {} expr", operand_ty),
                        Some("You can negate only i64, i32, i16, i8, f64 and f32 literals")
                    ));
                }
                if *op == UnOp::Not && operand_ty == Type::Char || operand_ty == Type::Str {
                    return Err(self.error(
                        operand.span,
                        &format!("Can't append not to {}", operand_ty),
                        Some("You can append not on any type literal except char and str")
                    ))
                }
                self.check_expr(&operand)?;
            },
            TypedExpression::Call { args, .. } => {
                for arg in args {
                    self.check_expr(&arg)?;
                }
            },
            TypedExpression::As { expr, ty } => {
                let expr_ty = expr.node.infer_type();
                if expr_ty == Type::Str || expr_ty == Type::Void {
                    return Err(self.error(
                        expr.span,
                        &format!("Can't cast from {}", expr_ty),
                        None
                    ));
                }
                if ty.node == Type::Str || ty.node == Type::Void {
                    return Err(self.error(
                        ty.span,
                        &format!("Can't cast to {}", ty.node),
                        None
                    ));
                }
                self.check_expr(&expr)?;
            },
            _ => ()
        }
        Ok(())
    }
}
