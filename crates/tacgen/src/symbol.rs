use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::{
        atomic::{AtomicU32, AtomicUsize},
        Arc,
    },
};

use azuki_tac::Ty;
use smol_str::SmolStr;
use vec1::{Size0Error, Vec1};

/// A struct for storing strings and reducing space usage.
pub struct StringInterner {
    set: HashSet<SmolStr>,
}

impl StringInterner {
    pub fn new() -> StringInterner {
        StringInterner {
            set: HashSet::new(),
        }
    }

    fn str_needs_interning(s: &str) -> bool {
        s.len() > 22
    }

    fn smolstr_needs_interning(s: &SmolStr) -> bool {
        s.is_heap_allocated()
    }

    /// Stores a string into the internal buffer. Returns an owned copy of this string, represented
    /// by a [`SmolStr`][SmolStr], which uses an `Arc` to store internal buffer and can be cloned in
    /// O(1) time.
    ///
    /// If the string doesn't need heap memory to store, it is not interned.
    pub fn intern_str(&mut self, s: &str) -> SmolStr {
        if !Self::str_needs_interning(s) {
            return SmolStr::new(s);
        }

        match self.set.get(s) {
            Some(s) => s.clone(),
            None => {
                let s = SmolStr::new(s);
                // clone is O(1)
                self.set.insert(s.clone());
                s
            }
        }
    }

    /// Stores an external [`SmolStr`][SmolStr] into internal buffer.
    ///
    /// If the string doesn't need heap memory to store, it is not interned, but simply copied.
    pub fn intern(&mut self, s: &SmolStr) -> SmolStr {
        if !Self::smolstr_needs_interning(s) {
            return s.clone();
        }

        match self.set.get(s) {
            Some(s) => s.clone(),
            None => {
                self.set.insert(s.clone());
                s.clone()
            }
        }
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

pub struct NumberingCounter(AtomicU32);

impl NumberingCounter {
    pub fn new(start: u32) -> Self {
        NumberingCounter(start.into())
    }

    pub fn next(&self) -> u32 {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

/// A single variable
pub struct Variable {
    /// Whether if this variable is a global variable
    pub is_global: bool,
    /// The unique global ID of this variable
    pub id: u32,
    /// The type of this variable
    pub ty: Ty,
}

pub struct ScopeBuilder {
    counter: Rc<NumberingCounter>,
    interner: Rc<RefCell<StringInterner>>,
    scopes: Vec1<Scope>,
}

impl ScopeBuilder {
    pub fn new(
        counter: Rc<NumberingCounter>,
        interner: Rc<RefCell<StringInterner>>,
    ) -> ScopeBuilder {
        ScopeBuilder {
            counter,
            interner,
            scopes: Vec1::new(Scope::new()),
        }
    }

    pub fn add_scope(&mut self) {
        self.scopes.push(Scope::new())
    }

    pub fn pop_scope(&mut self) -> Result<Scope, Size0Error> {
        self.scopes.try_pop()
    }

    pub fn is_top_scope_global(&self) -> bool {
        self.scopes().len() == 1
    }

    /// Insert a variable with given name and type into this scope. Returns a reference to the
    /// inserted variable if succeeded, and `None` if failed.
    pub fn insert(&mut self, name: &SmolStr, ty: Ty) -> Option<&Variable> {
        let interned_name = self.interner.borrow_mut().intern(name);
        let var_id = self.counter.next();
        let variable = Variable {
            is_global: self.is_top_scope_global(),
            id: var_id,
            ty,
        };

        let scope = self.top_scope_mut();
        scope.insert(interned_name, variable)
    }

    pub fn insert_global(&mut self, name: &SmolStr, ty: Ty) -> Option<&Variable> {
        let interned_name = self.interner.borrow_mut().intern(name);
        let var_id = self.counter.next();
        let variable = Variable {
            is_global: true,
            id: var_id,
            ty,
        };

        let scope = self.global_scope_mut();
        scope.insert(interned_name, variable)
    }

    pub fn find(&self, name: &str) -> Option<&Variable> {
        for scope in self.scopes().iter().rev() {
            if let Some(var) = scope.find(name) {
                return Some(var);
            }
        }
        None
    }

    pub fn scopes(&self) -> &[Scope] {
        &self.scopes
    }

    pub fn top_scope(&self) -> &Scope {
        self.scopes.last()
    }

    pub fn top_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut()
    }

    pub fn global_scope(&self) -> &Scope {
        self.scopes.first()
    }

    pub fn global_scope_mut(&mut self) -> &mut Scope {
        self.scopes.first_mut()
    }
}

pub struct Scope {
    vars: HashMap<SmolStr, Variable>,
}

impl Scope {
    pub fn new() -> Scope {
        Scope {
            vars: HashMap::new(),
        }
    }

    pub fn find(&self, name: &str) -> Option<&Variable> {
        self.vars.get(name)
    }

    pub fn insert(&mut self, name: SmolStr, val: Variable) -> Option<&Variable> {
        let entry = self.vars.entry(name);
        match entry {
            std::collections::hash_map::Entry::Occupied(_) => None,
            std::collections::hash_map::Entry::Vacant(e) => Some(e.insert(val)),
        }
    }
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}
