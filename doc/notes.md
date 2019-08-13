# Notes

## Semantics

Glue language:
 - Immutable values and data structures, structural sharing
 - Mutable synchronized variables?
 - Mutable stack-pinned resource/state/io management - 'with'
 - Closures with upvalues
 - Coroutines
 - Currying
 - Tail call recursion (continuations)
 - Duck typed? Traits/interfaces
 - Types: scalar, tuple, record, enum, map, array, list
 - Pattern matching, destructuring

 Function Prototype Definition:
  - Arity
  - Free variable symbol list
  - Code

Closure:
 - Function prototype pointer
 - Upvalues

Partial Application:
 - Closure pointer
 - Arity
 - Supplied parameters

Coroutine:
 - Arity
 - ReEntry pointer
 - Stack with 1 Activation Record

Activation Record?:
 - Closure definition pointer
 - Register bank <- Supplied parameters starting in reg 0

Activation Record?:
 - Return pointer
 - Dynamic pointer (parent base pointer)
 - Static pointer  (parent scope pointer)
 - Return value
 - Parameters


 ## Virtual Machine

### eval/apply

eval to identity:
 - pap -> self
 - closure -> self
 - value -> self

apply:
 - pap + eval(args) -> value
 - closure + eval(args) -> value

Bytecode:

### v1
 - lookup-sym reg str
 - atom reg = atom x
 - quote reg = quote x
 - car reg = car x:xs
 - cdr reg = cdr x:xs
 - cons reg = x:xs
 - eq reg = x==y
 - jump-if-false reg offset
 - jump offset

### v2
 - call sym(fn)
 - return reg

 ### v3
 - make-closure
 - get-upvalue
 - set-upvalue
 - close-upvalue

 ### vn
 - tailcall/cont sym(fn)
 - construct sym(type)
 - cons-str reg str
 - cons-int reg lit
 - match
 - with sym(fn) reg: pin to stack


## Syntax - easy to parse but unergonomic s-exprs

(defn fib (n)
    (match n
        ((0) 1)
        ((n) (+ n (fib (- n 1))))
    )
)

(defn function (x y)
    (let (variable expr)
        (* x y)
    )
)

(defn mult () function)
(mult 2 3)

(let (square (lambda (x) (* x x))))

(with (io.file "name" 'r) f
    (let (content (f.read)))
)

(deftype Option (
    (Some (value))
    (None)
))



# --- samples ---

/// This represents a pointer to a window of registers on the stack.
/// A mutator-lifetime limited view
struct ActivationFramePtr<'guard> {
    regs: &'guard [CellPtr; 256],

}
