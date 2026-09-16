use clap::Parser;

#[derive(Parser)]
#[command(name = "mus", version = "0.2.0", about = "Mus programming language")]
pub enum Cli {
    /// Run Mus program using JIT compilation
    Run {
        #[arg(long)]
        release: bool
    },
    /// Compile Mus program to an executable
    Build {
        #[arg(long)]
        release: bool
    }
}
