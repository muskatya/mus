use crate::frontend::ast::{Type, Visibility};

#[derive(Debug)]
pub struct SymbolTable {
    pub symbols: Vec<Symbol>,
    next_id: usize
}

impl SymbolTable {
    pub fn new() -> Self {
        Self { symbols: Vec::new(), next_id: 0 }
    }

    pub fn insert(&mut self, mut symbol: Symbol) -> usize {
        let id = self.next_id;
        symbol.id = id;
        self.symbols.push(symbol);
        self.next_id += 1;
        id
    }

    pub fn get(&self, id: usize) -> Option<&Symbol> {
        self.symbols.get(id)
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut Symbol> {
        self.symbols.get_mut(id)
    }
}

#[derive(Debug, PartialEq)]
pub enum SymbolKind {
    Function,
    Variable
}

#[derive(Debug)]
pub struct Symbol {
    pub id: usize,
    pub name: String,
    pub kind: SymbolKind,
    pub ty: Type,
    pub mangled: String,
    pub params: Vec<Type>,
    pub mutable: bool,
    pub visibility: Visibility,
    pub module: String
}

impl Symbol {
    pub fn new_fn(
        name: &str,
        mangled: &str,
        params: Vec<Type>,
        ret_type: Type,
        visibility: Visibility,
        module: &str
    ) -> Self {
        Self {
            id: 0,
            name: name.to_string(),
            kind: SymbolKind::Function,
            ty: ret_type,
            mangled: mangled.to_string(),
            params,
            mutable: false,
            visibility,
            module: module.to_string()
        }
    }

    pub fn new_var(name: &str, ty: Type, mutable: bool) -> Self {
        Self {
            id: 0,
            name: name.to_string(),
            kind: SymbolKind::Variable,
            ty, 
            mangled: String::new(),
            params: Vec::new(),
            mutable,
            visibility: Visibility::Private,
            module: String::new()
        }
    }
}
