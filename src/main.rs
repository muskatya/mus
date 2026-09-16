mod cli;
mod frontend;
mod backend;

use cli::cli::Cli;
use clap::Parser;
use colored::Colorize;
use inkwell::context::Context;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let parsed = Cli::parse();
    match parsed {
        Cli::Run { .. } | Cli::Build { .. } => {
            let path = Path::new("main.mus");
            if !path.exists() {
                eprintln!("{}: couldn't find 'main.mus'", "error".red().bold());
                process::exit(1);
            }
            let source = match fs::read_to_string("main.mus") {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{}: {}", "error".red().bold(), e.to_string());
                    process::exit(1);
                }
            };
            let context = Context::create();
            let mut driver = cli::driver::Driver::new(&context);
            match driver.compile(&source, "main.mus") {
                Ok(_) => (),
                Err(e) => {
                    e.display();
                    process::exit(1);
                }
            }
            match parsed {
                Cli::Run { release } => driver.run(release),
                Cli::Build { release } => driver.gen_exe(release),
            }
        }
    }
}
