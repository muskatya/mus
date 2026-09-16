use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum ScopeKind {
    Global,
    Module,
    Local,
    Loop
}

#[derive(Debug)]
pub struct Scope {
    pub id: usize,
    pub parent: Option<usize>,
    pub kind: ScopeKind,
    pub symbols: HashMap<String, usize>
}

#[derive(Debug)]
pub struct ScopeArena {
    pub scopes: Vec<Scope>
}

impl ScopeArena {
    pub fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    pub fn add_scope(&mut self, parent: Option<usize>, kind: ScopeKind) -> usize {
        let id = self.scopes.len();
        self.scopes.push(Scope {
            id,
            parent,
            kind,
            symbols: HashMap::new()
        });
        id
    }

    pub fn get(&self, id: usize) -> Option<&Scope> {
        self.scopes.get(id)
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut Scope> {
        self.scopes.get_mut(id)
    }

    pub fn get_parent(&self, id: usize) -> Option<usize> {
        self.scopes.get(id).and_then(|s| s.parent)
    }

    pub fn get_loop(&self, start: usize) -> Option<usize> {
        let mut current = Some(start);
        while let Some(id) = current {
            if let Some(scope) = self.scopes.get(id) {
                if scope.kind == ScopeKind::Loop {
                    return Some(id)
                }
            }
            current = self.get_parent(id);
        }
        None
    }

    pub fn add_symbol(&mut self, id: usize, name: &str, symbol: usize) {
        self.scopes[id].symbols.insert(name.to_string(), symbol);
    }

    pub fn lookup(&self, id: usize, name: &str) -> Option<usize> {
        self.scopes[id].symbols.get(name).copied()
    }

    pub fn lookup_visible(&self, start: usize, name: &str) -> Option<usize> {
        let mut current = Some(start);
        while let Some(id) = current {
            if let Some(symbol) = self.lookup(id, name) {
                return Some(symbol);
            }
            current = self.get_parent(id);
        }
        None
    }
}
