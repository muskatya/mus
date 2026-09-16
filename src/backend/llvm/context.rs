use crate::frontend::ast::Type;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{FunctionValue, PointerValue};
use inkwell::basic_block::BasicBlock;
use inkwell::AddressSpace;
use std::collections::HashMap;

pub struct CodegenContext<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub functions: HashMap<String, FunctionValue<'ctx>>,
    pub externs: HashMap<String, FunctionValue<'ctx>>,
    pub current_func: Option<FunctionValue<'ctx>>,
    pub continue_block: Option<BasicBlock<'ctx>>,
    pub break_block: Option<BasicBlock<'ctx>>,
    pub variables: HashMap<String, PointerValue<'ctx>>
}

impl<'ctx> CodegenContext<'ctx> {
    pub fn new(context: &'ctx Context) -> CodegenContext<'ctx> {
        CodegenContext {
            context,
            module: context.create_module("main"),
            builder: context.create_builder(),
            functions: HashMap::new(),
            externs: HashMap::new(),
            current_func: None,
            continue_block: None,
            break_block: None,
            variables: HashMap::new()
        }
    }

    pub fn is_block_terminated(&self) -> bool {
        self.builder.get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_some()
    }

    pub fn type_to_llvm(&self, ty: &Type) -> Option<BasicTypeEnum<'ctx>> {
        let llvm = match ty {
            Type::I64 | Type::U64 => self.context.i64_type().into(),
            Type::I32 | Type::U32 => self.context.i32_type().into(),
            Type::I16 | Type::U16 => self.context.i16_type().into(),
            Type::I8 | Type::U8 | Type::Char => self.context.i8_type().into(),
            Type::F64 => self.context.f64_type().into(),
            Type::F32 => self.context.f32_type().into(),
            Type::Bool => self.context.bool_type().into(),
            Type::Str => self.context.ptr_type(AddressSpace::default()).into(),
            _ => return None
        };
        Some(llvm)
    }
}
