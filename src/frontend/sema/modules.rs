use crate::frontend::{Span, Error, ErrorKind};
use crate::frontend::ast::*;
use crate::frontend::sema::context::Context;
use crate::frontend::{lexer, parser};
use super::scopes::ScopeKind;
use std::collections::{HashSet, HashMap};
use std::fs;
use std::path::PathBuf;

pub struct Module {
    pub path: String,
    pub program: Program,
    pub file: String,
    pub source: String
}

pub struct ModuleSystem {
    context: Context,
    modules: Vec<Module>,
    loaded: HashSet<String>,
    scopes: HashMap<String, usize>,
    file: String,
    source: String
}

impl ModuleSystem {
    pub fn new(file: String, source: String) -> ModuleSystem {
        ModuleSystem {
            context: Context::new(),
            modules: Vec::new(),
            loaded: HashSet::new(),
            scopes: HashMap::new(),
            file,
            source
        }
    }

    fn error(&self, span: Span, message: &str, note: Option<&str>) -> Error {
        Error::new(
            ErrorKind::Semantic,
            &self.file,
            &self.source,
            span,
            message,
            note
        )
    }

    pub fn into_parts(self) -> (Context, Vec<Module>, HashMap<String, usize>) {
        (self.context, self.modules, self.scopes)
    }

    fn load_module(&mut self, path: &str, span: Span) -> Result<(), Error> {
        if self.loaded.contains(path) {
            return Ok(());
        }
        let (source, imported_name, file) = self.load_source(path, span)?;
        match imported_name {
            Some(name) => {
                let parts: Vec<&str> = path.split(".").collect();
                let path = parts[..parts.len() - 1].join(".");
                self.load_module(&path, span)?;
                self.import_single(&path, &name, None, span)?;
                self.loaded.insert(path.to_string());
            },
            None => {
                let ast = Self::compile_source(source.clone(), file.clone())?;
                self.modules.push(Module {
                    path: path.to_string(),
                    program: ast.clone(),
                    file,
                    source
                });
                let scope = self.context.scopes.add_scope(None, ScopeKind::Module);
                let old = self.context.current_scope;
                self.context.current_scope = scope;
                self.scopes.insert(path.to_string(), scope);
                for item in &ast.items {
                    match item {
                        Item::Extern(ext) => {
                            self.context.add_func(
                                &ext.name,
                                &format!("{}.{}", path, ext.name),
                                ext.params.iter().map(|p| p.node.ty.node).collect(),
                                ext.ret_type.node,
                                ext.visibility,
                                path
                            );
                        },
                        Item::Function(func) => {
                            self.context.add_func(
                                &func.name.node,
                                &format!("{}.{}", path, func.name.node),
                                func.params.iter().map(|p| p.node.ty.node).collect(),
                                func.ret_type.node,
                                func.visibility,
                                path
                            );
                        },
                        Item::Import(_) => ()
                    }
                }
                for item in &ast.items {
                    if let Item::Import(imp) = item {
                        self.resolve_import(&imp)?;
                    }
                }
                self.context.current_scope = old;
                self.loaded.insert(path.to_string());
            }
        }
        Ok(())
    }

    fn resolve_import(&mut self, imp: &Import) -> Result<(), Error> {
        let path = imp.path.path.node.join(".");
        let alias = imp.path.alias.as_ref().map(|a| a.as_str());
        if imp.path.items.is_empty() {
            match self.load_source(&path, imp.path.path.span)? {
                (_, None, _) => self.import_all(&path, alias, imp.path.path.span)?,
                (_, Some(_), _) => {
                    let parts: Vec<&str> = path.split(".").collect();
                    let parent = parts[..parts.len() - 1].join(".");
                    let name = parts[parts.len() - 1];
                    self.load_module(&parent, imp.path.path.span)?;
                    self.import_single(&parent, name, alias, imp.path.path.span)?;
                    self.loaded.insert(path.to_string());
                }
            }
        } else {
            self.load_module(&path, imp.path.path.span)?;
            for item in &imp.path.items {
                let name = item.path.node.join(".");
                self.import_single(&path, &name, item.alias.as_ref().map(|a| a.as_str()), item.path.span)?;
                self.loaded.insert(path.to_string());
            }
        }
        Ok(())
    }

