mod modules;

use crate::errors::{Spanned, Error};
use crate::parser::ast::*;

use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::OptimizationLevel;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate};
use std::collections::HashMap;

pub struct Variable<'ctx> {
    pub ptr: PointerValue<'ctx>,
    pub ty: BasicTypeEnum<'ctx>,
    pub is_const: bool
}

pub struct Codegen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub functions: HashMap<String, FunctionValue<'ctx>>,
    pub current_function: Option<FunctionValue<'ctx>>,
    pub variables: HashMap<String, Variable<'ctx>>,
}

impl<'ctx> Codegen<'ctx> {
    pub fn new(context: &'ctx Context) -> Codegen<'ctx> {
        Codegen {
            context,
            module: context.create_module("main"),
            builder: context.create_builder(),
            functions: HashMap::new(),
            current_function: None,
            variables: HashMap::new()
        }
    }

    fn is_current_block_terminated(&self) -> bool {
        self.builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_some()
    }

    fn type_to_basic(&self, ty: &Spanned<Type>) -> Result<BasicTypeEnum<'ctx>, Error> {
        let basic = match ty.node {
            Type::I64 | Type::U64 => self.context.i64_type().into(),
            Type::I32 | Type::U32 => self.context.i32_type().into(),
            Type::I16 | Type::U16 => self.context.i16_type().into(),
            Type::I8 | Type::U8 => self.context.i8_type().into(),
            Type::F64 => self.context.f64_type().into(),
            Type::F32 => self.context.f32_type().into(),
            Type::Bool => self.context.bool_type().into(),
            Type::Str => self.context.ptr_type(AddressSpace::default()).into(),
            _ => return Err(Error::UnexpectedType { expected: vec![
                Type::I64, Type::I32, Type::I16, Type::I8,
                Type::U64, Type::U32, Type::U16, Type::U8,
                Type::F64, Type::F32,
                Type::Bool,
                Type::Str
            ], got: ty.node.clone(), span: ty.span })
        };
        Ok(basic)
    }

    fn basic_to_type(&self, llvm: Option<BasicTypeEnum<'ctx>>) -> Type {
        match llvm {
            Some(BasicTypeEnum::IntType(t)) if t == self.context.bool_type() => Type::Bool,
            Some(BasicTypeEnum::IntType(_)) => Type::I32,
            Some(BasicTypeEnum::FloatType(_)) => Type::F64,
            Some(BasicTypeEnum::PointerType(_)) => Type::Str,
            None => Type::Void,
            _ => unreachable!()
        }
    }

    fn expr_to_type(&self, expr: &Spanned<Expression>) -> Result<Type, Error> {
        let ty = match &expr.node {
            Expression::Integer(_) => Type::I32,
            Expression::Float(_) => Type::F64,
            Expression::Bool(_) => Type::Bool,
            Expression::String(_) => Type::Str,
            Expression::Identifier(ident) => self.basic_to_type(
                Some(self.variables.get(ident).ok_or_else(|| Error::UndefinedVariable { ident: ident.clone(), span: expr.span })?.ty)
            ),
            Expression::BinOp { left, right, .. } => {
                let left_ty = self.expr_to_type(left)?;
                let right_ty = self.expr_to_type(right)?;
                if left_ty == right_ty { left_ty } else {
                    return Err(Error::UnexpectedType { expected: vec![left_ty], got: right_ty, span: right.span })
                }
            },
            Expression::UnOp { operand, .. } => self.expr_to_type(operand)?,
            Expression::Call { ident, .. } => self.basic_to_type(
                self.functions.get(&ident.node).ok_or_else(|| Error::UndefinedFunction { ident: ident.node.clone(), span: expr.span })?.get_type().get_return_type()
            ),
            Expression::As { ty, .. } => ty.node.clone()
        };
        Ok(ty)
    }

