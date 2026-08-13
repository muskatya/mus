mod errors;
mod lexer;
mod parser;
mod codegen;

use clap::Parser;
use colored::Colorize;
use inkwell::context::Context;
use std::process;
use std::fs;

#[derive(Parser)]
#[command(name = "mus", version = "0.2.0", about = "Mus programming language")]
enum Cli {
    /// Run Mus program using JIT compilation
    Run,
    /// Compile Mus program to an executable
    Build
}

fn main() {
    let cli = Cli::parse();
    let source = match fs::read_to_string("main.mus") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: {}", "error".red(), e);
            process::exit(1);
        }
    };
    let mut lex = lexer::Lexer::new(source);
    if let Err(e) = lex.tokenize() {
        eprintln!("{}: {}", "error".red(), e);
        process::exit(1);
    }
    let mut parse = parser::Parser::new(lex.tokens);
    let program = match parse.parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: {}", "error".red(), e);
            process::exit(1);
        }
    };
    let context = Context::create();
    let mut codegenerator = codegen::Codegen::new(&context);
    if let Err(e) = codegenerator.compile_program(program) {
        eprintln!("{}: {}", "error".red(), e);
        process::exit(1);
    }
    let result = match cli {
        Cli::Run => codegenerator.run_jit(),
        Cli::Build => codegenerator.compile_to_executable()
    };
    if let Err(e) = result {
        eprintln!("{}: {}", "error".red(), e);
        process::exit(1);
    }
}
