use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::mem::forget;
use std::ops::{Deref, DerefMut};

use crate::taggedptr::{FatPtr, TaggedPtr};

// A thing to limit moveability and lifetime of root pointers
pub struct RootScopeGuard<'env> {
    _env: PhantomData<&'env Environment>,
}

impl<'env> RootScopeGuard<'env> {
    pub fn new(_env: &'env Environment) -> RootScopeGuard {
        RootScopeGuard { _env: PhantomData }
    }
}

// A GC managed root pointer
pub struct Root<'guard, 'env: 'guard> {
    root: FatPtr,
    _mkr: PhantomData<&'guard mut RootScopeGuard<'env>>,
}

impl<'guard, 'env> Root<'guard, 'env> {
    pub fn new(_guard: &'guard mut RootScopeGuard<'env>, ptr: FatPtr) -> Root<'guard, 'env> {
        Root {
            root: ptr,
            _mkr: PhantomData,
        }
    }
}

impl<'guard, 'env> Deref for Root<'guard, 'env> {
    type Target = FatPtr;

    fn deref(&self) -> &FatPtr {
        &self.root
    }
}

impl<'guard, 'env> Clone for Root<'guard, 'env> {
    fn clone(&self) -> Self {
        Root {
            root: self.root,
            _mkr: self._mkr,
        }
    }
}

impl<'guard, 'env> Copy for Root<'guard, 'env> {}

impl<'guard, 'env> fmt::Display for Root<'guard, 'env> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.root.fmt(f)
    }
}

// A GC managed mutable root pointer
struct MutRoot<'guard, 'env: 'guard> {
    root: *const FatPtr,
    _mkr: PhantomData<&'guard mut RootScopeGuard<'env>>,
}

impl<'guard, 'env> MutRoot<'guard, 'env> {
    fn new(_guard: &'guard mut RootScopeGuard<'env>, thing: &FatPtr) -> MutRoot<'guard, 'env> {
        MutRoot {
            root: thing as *const FatPtr,
            _mkr: PhantomData,
        }
    }

    fn replace_with_new<T>(&mut self, env: &Environment, object: T)
    where
        FatPtr: From<Gc<T>>,
    {
        env.alloc_over(unsafe { &mut *(self.root as *mut FatPtr) }, object);
    }

    fn replace_with_nil(&mut self) {
        *(self.deref_mut()) = FatPtr::Nil;
    }
}

impl<'guard, 'env> Deref for MutRoot<'guard, 'env> {
    type Target = FatPtr;

    fn deref(&self) -> &FatPtr {
        unsafe { &*self.root }
    }
}

impl<'guard, 'env> DerefMut for MutRoot<'guard, 'env> {
    fn deref_mut(&mut self) -> &mut FatPtr {
        unsafe { &mut *(self.root as *mut FatPtr) }
    }
}

impl<'guard, 'env> Clone for MutRoot<'guard, 'env> {
    fn clone(&self) -> Self {
        MutRoot {
            root: self.root,
            _mkr: self._mkr,
        }
    }
}

impl<'guard, 'env> Copy for MutRoot<'guard, 'env> {}

impl<'guard, 'env> fmt::Display for MutRoot<'guard, 'env> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe { &*self.root }.fmt(f)
    }
}

// Allocate a new object into a register and root it
macro_rules! let_root_from_new {
    ($env:ident, $root:ident, $object:expr, $reg:expr) => {
        // Ensure the root is scope-lifetime-guarded
        let mut $root = RootScopeGuard::new(&$env);

        // Shadow the original binding so that it can't be directly accessed ever again.
        #[allow(unused_mut)]
        let $root = Root::new(&mut $root, $env.alloc_into_reg($reg, $object));
    };
}

/*
// Bind a register to a root readonly
macro_rules! let_root_reg {
    ($env:ident, $name:ident, $reg:expr) => {
        // Ensure the root is scope-lifetime-guarded
        let mut $name = RootScopeGuard::new(&$env);

        // Shadow the original binding so that it can't be directly accessed ever again.
        #[allow(unused_mut)]
        let $name = Root::new(&mut $name, $env::regval($reg));
    }
}
*/