    fn expr_to_bool(&mut self, expr: &Spanned<Expression>, name: &str) -> Result<IntValue<'ctx>, Error> {
        let val = self.compile_some_expr(expr)?;
        match val {
            BasicValueEnum::IntValue(l ) => {
                self.builder.build_int_compare(IntPredicate::NE, l, l.get_type().const_int(0, false), name)
                    .map_err(|e| Error::LLVMError { error: e.to_string() })
            },
            BasicValueEnum::FloatValue(l) => {
                self.builder.build_float_compare(FloatPredicate::ONE, l, l.get_type().const_float(0.0), name)
                    .map_err(|e| Error::LLVMError { error: e.to_string() })
            },
            _ => Err(Error::UnexpectedType { expected: vec![
                Type::I64, Type::I32, Type::I16, Type::I8,
                Type::U64, Type::U32, Type::U16, Type::U8,
                Type::F64, Type::F32
            ], got: self.expr_to_type(expr)?, span: expr.span })
        }
    }

    pub fn compile_program(&mut self, program: Program) -> Result<(), Error> {
        self.compile_imports(&program.imports)?;
        for fn_extern in &program.externs {
            self.declare_extern(fn_extern)?;
        }
        for function in &program.functions {
            self.declare_function(function)?;
        }
        for function in &program.functions {
            self.compile_function(function)?;
        }
        Ok(())
    }

    pub fn run_jit(&self) -> Result<(), Error> {
        type MainFn = unsafe extern "C" fn();
        let engine = self.module.create_jit_execution_engine(OptimizationLevel::Default)
            .map_err(|e| Error::LLVMError { error: e.to_string() })?;
        unsafe {
            engine.get_function::<MainFn>("main")
                .map_err(|e| Error::LLVMError { error: e.to_string() })?
                .call();
        }
        Ok(())
    }

    fn compile_imports(&mut self, imports: &Vec<Import>) -> Result<(), Error> {
        let mut module_system = modules::ModuleSystem::new();
        for import in imports {
            module_system.resolve(&import.path)?;
        }
        for module in module_system.modules {
            if !module.imports.is_empty() {
                self.compile_imports(&module.imports)?;
            }
            for fn_extern in &module.externs {
                self.declare_extern(fn_extern)?;
            }
            for function in &module.functions {
                self.declare_function(function)?;
            }
            for function in &module.functions {
                self.compile_function(function)?;
            }
        }
        Ok(())
    }

    fn declare_extern(&mut self, fn_extern: &Extern) -> Result<(), Error> {
        let mut param_types = Vec::new();
        let mut is_var_args = false;
        for param in &fn_extern.params {
            if param.ty.node == Type::Ellipsis {
                is_var_args = true;
                continue;
            }
            param_types.push(self.type_to_basic(&param.ty)?.into());
        }
        let fn_type = if fn_extern.ret_type.node == Type::Void {
            self.context.void_type().fn_type(&param_types, is_var_args)
        } else {
            self.type_to_basic(&fn_extern.ret_type)?.fn_type(&param_types, is_var_args)
        };
        let fn_val = self.module.add_function(fn_extern.name.as_str(), fn_type, None);
        self.functions.insert(fn_extern.name.clone(), fn_val);
        Ok(())
    }

    fn declare_function(&mut self, function: &Function) -> Result<(), Error> {
        let mut param_types = Vec::new();
        let mut is_var_args = false;
        for param in &function.params {
            if param.ty.node == Type::Ellipsis {
                is_var_args = true;
                break;
            }
            param_types.push(self.type_to_basic(&param.ty)?.into());
        }
        let fn_type = if function.ret_type.node == Type::Void {
            self.context.void_type().fn_type(&param_types, is_var_args)
        } else {
            self.type_to_basic(&function.ret_type)?.fn_type(&param_types, is_var_args)
        };
        let fn_val = self.module.add_function(function.name.as_str(), fn_type, None);
        self.functions.insert(function.name.clone(), fn_val);
        Ok(())
    }

    fn compile_function(&mut self, function: &Function) -> Result<(), Error> {
        let fn_val = self.functions[function.name.as_str()];
        self.current_function = Some(fn_val);
        self.variables.clear();
        let entry = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry);
        for (i, param) in function.params.iter().enumerate() {
            if param.ty.node == Type::Ellipsis {
                break;
            }
            let val = fn_val.get_nth_param(i as u32).unwrap();
            let ptr = self.builder.build_alloca(val.get_type(), param.name.as_str())
                .map_err(|e| Error::LLVMError { error: e.to_string() })?;
            self.builder.build_store(ptr, val)
                .map_err(|e| Error::LLVMError { error: e.to_string() })?;
            self.variables.insert(param.name.clone(), Variable { ptr, ty: self.type_to_basic(&param.ty)?, is_const: false });
        }
        for stmt in &function.body.stmts {
            self.compile_statement(stmt)?;
            if self.is_current_block_terminated() {
                return Ok(())
            }
        }
        match function.ret_type.node {
            Type::Void => { self.builder.build_return(None)
                .map_err(|e| Error::LLVMError { error: e.to_string() })?; },
            _ => return Err(Error::UnexpectedType {
                expected: vec![function.ret_type.node.clone()],
                got: Type::Void,
                span: function.ret_type.span
            })
        }
        Ok(())
    }

    fn compile_statement(&mut self, statement: &Statement) -> Result<(), Error> {
        match statement {
            Statement::Expression(expr) => { self.compile_expr(expr)?; }
            Statement::Var { ident, ty, val } => self.compile_variable(ident, ty, val, false)?,
            Statement::Const { ident, ty, val } => self.compile_variable(ident, ty, val, true)?,
            Statement::Assign { ident, val } => {
                let compiled_val = self.compile_some_expr(val)?;
                let var = self.variables.get(&ident.node)
                    .ok_or_else(|| Error::UndefinedVariable { ident: ident.node.clone(), span: ident.span })?;
                if var.is_const {
                    return Err(Error::InvalidAssignment { ident: ident.node.clone(), span: ident.span });
                }
                self.builder.build_store(var.ptr, compiled_val)
                    .map_err(|e| Error::LLVMError { error: e.to_string() })?;
            },
            Statement::If { condition, then_br, else_br } => {
                let current_function = self.current_function.unwrap();
                let compiled_cond = self.expr_to_bool(condition, "if_cond")?;
                let then_block = self.context.append_basic_block(current_function, "if_then_block");
                let merge_block = self.context.append_basic_block(current_function, "if_merge_block");
                let else_terminated = match else_br {
                    Some(br) => {
                        let else_block = self.context.append_basic_block(current_function, "if_else_block");
                        self.builder.build_conditional_branch(compiled_cond, then_block, else_block)
                            .map_err(|e| Error::LLVMError { error: e.to_string() })?;
                        self.builder.position_at_end(else_block);
                        for stmt in &br.stmts {
                            self.compile_statement(stmt)?;
                        }
                        let terminated = self.is_current_block_terminated();
                        if !terminated {
                            self.builder.build_unconditional_branch(merge_block)
                                .map_err(|e| Error::LLVMError { error: e.to_string() })?;
                        }
                        terminated
                    },
                    None => {
                        self.builder.build_conditional_branch(compiled_cond, then_block, merge_block)
                            .map_err(|e| Error::LLVMError { error: e.to_string() })?;
                        false
                    }
                };
                self.builder.position_at_end(then_block);
                for stmt in &then_br.stmts {
                    self.compile_statement(stmt)?;
                }
                let then_terminated = self.is_current_block_terminated();
                if !then_terminated {
                    self.builder.build_unconditional_branch(merge_block)
                        .map_err(|e| Error::LLVMError { error: e.to_string() })?;
                }
                if then_terminated && else_terminated {
                    let unreachable_block = self.context.append_basic_block(current_function, "unreachable");
                    self.builder.position_at_end(unreachable_block);
                    self.builder.build_unreachable()
                        .map_err(|e| Error::LLVMError { error: e.to_string() })?;
                } else {
                    self.builder.position_at_end(merge_block);
                }
            },
            Statement::While { condition, body } => {
                let current_function = self.current_function.unwrap();
                let cond_block = self.context.append_basic_block(current_function, "while_cond_block");
                let do_block = self.context.append_basic_block(current_function, "while_do_block");
                let merge_block = self.context.append_basic_block(current_function, "while_merge_block");
                self.builder.build_unconditional_branch(cond_block)
                    .map_err(|e| Error::LLVMError { error: e.to_string() })?;
                self.builder.position_at_end(cond_block);
                let bool_cond = self.expr_to_bool(condition, "while_cond")?;
                self.builder.build_conditional_branch(bool_cond, do_block, merge_block)
                    .map_err(|e| Error::LLVMError { error: e.to_string() })?;
                self.builder.position_at_end(do_block);
                for stmt in &body.stmts {
                    self.compile_statement(stmt)?;
                    if self.is_current_block_terminated() {
                        break;
                    }
                }
                if !self.is_current_block_terminated() {
                    self.builder.build_unconditional_branch(cond_block)
                        .map_err(|e| Error::LLVMError { error: e.to_string() })?;
                }
                self.builder.position_at_end(merge_block);
            },
            Statement::Return(expr) => match expr {
                Some(e) => {
                    let val = self.compile_some_expr(e)?;
                    self.builder.build_return(Some(&val as &dyn BasicValue))
                        .map_err(|e| Error::LLVMError { error: e.to_string() })?;
                },
                None => { self.builder.build_return(None)
                    .map_err(|e| Error::LLVMError { error: e.to_string() })?; }
            }
            _ => todo!()
        }
        Ok(())
    }

    fn compile_variable(&mut self, ident: &str, ty: &Option<Spanned<Type>>, val: &Spanned<Expression>, is_const: bool) -> Result<(), Error> {
        let basic_ty = self.type_to_basic(
            &ty.clone().unwrap_or(Spanned::new(self.expr_to_type(val)?, val.span))
        )?;
        let val = self.compile_some_expr(val)?;
        let ptr = self.builder.build_alloca(basic_ty, ident)
            .map_err(|e| Error::LLVMError { error: e.to_string() })?;
        self.builder.build_store(ptr, val)
            .map_err(|e| Error::LLVMError { error: e.to_string() })?;
        self.variables.insert(ident.to_string(), Variable { ptr, ty: basic_ty, is_const });
        Ok(())
    }

    fn compile_expr(&mut self, expression: &Spanned<Expression>) -> Result<Option<BasicValueEnum<'ctx>>, Error> {
        let compiled: BasicValueEnum<'ctx> = match &expression.node {
            Expression::Integer(num) => self.context.i32_type().const_int(*num as u64, false).into(),
            Expression::Float(num) => self.context.f64_type().const_float(*num).into(),
            Expression::Bool(num) => self.context.bool_type().const_int(*num as u64, false).into(),
            Expression::String(string) => {
                let ptr = self.builder.build_global_string_ptr(string, "str")
                    .map_err(|e| Error::LLVMError { error: e.to_string() })?;
                ptr.as_pointer_value().into()
            },
            Expression::Identifier(ident) => {
                let var = self.variables.get(ident)
                    .ok_or_else(|| Error::UndefinedVariable { ident: ident.clone(), span: expression.span })?;
                self.builder.build_load(var.ty, var.ptr, "var")
                    .map_err(|e| Error::LLVMError { error: e.to_string() })?
            },
            Expression::BinOp { left, op, right } => {
                self.compile_binop(left, right, op)?
            },
            Expression::UnOp { op, operand } => {
                match op {
                    UnOp::Negate => {
                        let val = self.compile_some_expr(operand)?;
                        match val {
                            BasicValueEnum::IntValue(int) => self.builder.build_int_neg(int, "int_neg")
                                .map_err(|e| Error::LLVMError { error: e.to_string() })?
                                .into(),
                            BasicValueEnum::FloatValue(float) => self.builder.build_float_neg(float, "float_neg")
                                .map_err(|e| Error::LLVMError { error: e.to_string() })?
                                .into(),
                            _ => return Err(Error::UnexpectedType { expected: vec![
                                Type::I64, Type::I32, Type::I16, Type::I8,
                                Type::U64, Type::U32, Type::U16, Type::U8,
                                Type::F64, Type::F32
                            ], got: self.expr_to_type(operand)?, span: operand.span })
                        }
                    },
                    UnOp::Not => {
                        let bool_val = self.expr_to_bool(operand, "not_val")?;
                        self.builder.build_not(bool_val, "not")
                            .map_err(|e| Error::LLVMError { error: e.to_string() })?
                            .into()
                    }
                }
            },
            Expression::Call { ident, args } => {
                let func = self.functions.get(&ident.node)
                    .ok_or_else(|| Error::UndefinedFunction { ident: ident.node.clone(), span: ident.span })?
                    .clone();
                let mut compiled_args = Vec::new();
                for arg in args {
                    compiled_args.push(self.compile_some_expr(arg)?.into());
                }
                let call = self.builder.build_call(func, &compiled_args, "call")
                    .map_err(|e| Error::LLVMError { error: e.to_string() })?;
                return Ok(call.try_as_basic_value().basic())
            },
            Expression::As { expr, ty } => {
                let val = self.compile_some_expr(expr)?;
                let target_type = self.type_to_basic(ty)?;
                if val.get_type() == target_type {
                    return Ok(Some(val));
                }
                match target_type {
                    BasicTypeEnum::IntType(int_type) => match val {
                        BasicValueEnum::IntValue(int_value) => if int_value.get_type().get_bit_width() > int_type.get_bit_width() {
                            self.builder.build_int_truncate(int_value, int_type, "int_trunc")
                                .map_err(|e| Error::LLVMError { error: e.to_string() })?.into()
                        } else {
                            self.builder.build_int_s_extend(int_value, int_type, "int_sext")
                                .map_err(|e| Error::LLVMError { error: e.to_string() })?.into()
                        },
                        BasicValueEnum::FloatValue(float_value) => self.builder.build_float_to_signed_int(float_value, int_type, "int_to_float")
                            .map_err(|e| Error::LLVMError { error: e.to_string() })?.into(),
                        _ => return Err(Error::UnexpectedType { expected: vec![
                            Type::I64, Type::I32, Type::I16, Type::I8,
                            Type::U64, Type::U32, Type::U16, Type::U8,
                            Type::F64, Type::F32
                        ], got: self.expr_to_type(expr)?, span: expr.span })
                    },
                    BasicTypeEnum::FloatType(float_type) => match val {
                        BasicValueEnum::IntValue(int_value) => self.builder.build_signed_int_to_float(int_value, float_type, "float_to_int")
                            .map_err(|e| Error::LLVMError { error: e.to_string() })?.into(),
                        BasicValueEnum::FloatValue(float_value) => if float_value.get_type().get_bit_width() > float_type.get_bit_width() {
                            self.builder.build_float_trunc(float_value, float_type, "float_trunc")
                                .map_err(|e| Error::LLVMError { error: e.to_string() })?.into()
                        } else {
                            self.builder.build_float_ext(float_value, float_type, "float_ext")
                                .map_err(|e| Error::LLVMError { error: e.to_string() })?.into()
                        },
                        _ => return Err(Error::UnexpectedType { expected: vec![
                            Type::I64, Type::I32, Type::I16, Type::I8,
                            Type::U64, Type::U32, Type::U16, Type::U8,
                            Type::F64, Type::F32
                        ], got: self.expr_to_type(expr)?, span: expr.span })
                    },
                    _ => return Err(Error::UnexpectedType { expected: vec![
                        Type::I64, Type::I32, Type::I16, Type::I8,
                        Type::U64, Type::U32, Type::U16, Type::U8,
                        Type::F64, Type::F32
                    ], got: ty.node.clone(), span: ty.span })
                }
            }
        };
        Ok(Some(compiled))
    }

    fn compile_binop(&mut self, left_expr: &Spanned<Expression>, right_expr: &Spanned<Expression>, op: &BinOp) -> Result<BasicValueEnum<'ctx>, Error> {
        if matches!(op, BinOp::And | BinOp::Or) {
            let current_fn = self.current_function.unwrap();
            let current_block = self.builder.get_insert_block().unwrap();
            let left_cond = self.expr_to_bool(left_expr, "and_or_left_cond")?;
            let right_block = self.context.append_basic_block(current_fn, "and_or_right_block");
            let merge_block = self.context.append_basic_block(current_fn, "and_or_merge_block");
            match op {
                BinOp::And => self.builder.build_conditional_branch(left_cond, right_block, merge_block),
                BinOp::Or => self.builder.build_conditional_branch(left_cond, merge_block, right_block),
                _ => unreachable!()
            }
                .map_err(|e| Error::LLVMError { error: e.to_string() })?;
            self.builder.position_at_end(right_block);
            let right_cond = self.expr_to_bool(right_expr, "and_or_right_cond")?;
            let right_block_end = self.builder.get_insert_block().unwrap();
            self.builder.build_unconditional_branch(merge_block)
                .map_err(|e| Error::LLVMError { error: e.to_string() })?;
            self.builder.position_at_end(merge_block);
            let phi = self.builder.build_phi(self.context.bool_type(), "and_or_phi")
                .map_err(|e| Error::LLVMError { error: e.to_string() })?;
            match op {
                BinOp::And => {
                    phi.add_incoming(&[(
                        &self.context.bool_type().const_int(0, false),
                        current_block
                    )]);
                    phi.add_incoming(&[(
                        &right_cond,
                        right_block_end
                    )]);
                },
                BinOp::Or => {
                    phi.add_incoming(&[(
                        &self.context.bool_type().const_int(1, false),
                        current_block
                    )]);
                    phi.add_incoming(&[(
                        &right_cond,
                        right_block_end
                    )]);
                },
                _ => unreachable!()
            }
            return Ok(phi.as_basic_value());
        }
        let left = self.compile_some_expr(left_expr)?;
        let right = self.compile_some_expr(right_expr)?;
        match (left, right) {
            (BasicValueEnum::IntValue(left_val), BasicValueEnum::IntValue(right_val)) => match op {
                BinOp::Plus => self.builder.build_int_add(left_val, right_val, "int_add"),
                BinOp::Minus => self.builder.build_int_sub(left_val, right_val, "int_sub"),
                BinOp::Multiply => self.builder.build_int_mul(left_val, right_val, "int_mul"),
                BinOp::Divide => self.builder.build_int_signed_div(left_val, right_val, "int_div"),
                BinOp::Eq => self.builder.build_int_compare(IntPredicate::EQ, left_val, right_val, "int_eq"),
                BinOp::Greater => self.builder.build_int_compare(IntPredicate::SGT, left_val, right_val, "int_sgt"),
                BinOp::Lower => self.builder.build_int_compare(IntPredicate::SLT, left_val, right_val, "int_slt"),
                BinOp::GreaterEq => self.builder.build_int_compare(IntPredicate::SGE, left_val, right_val, "int_sge"),
                BinOp::LowerEq => self.builder.build_int_compare(IntPredicate::SLE, left_val, right_val, "int_sle"),
                BinOp::NotEq => self.builder.build_int_compare(IntPredicate::NE, left_val, right_val, "int_ne"),
                _ => unreachable!()
            }
                .map_err(|e| Error::LLVMError { error: e.to_string() })
                .map(|res| res.into()),
            (BasicValueEnum::FloatValue(left_val), BasicValueEnum::FloatValue(right_val)) => match op {
                BinOp::Plus => self.builder.build_float_add(left_val, right_val, "float_add").map(|res| res.into()),
                BinOp::Minus => self.builder.build_float_sub(left_val, right_val, "float_sub").map(|res| res.into()),
                BinOp::Multiply => self.builder.build_float_mul(left_val, right_val, "float_mul").map(|res| res.into()),
                BinOp::Divide => self.builder.build_float_div(left_val, right_val, "float_div").map(|res| res.into()),
                BinOp::Eq => self.builder.build_float_compare(FloatPredicate::OEQ, left_val, right_val, "float_oeq").map(|res| res.into()),
                BinOp::Greater => self.builder.build_float_compare(FloatPredicate::OGT, left_val, right_val, "float_ogt").map(|res| res.into()),
                BinOp::Lower => self.builder.build_float_compare(FloatPredicate::OLT, left_val, right_val, "float_olt").map(|res| res.into()),
                BinOp::GreaterEq => self.builder.build_float_compare(FloatPredicate::OGE, left_val, right_val, "float_oge").map(|res| res.into()),
                BinOp::LowerEq => self.builder.build_float_compare(FloatPredicate::OLE, left_val, right_val, "float_ole").map(|res| res.into()),
                BinOp::NotEq => self.builder.build_float_compare(FloatPredicate::ONE, left_val, right_val, "float_one").map(|res| res.into()),
                _ => unreachable!()
            }
                .map_err(|e| Error::LLVMError { error: e.to_string() }),
            _ => Err(Error::UnexpectedType { expected: vec![
                Type::I64, Type::I32, Type::I16, Type::I8,
                Type::U64, Type::U32, Type::U16, Type::U8,
                Type::F64, Type::F32
            ], got: self.expr_to_type(left_expr)?, span: left_expr.span })
        }
    }

    fn compile_some_expr(&mut self, expression: &Spanned<Expression>) -> Result<BasicValueEnum<'ctx>, Error> {
        let ty = self.expr_to_type(expression)?;
        self.compile_expr(expression)?
            .ok_or_else(|| Error::UnexpectedType { expected: vec![
                Type::I64, Type::I32, Type::I16, Type::I8,
                Type::U64, Type::U32, Type::U16, Type::U8,
                Type::F64, Type::F32,
                Type::Bool,
                Type::Str
            ], got: ty, span: expression.span })
    }
}
