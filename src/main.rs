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
    Run
}

fn run_file() -> Result<(), ()> {
    let source = fs::read_to_string("main.mus").map_err(|e| {
        eprintln!("{}: {}", "error".red().bold(), e);
    })?;
    let mut lex = lexer::Lexer::new(source);
    lex.tokenize().map_err(|e| {
        eprintln!("{}: {}", "error".red().bold(), e);
    })?;
    let mut parse = parser::Parser::new(lex.tokens);
    let program = parse.parse_program().map_err(|e| {
        eprintln!("{}: {}", "error".red().bold(), e);
    })?;
    let context = Context::create();
    let mut codegenerator = codegen::Codegen::new(&context);
    codegenerator.compile_program(program).map_err(|e| {
        eprintln!("{}: {}", "error".red().bold(), e);
    })?;
    codegenerator.run_jit().map_err(|e| {
        eprintln!("{}: {}", "error".red().bold(), e);
    })?;
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let result = match cli {
        Cli::Run => run_file()
    };
    if result.is_err() {
        process::exit(1)
    }
}
