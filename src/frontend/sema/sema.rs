use crate::frontend::{Error, ErrorKind, Span, Spanned};
use crate::frontend::ast::*;
use crate::frontend::sema::context::Context;
use crate::frontend::sema::{modules, typed_ast::*};
use crate::frontend::sema::scopes::ScopeKind;

pub struct Sema {
    context: Context,
    current_func: Option<String>,
    current_module: Option<String>,
    current_file: String,
    current_source: String
}

impl Sema {
    pub fn new(file: String, source: String) -> Self {
        Self {
            context: Context::new(),
            current_func: None,
            current_module: None,
            current_file: file,
            current_source: source
        }
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

    pub fn analyze(&mut self, program: &Program) -> Result<TypedProgram, Error> {
        let mut module_system = modules::ModuleSystem::new(self.current_file.clone(), self.current_source.clone());
        module_system.register_main(&program, self.current_file.clone(), self.current_source.clone())?;
        let (context, modules, scopes) = module_system.into_parts();
        self.context = context;
        let global_scope = self.context.current_scope;
        if self.context.get_func("main.main").is_none() {
            return Err(self.error(
                Span::short(0),
                "Function 'main' is undefined",
                Some("Define a public function 'main'")
            ));
        }
        let mut items = Vec::new();
        for module in &modules {
            self.current_module = Some(module.path.clone());
            self.current_file = module.file.clone();
            self.current_source = module.source.clone();
            self.context.current_scope = match scopes.get(&module.path) {
                Some(&s) => s,
                None => global_scope
            };
            for item in &module.program.items {
                let typed_item = match item {
                    Item::Extern(ext) => TypedItem::Extern(self.analyze_extern(&ext, &module.path)?),
                    Item::Function(func) => TypedItem::Function(self.analyze_function(&func, &module.path)?),
                    _ => continue
                };
                items.push(typed_item);
            }
            self.current_module = None;
        }
        Ok(TypedProgram { items })
    }

    fn analyze_extern(&mut self, ext: &Extern, module_path: &str) -> Result<TypedExtern, Error> {
        let mangled = format!("{}.{}", module_path, ext.name);
        let symbol = self.context.get_func(&mangled).unwrap().id;
        Ok(TypedExtern {
            name: ext.name.clone(),
            mangled,
            params: ext.params.iter().map(|p| Spanned::new(TypedParam {
                name: p.node.name.clone(),
                ty: p.node.ty.clone()
            }, p.span)).collect(),
            ret_type: ext.ret_type.clone(),
            symbol,
            visibility: ext.visibility
        })
    }

    fn analyze_function(&mut self, func: &Function, module_path: &str) -> Result<TypedFunction, Error> {
        self.context.push_scope(ScopeKind::Local);
        let mangled = format!("{}.{}", module_path, func.name.node);
        self.current_func = Some(mangled.clone());
        if mangled == "main.main" {
            if func.visibility != Visibility::Public {
                return Err(self.error(
                    func.name.span,
                    "Function 'main' is private",
                    Some("Make function 'main' public")
                ));
            }
            if let Some(p) = func.params.get(0) {
                return Err(self.error(
                    p.span,
                    "Function 'main' can't have any params",
                    None
                ));
            }
        }
        for param in &func.params {
            self.context.add_var(
                &param.node.name,
                param.node.ty.node,
                true
            );
        }
        let body = self.analyze_block(&func.body)?;
        let symbol = self.context.get_func(&mangled).unwrap().id;
        self.context.pop_scope();
        self.current_func = None;
        Ok(TypedFunction {
            name: func.name.clone(),
            mangled,
            params: func.params.iter().map(|p| Spanned::new(TypedParam {
                name: p.node.name.clone(),
                ty: p.node.ty.clone()
            }, p.span)).collect(),
            ret_type: func.ret_type.clone(),
            body,
            symbol,
            visibility: func.visibility,
            file: self.current_file.clone(),
            source: self.current_source.clone()
        })
    }

    fn analyze_block(&mut self, block: &Block) -> Result<TypedBlock, Error> {
        self.context.push_scope(ScopeKind::Local);
        let mut body = Vec::new();
        for stmt in &block.stmts {
            body.push(self.analyze_stmt(&stmt)?);
        }
        self.context.pop_scope();
        Ok(TypedBlock { body })
    }

    fn analyze_stmt(&mut self, stmt: &Statement) -> Result<TypedStatement, Error> {
        let typed_stmt = match stmt {
            Statement::Expression(expr) => TypedStatement::Expression(self.analyze_expr(expr, None)?),
            Statement::Var { ident, ty, val, is_const } => {
                let typed_val = self.analyze_expr(&val, ty.clone().map(|t| t.node))?;
                let var_ty = if let Some(t) = ty {
                    t.clone()
                } else {
                    Spanned::new(typed_val.node.infer_type(), typed_val.span)
                };
                let is_const = *is_const;
                self.context.add_var(&ident, var_ty.node, !is_const);
                TypedStatement::Var {
                    ident: ident.clone(),
                    ty: var_ty,
                    val: typed_val,
                    is_const
                }
            },
            Statement::Assign { ident, val } => {
                let symbol = self.context.get_var(&ident.node)
                    .ok_or_else(|| self.error(
                        ident.span,
                        &format!("Undefined variable '{}'", ident.node),
                        None
                    ))?;
                if !symbol.mutable {
                    return Err(self.error(
                        ident.span,
                        &format!("Assign to const '{}'", ident.node),
                        None
                    ));
                }
                TypedStatement::Assign {
                    ident: ident.clone(),
                    val: self.analyze_expr(&val, Some(symbol.ty))?,
                    symbol: symbol.id,
                    var_ty: symbol.ty
                }
            },
            Statement::If { condition, then_br, else_br } => {
                let typed_else = if let Some(b) = else_br {
                    Some(self.analyze_block(&b)?)
                } else {
                    None
                };
                TypedStatement::If {
                    condition: self.analyze_expr(&condition, None)?,
                    then_br: self.analyze_block(&then_br)?,
                    else_br: typed_else
                }
            },
            Statement::While { condition, body } => {
                self.context.push_scope(ScopeKind::Loop);
                let typed_body = self.analyze_block(&body)?;
                self.context.pop_scope();
                TypedStatement::While {
                    condition: self.analyze_expr(&condition, None)?,
                    body: typed_body
                }
            },
            Statement::Continue(span) => {
                if !self.context.in_loop() {
                    return Err(self.error(
                        *span,
                        "Continue outside of loop",
                        Some("Remove the continue statement")
                    ));
                }
                TypedStatement::Continue(*span)
            },
            Statement::Break(span) => {
                if !self.context.in_loop() {
                    return Err(self.error(
                        *span,
                        "Break outside of loop",
                        Some("Remove the break statement")
                    ));
                }
                TypedStatement::Break(*span)
            },
            Statement::Return(expr, span) => {
                let typed_expr = if let Some(e) = expr {
                    let ty = self.context.get_func(self.current_func.as_ref().unwrap()).unwrap().ty;
                    Some(self.analyze_expr(&e, Some(ty))?)
                } else {
                    None
                };
                TypedStatement::Return(typed_expr, *span)
            },
            _ => todo!()
        };
        Ok(typed_stmt)
    }

    fn analyze_expr(&self, expr: &Spanned<Expression>, expected_ty: Option<Type>) -> Result<Spanned<TypedExpression>, Error> {
        let typed_expr = match &expr.node {
            Expression::Integer(val) => {
                let ty = match expected_ty {
                    Some(Type::I64) => Type::I64, Some(Type::U64) => Type::U64,
                    Some(Type::I32) => Type::I32, Some(Type::U32) => Type::U32,
                    Some(Type::I16) => Type::I16, Some(Type::U16) => Type::U16,
                    Some(Type::I8) => Type::I8, Some(Type::U8) => Type::U8,
                    _ => Type::I32
                };
                TypedExpression::Integer { val: *val, ty }
            },
            Expression::Float(val) => {
                let ty = match expected_ty {
                    Some(Type::F64) => Type::F64,
                    Some(Type::F32) => Type::F32,
                    _ => Type::F64
                };
                TypedExpression::Float { val: *val, ty }
            },
            Expression::Bool(val) => TypedExpression::Bool(*val),
            Expression::Char(val) => TypedExpression::Char(*val),
            Expression::String(val) => TypedExpression::String(val.clone()),
            Expression::Identifier(ident) => {
                let symbol = self.context.get_var(&ident)
                    .ok_or_else(|| self.error(
                        expr.span,
                        &format!("Undefined variable '{}'", ident),
                        None
                    ))?;
                TypedExpression::Identifier { name: ident.clone(), ty: symbol.ty, symbol: symbol.id }
            },
            Expression::BinOp { left, op, right } => {
                let typed_left = self.analyze_expr(left, expected_ty)?;
                let left_ty = typed_left.node.infer_type();
                let typed_right = self.analyze_expr(right, Some(left_ty))?;
                let ty = match op {
                    BinOp::Plus | BinOp::Minus | BinOp::Multiply | BinOp::Divide => left_ty,
                    _ => Type::Bool
                };
                TypedExpression::BinOp { left: Box::new(typed_left), op: *op, right: Box::new(typed_right), ty }
            },
            Expression::UnOp { op, operand } => {
                let typed_operand = self.analyze_expr(&operand, expected_ty)?;
                let ty = typed_operand.node.infer_type();
                TypedExpression::UnOp { op: *op, operand: Box::new(typed_operand), ty }
            },
            Expression::Call { ident, args } => {
                let symbol = self.context.find_func(&ident.node, self.current_module.as_ref().unwrap())
                    .ok_or_else(|| self.error(
                        ident.span,
                        &format!("Undefined function '{}'", ident.node),
                        None
                    ))?;
                let is_var_args = symbol.params.last() == Some(&Type::Ellipsis);
                let params_count = if is_var_args {
                    symbol.params.len() - 1
                } else {
                    symbol.params.len()
                };
                if args.len() < params_count || (!is_var_args && args.len() > params_count) {
                    return Err(self.error(
                        ident.span,
                        &format!("Function {} takes {} args, but gets {}", ident.node, params_count, args.len()),
                        None
                    ))
                }
                let mut typed_args = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    let expected_ty = if i < params_count {
                        Some(symbol.params[i])
                    } else {
                        None
                    };
                    typed_args.push(self.analyze_expr(arg, expected_ty)?);
                }
                TypedExpression::Call {
                    ident: ident.clone(),
                    mangled: symbol.mangled.clone(),
                    ty: symbol.ty,
                    symbol: symbol.id,
                    args: typed_args
                }
            },
            Expression::As { expr, ty } => TypedExpression::As {
                expr: Box::new(self.analyze_expr(expr, expected_ty)?),
                ty: ty.clone()
            }
        };
        Ok(Spanned::new(typed_expr, expr.span))
    }
}
