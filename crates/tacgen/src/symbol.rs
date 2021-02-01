use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::{atomic::AtomicUsize, Arc},
};

use azuki_tac::Ty;
use smol_str::SmolStr;

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

pub struct NumberingCounter(AtomicUsize);

impl NumberingCounter {
    pub fn new(start: usize) -> Self {
        NumberingCounter(start.into())
    }

    pub fn next(&self) -> usize {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

/// A single variable
pub struct Variable {
    /// Whether if this variable is a global variable
    pub is_global: bool,
    /// The unique global ID of this variable
    pub id: usize,
    /// The type of this variable
    pub ty: Ty,
}

pub struct ScopeBuilder<'a> {
    counter: Rc<NumberingCounter>,
    interner: Rc<RefCell<StringInterner>>,
    scope: Scope<'a>,
}

impl<'a> ScopeBuilder<'a> {
    pub fn new(
        parent: Option<&'a Scope>,
        counter: Rc<NumberingCounter>,
        interner: Rc<RefCell<StringInterner>>,
    ) -> ScopeBuilder<'a> {
        ScopeBuilder {
            counter,
            interner,
            scope: Scope::new(parent),
        }
    }

    /// Insert a variable with given name and type into this scope. Returns a reference to the
    /// inserted variable if succeeded, and `None` if failed.
    pub fn insert(&mut self, name: &SmolStr, ty: Ty) -> Option<&Variable> {
        if self.scope.get_self(name).is_some() {
            return None;
        }

        let interned_name = self.interner.borrow_mut().intern(name);
        let var_id = self.counter.next();
        let variable = Variable {
            is_global: self.scope.is_global(),
            id: var_id,
            ty,
        };

        let entry = self.scope.vars.entry(interned_name).or_insert(variable);

        Some(entry)
    }

    pub fn scope(&self) -> &Scope<'a> {
        &self.scope
    }
}

pub struct Scope<'a> {
    parent: Option<&'a Scope<'a>>,
    vars: HashMap<SmolStr, Variable>,
}

impl<'a> Scope<'a> {
    pub fn new(parent: Option<&'a Scope>) -> Scope<'a> {
        Scope {
            parent,
            vars: HashMap::new(),
        }
    }

    pub fn is_global(&self) -> bool {
        self.parent.is_none()
    }

    pub fn get_self(&self, name: &str) -> Option<&Variable> {
        self.vars.get(name)
    }

    pub fn get_recursive(&self, name: &str) -> Option<&Variable> {
        match self.get_self(name) {
            x @ Some(_) => x,
            None => self.parent.and_then(|parent| parent.get_recursive(name)),
        }
    }
}
