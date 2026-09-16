use crate::frontend::Error;
use crate::frontend::{lexer::lexer, parser::parser, sema::{sema, type_checker}};
use crate::backend::llvm::{codegen, context::CodegenContext};
use colored::Colorize;
use inkwell::context::Context;
use inkwell::OptimizationLevel;
use inkwell::targets::{InitializationConfig, Target, TargetMachine, RelocMode, CodeModel, FileType};
use std::path::Path;
use std::time::Instant;
use std::process::{self, Command};
use std::fs;

pub struct Driver<'ctx> {
    llvm_ctx: &'ctx Context,
    codegen_ctx: Option<CodegenContext<'ctx>>
}

impl<'ctx> Driver<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self { llvm_ctx: context, codegen_ctx: None }
    }

    pub fn compile(&mut self, source: &str, file: &str) -> Result<(), Error> {
        println!("{:>12} '{}'", "Compiling".green().bold(), file);
        let start = Instant::now();
        let mut lex = lexer::Lexer::new(source.to_string(), file.to_string());
        lex.tokenize()?;
        let mut parse = parser::Parser::new(lex.tokens, file.to_string(), source.to_string());
        let ast = parse.parse_program()?;
        let mut semantic = sema::Sema::new(file.to_string(), source.to_string());
        let typed_ast = semantic.analyze(&ast)?;
        let mut type_check = type_checker::TypeChecker::new(file.to_string(), source.to_string());
        type_check.check(&typed_ast)?;
        let mut codegenerate = codegen::Codegen::new(self.llvm_ctx);
        codegenerate.compile_program(&typed_ast);
        self.codegen_ctx = Some(codegenerate.into_context());
        println!("{:>12} in {:.2}s", "Finished".green().bold(), start.elapsed().as_secs_f64());
        Ok(())
    }

    pub fn print_ir(&self) {
        self.codegen_ctx.as_ref().unwrap().module.print_to_stderr();
    }

    pub fn run(&self, release: bool) {
        let opt_level = if release {
            OptimizationLevel::Aggressive
        } else {
            OptimizationLevel::Default
        };
        let execution_engine = self.codegen_ctx.as_ref().unwrap().module
            .create_jit_execution_engine(opt_level)
            .unwrap();
        type MainFn = unsafe extern "C" fn() -> i32;
        unsafe {
            execution_engine.get_function::<MainFn>("main")
                .unwrap()
                .call();
        }
    }

    pub fn gen_exe(&self, release: bool) {
        let opt_level = if release {
            OptimizationLevel::Aggressive
        } else {
            OptimizationLevel::Default
        };
        Target::initialize_all(&InitializationConfig::default());
        let triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&triple).unwrap();
        let target_machine = target.create_target_machine(
            &triple,
            TargetMachine::get_host_cpu_name().to_str().unwrap(),
            TargetMachine::get_host_cpu_features().to_str().unwrap(),
            opt_level,
            RelocMode::PIC,
            CodeModel::Default
        )
            .unwrap();
        let object_name = if cfg!(target_os = "windows") {
            "main.obj"
        } else {
            "main.o"
        };
        target_machine.write_to_file(
            &self.codegen_ctx.as_ref().unwrap().module,
            FileType::Object,
            &Path::new(object_name)
        )
            .unwrap();
        let exit_status = if cfg!(target_os = "windows") {
            Command::new("link.exe").args(&[object_name, "/out:main.exe", "/entry:main", "/subsystem:console", "/defaultlib:msvcrt"]).status()
        } else {
            Command::new("cc").args(&[object_name, "-o", "main", "-lc"]).status()
        }
            .unwrap();
        if !exit_status.success() {
            eprintln!("{}: linking executable failed with code {}", "error".red().bold(), exit_status.code().unwrap());
            process::exit(1);
        }
        let _ = fs::remove_file(object_name);
    }
}
