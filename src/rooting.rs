/// XXX a test - merge into main code XXX

use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::mem::forget;
use std::ops::Deref;
use std::ptr::NonNull;

// A pretend GC-managed raw pointer, no safety
struct Gc<T> {
    obj: NonNull<T>,
}

impl<T> Clone for Gc<T> {
    fn clone(&self) -> Gc<T> {
        Gc { obj: self.obj }
    }
}

impl<T> Copy for Gc<T> {}

impl<T> Gc<T> {
    fn new(obj: T) -> Gc<T> {
        let mut thing = Box::new(obj);
        let ptr = &mut *thing as *mut T;

        forget(thing);

        Gc {
            obj: unsafe { NonNull::new_unchecked(ptr) },
        }
    }

    unsafe fn as_ref<'scope>(&self) -> &'scope T {
        &*self.obj.as_ptr()
    }
}

// Allowed types of managed pointers, as unsafe as Gc<T>
#[derive(Copy, Clone)]
enum FatPtr {
    Nil,
    Str(Gc<String>),
    Int(Gc<i64>),
}

impl FatPtr {
    /// This is very unsafe, it depends on the FatPtr Gc instances to be valid pointers
    unsafe fn as_value<'scope>(&self) -> Value<'scope> {
        match self {
            FatPtr::Nil => Value::Nil,
            FatPtr::Str(s) => Value::Str(s.as_ref()),
            FatPtr::Int(i) => Value::Int(*i.as_ref()),
        }
    }
}

impl From<Gc<String>> for FatPtr {
    fn from(ptr: Gc<String>) -> FatPtr {
        FatPtr::Str(ptr)
    }
}

impl From<Gc<i64>> for FatPtr {
    fn from(ptr: Gc<i64>) -> FatPtr {
        FatPtr::Int(ptr)
    }
}

/// A fully safe, immutable interface to FatPtr values.
#[derive(Copy, Clone)]
enum Value<'scope> {
    Nil,
    Str(&'scope str),
    Int(i64)
}

impl<'scope> fmt::Display for Value<'scope> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "Nil"),
            Value::Int(i) => write!(f, "{}", i),
            Value::Str(s) => write!(f, "{}", s),
        }
    }
}

// A GC managed scope-limited pointer
#[derive(Copy, Clone)]
struct ScopedPtr<'guard, 'env: 'guard> {
    ptr: FatPtr,
    value: Value<'guard>,
    _mkr: PhantomData<&'guard EnvScopeGuard<'env>>,
}

impl<'guard, 'env> ScopedPtr<'guard, 'env> {
    fn new(_guard: &'guard EnvScopeGuard<'env>, thing: FatPtr) -> ScopedPtr<'guard, 'env> {
        ScopedPtr {
            ptr: thing,
            value: unsafe { thing.as_value() },
            _mkr: PhantomData,
        }
    }
}

impl<'guard, 'env> Deref for ScopedPtr<'guard, 'env> {
    type Target = Value<'guard>;

    fn deref(&self) -> &Value<'guard> {
        &self.value
    }
}

impl<'guard, 'env> fmt::Display for ScopedPtr<'guard, 'env> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

// Addition function
fn add<'guard, 'env>(env: &'guard EnvScopeGuard<'env>,
                     a: ScopedPtr<'guard, 'env>,
                     b: ScopedPtr<'guard, 'env>)
                     -> Result<ScopedPtr<'guard, 'env>, ()> {
    match *a {
        Value::Int(i) => {
            if let Value::Int(j) = *b {
                let result = env.alloc(i + j);
                Ok(result)
            } else {
                Err(())
            }
        },

        Value::Str(s) => {
            if let Value::Str(t) = *b {
                let result = env.alloc(format!("{}{}", s, t));
                Ok(result)
            } else {
                Err(())
            }
        }

        _ => Err(())
    }
}

fn mutator(env: &mut EnvScopeGuard) {
    let name = env.alloc(String::from("Bob"));

    let age = env.alloc(38);
    let now = env.alloc(2019);

    if let Ok(result) = add(env, age, now) {
        env.set_reg(1, result);
        println!("age years from now will be {}", result);
    }

    println!("name={} age={}", name, age);
}

// A thing to limit moveability and lifetime of root pointers
struct EnvScopeGuard<'env> {
    env: &'env Environment
}

impl<'env> EnvScopeGuard<'env> {
    fn new(env: &'env Environment) -> EnvScopeGuard {
        EnvScopeGuard { env }
    }

    fn get_reg(&self, reg: usize) -> ScopedPtr<'_, 'env> {
        ScopedPtr::new(self, self.env.get_reg(reg))
    }

    fn set_reg(&self, reg: usize, ptr: ScopedPtr<'_, 'env>) {
        self.env.set_reg(reg, ptr.ptr);
    }

    fn alloc<T>(&self, object: T) -> ScopedPtr<'_, 'env>
    where
        FatPtr: From<Gc<T>>
    {
        ScopedPtr::new(self, self.env.alloc(object))
    }

    fn alloc_into_reg<T>(&self, reg: usize, object: T) -> ScopedPtr<'_, 'env>
    where
        FatPtr: From<Gc<T>>
    {
        ScopedPtr::new(self, self.env.alloc_into_reg(reg, object))
    }
}

// A minimal pretend GC environment
struct Environment {
    regs: Vec<Cell<FatPtr>>,
}

impl Environment {
    fn new() -> Environment {
        let capacity = 256;

        let mut regs = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            regs.push(Cell::new(FatPtr::Nil));
        }

        Environment {
            regs: regs
        }
    }

    fn get_reg(&self, reg: usize) -> FatPtr {
        self.regs[reg].get()
    }

    fn set_reg(&self, reg: usize, ptr: FatPtr) {
        self.regs[reg].set(ptr);
    }

    // Heap-allocate an unrooted object
    fn alloc<T>(&self, object: T) -> FatPtr
    where
        FatPtr: From<Gc<T>>
    {
        FatPtr::from(Gc::new(object))
    }

    // Allocate an object and store it's pointer into the specified register number
    fn alloc_into_reg<T>(&self, reg: usize, object: T) -> FatPtr
    where
        FatPtr: From<Gc<T>>
    {
        let ptr = FatPtr::from(Gc::new(object));
        self.regs[reg].set(ptr);
        ptr
    }

    fn mutate<F>(&self, f: F) where F: Fn(&mut EnvScopeGuard) {
        let mut guard = EnvScopeGuard::new(self);
        f(&mut guard);
    }
}

// Do stuff
fn main() {
    let env = Environment::new();

    env.mutate(mutator);
}
