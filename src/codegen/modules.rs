use crate::errors::{Spanned, Error};
use crate::lexer::Lexer;
use crate::parser::ast::Program;
use crate::parser::Parser;

use std::path::PathBuf;
use std::fs;

pub struct ModuleSystem {
    pub modules: Vec<Program>,
}

impl ModuleSystem {
    pub fn new() -> ModuleSystem {
        ModuleSystem { modules: Vec::new() }
    }

    pub fn resolve(&mut self, path: &Spanned<String>) -> Result<(), Error> {
        let pathfmt = format!("{}.mus", path.node);
        let pathbuf = PathBuf::from(pathfmt.as_str());
        if !pathbuf.exists() {
            return Err(Error::ModuleNotFound { path: path.node.clone(), span: path.span });
        }
        let src = fs::read_to_string(pathbuf)
            .map_err(|e| Error::LLVMError { error: e.to_string() })?;
        let mut lexer = Lexer::new(src);
        lexer.tokenize()?;
        let mut parser = Parser::new(lexer.tokens);
        let ast = parser.parse_program()?;
        self.modules.push(ast);
        Ok(())
    }
}