    fn import_all(&mut self, path: &str, alias: Option<&str>, span: Span) -> Result<(), Error> {
        self.load_module(path, span)?;
        let scope = self.scopes[path];
        let prefix = alias.unwrap_or(path.rsplit(".").next().unwrap_or(path));
        for (name, id) in &self.context.scopes.scopes.iter().find(|s| s.id == scope).unwrap().symbols.clone() {
            let symbol = self.context.symbols.get(*id).unwrap();
            if symbol.visibility != Visibility::Public {
                continue;
            }
            let local = format!("{}.{}", prefix, name);
            self.context.scopes.add_symbol(self.context.current_scope, &local, *id);
        }
        Ok(())
    }

    fn import_single(&mut self, path: &str, name: &str, alias: Option<&str>, span: Span) -> Result<(), Error> {
        let mangled = format!("{}.{}", path, name);
        let symbol = self.context.get_func(&mangled)
            .or(self.context.get_func(name))
            .ok_or(self.error(
                span,
                &format!("Item '{}' not found in module '{}'", name, path),
                None
            ))?;
        if symbol.visibility != Visibility::Public {
            return Err(self.error(
                span,
                &format!("Imported item '{}' is private", name),
                None
            ))
        }
        let local = alias.unwrap_or(name);
        self.context.scopes.add_symbol(self.context.current_scope, &local, symbol.id);
        Ok(())
    }

    pub fn register_main(&mut self, program: &Program, file: String, source: String) -> Result<(), Error> {
        for item in &program.items {
            match item {
                Item::Extern(ext) => {
                    self.context.add_func(
                        &ext.name,
                        &format!("main.{}", ext.name),
                        ext.params.iter().map(|p| p.node.ty.node).collect(),
                        ext.ret_type.node,
                        ext.visibility,
                        "main"
                    );
                },
                Item::Function(func) => {
                    self.context.add_func(
                        &func.name.node,
                        &format!("main.{}", func.name.node),
                        func.params.iter().map(|p| p.node.ty.node).collect(),
                        func.ret_type.node,
                        func.visibility,
                        "main"
                    );
                },
                Item::Import(_) => ()
            }
        }
        for item in &program.items {
            if let Item::Import(imp) = item {
                self.resolve_import(imp)?;
            }
        }
        self.modules.push(Module {
            path: "main".to_string(),
            program: program.clone(),
            file,
            source
        });
        self.loaded.insert("main".to_string());
        Ok(())
    }

    fn load_source(&self, path: &str, span: Span) -> Result<(String, Option<String>, String), Error> {
        let parts: Vec<&str> = path.split(".").collect();
        let path_buf = Self::build_path(&parts);
        if path_buf.exists() {
            return Ok((fs::read_to_string(path_buf.clone())
                .map_err(|e| self.error(
                    span,
                    &format!("An error occurred when reading module '{}': {}", path, e),
                    None
                ))?, None, path_buf.to_str().unwrap().to_string()));
        }
        if parts.len() >= 2 {
            let func = parts[parts.len() - 1];
            let parent_buf = Self::build_path(&parts[..parts.len() - 1]);
            if parent_buf.exists() {
                return Ok((fs::read_to_string(parent_buf.clone())
                    .map_err(|e| self.error(
                        span,
                        &format!("An error occurred when reading module '{}': {}", path, e),
                        None
                    ))?, Some(func.to_string()), parent_buf.to_str().unwrap().to_string()));
            }
        }
        Err(self.error(
            span,
            &format!("Module '{}' not found", path),
            None
        ))
    }

    fn build_path(parts: &[&str]) -> PathBuf {
        let mut path = PathBuf::new();
        for part in parts {
            path.push(part);
        }
        path.add_extension("mus");
        path
    }

    fn compile_source(source: String, file: String) -> Result<Program, Error> {
        let mut lex = lexer::lexer::Lexer::new(source.clone(), file.clone());
        lex.tokenize()?;
        let mut parse = parser::parser::Parser::new(lex.tokens, source, file);
        parse.parse_program()
    }
}