// Get a reference to a register in the stack and root it
macro_rules! let_root_mut_reg {
    ($env:ident, $name:ident, $reg:expr) => {
        // Ensure the root is scope-lifetime-guarded
        let mut $name = RootScopeGuard::new(&$env);

        // Shadow the original binding so that it can't be directly accessed ever again.
        #[allow(unused_mut)]
        let $name = MutRoot::new(&mut $name, $env.get_reg_ref($reg));
    };
}

// Expand a root into it's possible dynamic value and match on it
macro_rules! match_root {
    (Root: $root:expr;
     Int($i:ident) => $int_handler:expr,
     Str($s:ident) => $str_handler:expr,
     Nil => $nil_handler:expr
    ) => {
        match $root.deref() {
            FatPtr::Integer(i) => {
                let $i = unsafe { *i.as_ref() };
                $int_handler
            }
            FatPtr::String(s) => {
                let $s = unsafe { s.as_ref() };
                $str_handler
            }
            FatPtr::Nil => $nil_handler,
        }
    };
}

// A minimal pretend GC environment
struct Environment {
    regs: Vec<Cell<FatPtr>>>,
}

impl Environment {
    fn new() -> Environment {
        let capacity = 256;

        let mut regs = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            regs.push(Cell::new(FatPtr::Nil));
        }

        Environment {
            regs: regs,
        }
    }

    // Replace an existing ptr with a new ptr to a newly allocated object
    fn alloc_over<T>(&self, dest: &mut FatPtr, object: T)
    where
        FatPtr: From<Gc<T>>,
    {
        *dest = FatPtr::from(Gc::new(object));
    }

    // Replace a register ptr with a new ptr to a newly allocated object
    fn alloc_into_reg<T>(&self, reg: usize, object: T) -> FatPtr
    where
        FatPtr: From<Gc<T>>,
    {
        let regs = unsafe { &mut *self.regs.get() };
        let ptr = FatPtr::from(Gc::new(object));
        regs[reg] = ptr;
        ptr
    }

    fn get_reg_ref(&self, reg: usize) -> &mut FatPtr {
        let regs = unsafe { &mut *self.regs.get() };
        &mut regs[reg]
    }
}

fn add(env: &Environment, a: Root, b: Root, mut c: MutRoot) -> Result<(), ()> {
    match_root! {
        Root: a;
        Int(value1) => {
            if let Some(value2) = b.as_integer() {
                let result = value1 + value2;
                c.replace_with_new(env, result);
                return Ok(());
            }
        },
        Str(value1) => {
            if let Some(value2) = b.as_string() {
                let result = format!("{}{}", value1, value2);
                c.replace_with_new(env, result);
                return Ok(());
            }
        },
        Nil => {}
    }

    c.replace_with_nil();

    Err(())
}

// Do stuff
fn main() {
    {
        let env = Environment::new();

        let_root_from_new!(env, a, 3, 0);
        let_root_from_new!(env, b, 4, 1);
        let_root_mut_reg!(env, c, 2);

        let _res = add(&env, a, b, c).unwrap();

        println!("{} + {} = {}", a, b, c);
    }

    {
        let env = Environment::new();

        let_root_from_new!(env, a, String::from("foo"), 0);
        let_root_from_new!(env, b, String::from("bar"), 1);
        let_root_mut_reg!(env, c, 2);

        let _res = add(&env, a, b, c).unwrap();

        println!("{} + {} = {}", a, b, c);
    }

    {
        let env = Environment::new();

        let_root_from_new!(env, a, String::from("foo"), 0);
        let_root_from_new!(env, b, 3, 1);
        let_root_mut_reg!(env, c, 2);

        let _res = add(&env, a, b, c);

        println!("{} + {} = {}", a, b, c);
    }
}
