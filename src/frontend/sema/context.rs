use crate::frontend::ast::{Type, Visibility};
use crate::frontend::sema::scopes::{ScopeArena, ScopeKind};
use crate::frontend::sema::symbols::{SymbolTable, Symbol, SymbolKind};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Context {
    pub scopes: ScopeArena,
    pub current_scope: usize,
    pub symbols: SymbolTable,
    pub functions: HashMap<String, usize>
}

impl Context {
    pub fn new() -> Self {
        let mut scopes = ScopeArena::new();
        let current_scope = scopes.add_scope(None, ScopeKind::Global);
        Self { scopes, current_scope, symbols: SymbolTable::new(), functions: HashMap::new() }
    }

    pub fn in_loop(&self) -> bool {
        self.scopes
            .get_loop(self.current_scope)
            .is_some()
    }

    pub fn push_scope(&mut self, kind: ScopeKind) -> usize {
        let parent = self.current_scope;
        let scope = self.scopes.add_scope(Some(parent), kind);
        self.current_scope = scope;
        scope
    }

    pub fn pop_scope(&mut self) -> usize {
        let old = self.current_scope;
        if let Some(parent) = self.scopes.get_parent(old) {
            self.current_scope = parent;
        }
        old
    }

    pub fn add_func(
        &mut self,
        name: &str,
        mangled: &str,
        params: Vec<Type>,
        ret_type: Type,
        visibility: Visibility,
        module: &str
    ) -> usize {
        let func = Symbol::new_fn(name, mangled, params, ret_type, visibility, module);
        let id = self.symbols.insert(func);
        self.scopes.add_symbol(self.current_scope, name, id);
        self.functions.insert(mangled.to_string(), id);
        id
    }

    pub fn get_func(&self, mangled: &str) -> Option<&Symbol> {
        self.functions.get(mangled).and_then(|&i| self.symbols.get(i))
    }

    pub fn find_func(&self, name: &str, module_path: &str) -> Option<&Symbol> {
        if let Some(symbol) = self.get_func(&format!("{}.{}", module_path, name)) {
            return Some(symbol);
        }
        if let Some(id) = self.scopes.lookup_visible(self.current_scope, name) {
            if let Some(symbol) = self.symbols.get(id) && symbol.kind == SymbolKind::Function {
                return Some(symbol);
            }
        }
        None
    }

    pub fn add_var(&mut self, name: &str, ty: Type, mutable: bool) -> usize {
        let var = Symbol::new_var(name, ty, mutable);
        let id = self.symbols.insert(var);
        self.scopes.add_symbol(self.current_scope, name, id);
        id
    }

    pub fn get_var(&self, name: &str) -> Option<&Symbol> {
        self.scopes.lookup_visible(self.current_scope, name).and_then(|i| self.symbols.get(i))
    }
}
