use crate::frontend::Spanned;
use crate::frontend::ast::{Type, BinOp, UnOp};
use crate::frontend::sema::typed_ast::*;
use crate::backend::llvm::context::CodegenContext;
use inkwell::context::Context;
use inkwell::types::{BasicType, BasicTypeEnum, BasicMetadataTypeEnum};
use inkwell::values::{BasicValue, BasicValueEnum, BasicMetadataValueEnum, IntValue};
use inkwell::{IntPredicate, FloatPredicate};

pub struct Codegen<'ctx> {
    context: CodegenContext<'ctx>
}

impl<'ctx> Codegen<'ctx> {
    pub fn new(context: &'ctx Context) -> Codegen<'ctx> {
        Codegen { context: CodegenContext::new(context) }
    }

    pub fn into_context(self) -> CodegenContext<'ctx> {
        self.context
    }

    pub fn expr_to_bool(&self, expr: &Spanned<TypedExpression>) -> IntValue<'ctx> {
        match self.compile_expr(expr).unwrap() {
            BasicValueEnum::IntValue(val) => self.context.builder.build_int_compare(
                IntPredicate::NE,
                val,
                val.get_type().const_int(0, false),
                "int_to_bool"
            )
                .unwrap(),
            BasicValueEnum::FloatValue(val) => self.context.builder.build_float_compare(
                FloatPredicate::ONE,
                val,
                val.get_type().const_float(0.0),
                "float_to_bool"
            )
                .unwrap(),
            _ => unreachable!()
        }
    }

    pub fn compile_program(&mut self, program: &TypedProgram) {
        for item in &program.items {
            match &item {
                TypedItem::Extern(ext) => self.declare_extern(&ext),
                TypedItem::Function(func) => self.declare_function(&func),
                _ => ()
            }
        }
        for item in &program.items {
            match &item {
                TypedItem::Function(func) => self.compile_function(&func),
                _ => ()
            }
        }
        self.compile_main_wrapper();
    }

    fn declare_extern(&mut self, ext: &TypedExtern) {
        if let Some(&f) = self.context.externs.get(&ext.name) {
            self.context.functions.insert(ext.mangled.clone(), f);
            return;
        }
        let param_types: Vec<BasicMetadataTypeEnum> = ext.params.iter()
            .filter(|p| p.node.ty.node != Type::Ellipsis)
            .map(|p| self.context.type_to_llvm(&p.node.ty.node).unwrap().into())
            .collect();
        let is_var_args = ext.params.iter().any(|p| p.node.ty.node == Type::Ellipsis);
        let ty = self.context.type_to_llvm(&ext.ret_type.node)
            .map_or_else(
                || self.context.context.void_type().fn_type(&param_types, is_var_args),
                |t| t.fn_type(&param_types, is_var_args)
            );
        let val = self.context.module.add_function(
            &ext.name,
            ty,
            None
        );
        self.context.externs.insert(ext.name.clone(), val);
        self.context.functions.insert(ext.mangled.clone(), val);
    }

    fn declare_function(&mut self, func: &TypedFunction) {
        let param_types: Vec<BasicMetadataTypeEnum> = func.params.iter()
            .filter(|p| p.node.ty.node != Type::Ellipsis)
            .map(|p| self.context.type_to_llvm(&p.node.ty.node).unwrap().into())
            .collect();
        let is_var_args = func.params.iter().any(|p| p.node.ty.node == Type::Ellipsis);
        let ty = self.context.type_to_llvm(&func.ret_type.node)
            .map_or_else(
                || self.context.context.void_type().fn_type(&param_types, is_var_args),
                |t| t.fn_type(&param_types, is_var_args)
            );
        let val = self.context.module.add_function(
            &func.mangled,
            ty,
            None
        );
        self.context.functions.insert(func.mangled.clone(), val);
    }

    fn compile_function(&mut self, func: &TypedFunction) {
        let func_val = self.context.functions[&func.mangled];
        let entry = self.context.context.append_basic_block(func_val, "entry");
        self.context.builder.position_at_end(entry);
        self.context.current_func = Some(func_val);
        for (i, param) in func.params.iter().filter(|p| p.node.ty.node != Type::Ellipsis).enumerate() {
            let param_val = func_val.get_nth_param(i as u32).unwrap();
            let alloca = self.context.builder.build_alloca(param_val.get_type(), &param.node.name).unwrap();
            self.context.builder.build_store(alloca, param_val).unwrap();
            self.context.variables.insert(param.node.name.clone(), alloca);
        }
        self.compile_block(&func.body);
        if func.ret_type.node == Type::Void && !self.context.is_block_terminated() {
            self.context.builder.build_return(None).unwrap();
        }
        self.context.current_func = None;
    }

    fn compile_main_wrapper(&mut self) {
        let main_func = self.context.functions["main.main"];
        let wrapper_func = self.context.module.add_function(
            "main",
            self.context.context.i32_type().fn_type(&[], false),
            None
        );
        let entry = self.context.context.append_basic_block(wrapper_func, "entry");
        self.context.builder.position_at_end(entry);
        self.context.builder.build_call(main_func, &[], "main_call").unwrap();
        self.context.builder.build_return(
            Some(&self.context.context.i32_type().const_int(0, false) as &dyn BasicValue)
        )
            .unwrap();
    }

    fn compile_block(&mut self, block: &TypedBlock) {
        for stmt in &block.body {
            self.compile_stmt(&stmt);
            if self.context.is_block_terminated() {
                break;
            }
        }
    }

    fn compile_stmt(&mut self, stmt: &TypedStatement) {
        match stmt {
            TypedStatement::Expression(expr) => {
                self.compile_expr(&expr);
            },
            TypedStatement::Var { ident, ty, val, .. } => {
                let compiled_val = self.compile_expr(&val).unwrap();
                let alloca = self.context.builder.build_alloca(self.context.type_to_llvm(&ty.node).unwrap(), ident).unwrap();
                self.context.builder.build_store(alloca, compiled_val).unwrap();
                self.context.variables.insert(ident.clone(), alloca);
            },
            TypedStatement::Assign { ident, val, .. } => {
                let alloca = self.context.variables[&ident.node];
                let compiled_val = self.compile_expr(&val).unwrap();
                self.context.builder.build_store(alloca, compiled_val).unwrap();
            },
            TypedStatement::If { condition, then_br, else_br } => {
                let current_func = self.context.current_func.unwrap();
                let compiled_cond = self.expr_to_bool(condition);
                let then_block = self.context.context.append_basic_block(current_func, "if_then_block");
                let merge_block = self.context.context.append_basic_block(current_func, "if_merge_block");
                let else_block = else_br.as_ref()
                    .map(|_| self.context.context.append_basic_block(current_func, "if_else_block"));
                self.context.builder.build_conditional_branch(compiled_cond, then_block, else_block.unwrap_or(merge_block)).unwrap();
                self.context.builder.position_at_end(then_block);
                self.compile_block(&then_br);
                let then_terminated = self.context.is_block_terminated();
                if !then_terminated {
                    self.context.builder.build_unconditional_branch(merge_block).unwrap();
                }
                let else_terminated = match (else_block, else_br) {
                    (Some(block), Some(br)) => {
                        self.context.builder.position_at_end(block);
                        self.compile_block(&br);
                        let terminated = self.context.is_block_terminated();
                        if !terminated {
                            self.context.builder.build_unconditional_branch(merge_block).unwrap();
                        }
                        terminated
                    },
                    _ => false
                };
                if then_terminated && else_terminated {
                    let unreachable_block = self.context.context.append_basic_block(current_func, "unreachable");
                    self.context.builder.position_at_end(unreachable_block);
                    self.context.builder.build_unreachable().unwrap();
                } else {
                    self.context.builder.position_at_end(merge_block);
                }
            },
            TypedStatement::While { condition, body } => {
                let current_func = self.context.current_func.unwrap();
                let cond_block = self.context.context.append_basic_block(current_func, "while_cond_block");
                let body_block = self.context.context.append_basic_block(current_func, "while_body_block");
                let merge_block = self.context.context.append_basic_block(current_func, "while_merge_block");
                let old_continue_block = self.context.continue_block;
                let old_break_block = self.context.break_block;
                self.context.continue_block = Some(cond_block);
                self.context.break_block = Some(merge_block);
                self.context.builder.build_unconditional_branch(cond_block).unwrap();
                self.context.builder.position_at_end(cond_block);
                let compiled_cond = self.expr_to_bool(condition);
                self.context.builder.build_conditional_branch(compiled_cond, body_block, merge_block).unwrap();
                self.context.builder.position_at_end(body_block);
                self.compile_block(body);
                if !self.context.is_block_terminated() {
                    self.context.builder.build_unconditional_branch(cond_block).unwrap();
                }
                self.context.builder.position_at_end(merge_block);
                self.context.continue_block = old_continue_block;
                self.context.break_block = old_break_block;
            },
            TypedStatement::Continue(_) => {
                self.context.builder.build_unconditional_branch(self.context.continue_block.unwrap()).unwrap();
            },
            TypedStatement::Break(_) => {
                self.context.builder.build_unconditional_branch(self.context.break_block.unwrap()).unwrap();
            },
            TypedStatement::Return(expr, ..) => {
                let compiled_expr = if let Some(e) = expr {
                    Some(&self.compile_expr(&e).unwrap() as &dyn BasicValue)
                } else {
                    None
                };
                self.context.builder.build_return(compiled_expr).unwrap();
            }
        }
    }

    fn compile_expr(&self, expr: &Spanned<TypedExpression>) -> Option<BasicValueEnum<'ctx>> {
        let compiled_expr = match &expr.node {
            TypedExpression::Integer { val, ty } => {
                let basic_ty = match ty {
                    Type::I64 | Type::U64 => self.context.context.i64_type(),
                    Type::I32 | Type::U32 => self.context.context.i32_type(),
                    Type::I16 | Type::U16 => self.context.context.i16_type(),
                    Type::I8 | Type::U8 => self.context.context.i8_type(),
                    _ => unreachable!()
                };
                let sign_extend = matches!(ty, Type::I64 | Type::I32 | Type::I16 | Type::I8);
                basic_ty.const_int(*val as u64, sign_extend).into()
            },
            TypedExpression::Float { val, ty } => {
                let basic_ty = match ty {
                    Type::F64 => self.context.context.f64_type(),
                    Type::F32 => self.context.context.f32_type(),
                    _ => unreachable!()
                };
                basic_ty.const_float(*val).into()
            },
            TypedExpression::Bool(val) => self.context.context.bool_type()
                .const_int(*val as u64, false)
                .into(),
            TypedExpression::Char(val) => self.context.context.i8_type()
                .const_int(*val as u64, false)
                .into(),
            TypedExpression::String(val) => self.context.builder.build_global_string_ptr(val, "str").unwrap()
                .as_pointer_value()
                .into(),
            TypedExpression::Identifier { name, ty, .. } => self.context.builder.build_load(
                self.context.type_to_llvm(ty).unwrap(),
                self.context.variables[name],
                &format!("load_of_{}", name)
            )
                .unwrap()
                .into(),
            TypedExpression::BinOp { left, op, right, ty } => self.compile_binop(left, *op, right, *ty),
            TypedExpression::UnOp { op, operand, .. } => match op {
                UnOp::Negate => match self.compile_expr(operand).unwrap() {
                    BasicValueEnum::IntValue(val) => self.context.builder.build_int_neg(val, "int_neg").unwrap().into(),
                    BasicValueEnum::FloatValue(val) => self.context.builder.build_float_neg(val, "float_neg").unwrap().into(),
                    _ => unreachable!()
                },
                UnOp::Not => self.context.builder.build_not(
                    self.expr_to_bool(operand),
                    "not"
                )
                    .unwrap()
                    .into()
            },
            TypedExpression::Call { mangled, args, .. } => return self.context.builder.build_call(
                self.context.functions[mangled],
                &args.iter().map(|a| self.compile_expr(a).unwrap().into()).collect::<Vec<BasicMetadataValueEnum>>(),
                "call"
            )
                .unwrap()
                .try_as_basic_value()
                .basic(),
            TypedExpression::As { expr, ty } => self.compile_cast(expr, ty)
        };
        Some(compiled_expr)
    }

    fn compile_binop(&self, left: &Spanned<TypedExpression>, op: BinOp, right: &Spanned<TypedExpression>, ty: Type) -> BasicValueEnum<'ctx> {
        if matches!(op, BinOp::And | BinOp::Or) {
            let current_func = self.context.current_func.unwrap();
            let current_block = self.context.builder.get_insert_block().unwrap();
            let left_cond = self.expr_to_bool(left);
            let right_block = self.context.context.append_basic_block(current_func, "logical_right_block");
            let merge_block = self.context.context.append_basic_block(current_func, "logical_merge_block");
            match op {
                BinOp::And => self.context.builder.build_conditional_branch(left_cond, right_block, merge_block),
                BinOp::Or => self.context.builder.build_conditional_branch(left_cond, merge_block, right_block),
                _ => unreachable!()
            }
                .unwrap();
            self.context.builder.position_at_end(right_block);
            let right_cond = self.expr_to_bool(right);
            let right_block_end = self.context.builder.get_insert_block().unwrap();
            self.context.builder.build_unconditional_branch(merge_block).unwrap();
            self.context.builder.position_at_end(merge_block);
            let phi = self.context.builder.build_phi(self.context.context.bool_type(), "logical_phi").unwrap();
            match op {
                BinOp::And => phi.add_incoming(&[(
                    &self.context.context.bool_type().const_int(0, false),
                    current_block
                )]),
                BinOp::Or => phi.add_incoming(&[(
                    &self.context.context.bool_type().const_int(1, false),
                    current_block
                )]),
                _ => unreachable!()
            }
            phi.add_incoming(&[(
                &right_cond,
                right_block_end
            )]);
            return phi.as_basic_value();
        }
        let compiled_left = self.compile_expr(left).unwrap();
        let compiled_right = self.compile_expr(right).unwrap();
        let signed = matches!(ty, Type::I64 | Type::I32 | Type::I16 | Type::I8);
        match (compiled_left, compiled_right) {
            (BasicValueEnum::IntValue(left_val), BasicValueEnum::IntValue(right_val)) => match op {
                BinOp::Plus => self.context.builder.build_int_add(left_val, right_val, "int_add"),
                BinOp::Minus => self.context.builder.build_int_sub(left_val, right_val, "int_sub"),
                BinOp::Multiply => self.context.builder.build_int_mul(left_val, right_val, "int_mul"),
                BinOp::Divide => if signed {
                    self.context.builder.build_int_signed_div(left_val, right_val, "int_div")
                } else {
                    self.context.builder.build_int_unsigned_div(left_val, right_val, "uint_div")
                },
                BinOp::Eq => self.context.builder.build_int_compare(IntPredicate::EQ, left_val, right_val, "int_eq"),
                BinOp::Greater => if signed {
                    self.context.builder.build_int_compare(IntPredicate::SGT, left_val, right_val, "int_sgt")
                } else {
                    self.context.builder.build_int_compare(IntPredicate::UGT, left_val, right_val, "uint_ugt")
                },
                BinOp::Lower => if signed {
                    self.context.builder.build_int_compare(IntPredicate::SLT, left_val, right_val, "int_slt")
                } else {
                    self.context.builder.build_int_compare(IntPredicate::ULT, left_val, right_val, "uint_ult")
                },
                BinOp::GreaterEq => if signed {
                    self.context.builder.build_int_compare(IntPredicate::SGE, left_val, right_val, "int_sge")
                } else {
                    self.context.builder.build_int_compare(IntPredicate::UGE, left_val, right_val, "uint_uge")
                },
                BinOp::LowerEq => if signed {
                    self.context.builder.build_int_compare(IntPredicate::SLE, left_val, right_val, "int_sle")
                } else {
                    self.context.builder.build_int_compare(IntPredicate::ULE, left_val, right_val, "uint_ule")
                },
                BinOp::NotEq => self.context.builder.build_int_compare(IntPredicate::NE, left_val, right_val, "int_ne"),
                _ => unreachable!()
            }
                .unwrap()
                .into(),
            (BasicValueEnum::FloatValue(left_val), BasicValueEnum::FloatValue(right_val)) => match op {
                BinOp::Plus => self.context.builder.build_float_add(left_val, right_val, "float_add").unwrap().into(),
                BinOp::Minus => self.context.builder.build_float_sub(left_val, right_val, "float_sub").unwrap().into(),
                BinOp::Multiply => self.context.builder.build_float_mul(left_val, right_val, "float_mul").unwrap().into(),
                BinOp::Divide => self.context.builder.build_float_div(left_val, right_val, "float_div").unwrap().into(),
                BinOp::Eq => self.context.builder.build_float_compare(FloatPredicate::OEQ, left_val, right_val, "float_oeq").unwrap().into(),
                BinOp::Greater => self.context.builder.build_float_compare(FloatPredicate::OGT, left_val, right_val, "float_ogt").unwrap().into(),
                BinOp::Lower => self.context.builder.build_float_compare(FloatPredicate::OLT, left_val, right_val, "float_olt").unwrap().into(),
                BinOp::GreaterEq => self.context.builder.build_float_compare(FloatPredicate::OGE, left_val, right_val, "float_oge").unwrap().into(),
                BinOp::LowerEq => self.context.builder.build_float_compare(FloatPredicate::OLE, left_val, right_val, "float_ole").unwrap().into(),
                BinOp::NotEq => self.context.builder.build_float_compare(FloatPredicate::ONE, left_val, right_val, "float_ne").unwrap().into(),
                _ => unreachable!()
            }
            _ => unreachable!()
        }
    }

    fn compile_cast(&self, expr: &Spanned<TypedExpression>, ty: &Spanned<Type>) -> BasicValueEnum<'ctx> {
        let compiled_expr = self.compile_expr(expr).unwrap();
        let target_ty = self.context.type_to_llvm(&ty.node).unwrap();
        if compiled_expr.get_type() == target_ty {
            return compiled_expr;
        }
        let from_signed = matches!(expr.node.infer_type(), Type::I64 | Type::I32 | Type::I16 | Type::I8);
        let to_signed = matches!(ty.node, Type::I64 | Type::I32 | Type::I16 | Type::I8);
        match target_ty {
            BasicTypeEnum::IntType(int_ty) => match compiled_expr {
                BasicValueEnum::IntValue(val) => if val.get_type().get_bit_width() > int_ty.get_bit_width() {
                    self.context.builder.build_int_truncate(val, int_ty, "int_trunc")
                } else {
                    if from_signed {
                        self.context.builder.build_int_s_extend(val, int_ty, "int_s_ext")
                    } else {
                        self.context.builder.build_int_z_extend(val, int_ty, "uint_z_ext")
                    }
                }
                    .unwrap()
                    .into(),
                BasicValueEnum::FloatValue(val) => if to_signed {
                    self.context.builder.build_float_to_signed_int(val, int_ty, "float_to_int")
                } else {
                    self.context.builder.build_float_to_unsigned_int(val, int_ty, "float_to_uint")
                }
                    .unwrap()
                    .into(),
                _ => unreachable!()
            },
            BasicTypeEnum::FloatType(float_ty) => match compiled_expr {
                BasicValueEnum::IntValue(val) => if from_signed {
                    self.context.builder.build_signed_int_to_float(val, float_ty, "int_to_float")
                } else {
                    self.context.builder.build_unsigned_int_to_float(val, float_ty, "uint_to_float")
                }
                    .unwrap()
                    .into(),
                BasicValueEnum::FloatValue(val) => if val.get_type().get_bit_width() > float_ty.get_bit_width() {
                    self.context.builder.build_float_trunc(val, float_ty, "float_trunc")
                } else {
                    self.context.builder.build_float_ext(val, float_ty, "float_ext")
                }
                    .unwrap()
                    .into(),
                _ => unreachable!()
            },
            _ => unreachable!()
        }
    }
}
