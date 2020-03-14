# Notes

## Semantics

Glue language:
 - Immutable values and data structures, structural sharing
 - Mutable concurrency-safe variables?
 - Mutable stack-pinned resource/state/io management - 'with' (needs escape analysis?)
 - Closures with lambda lifting
 - Partial application
 - Tail call recursion (continuations)
 - Duck typed? Traits/interfaces
 - Types: scalar, tuple, record, enum, map, array, list
 - Pattern matching, destructuring
 - Coroutines?

 Function Prototype Definition (lambda lifted):
  - Name
  - Arity
  - Code
  - Docstring

Partial Application:
 - Arity
 - Parameter Count
 - Parameter value list
 - Function Prototype Definition

Coroutine: (?)
 - Next arity
 - Activation Record
 - Registers

Activation Record Stack:
 - Function Prototype Definition pointer
 - IP
 - Stack base pointer

 Register Stack
 - return value [0]
 - parameters [1..n]
 - registers [n..255]


## Virtual Machine

### eval/apply

eval:
 - quote x -> x
 - list -> apply sym args
 - pap -> self
 - closure -> self
 - int -> int
 - nil -> nil
 - true -> true

apply:
 - sym + eval(args) -> pap args | closure
 - pap + eval(args) -> pap args | value
 - closure + eval(args) -> value

Bytecode:

### v1
 - load reg sym
 - load reg int
 - load reg pair
 - atom regr regx: regr = atom regx
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

### vn
 - tailcall/cont sym(fn)
 - construct sym(type)
 - cons-str reg str
 - cons-int reg lit
 - match
 - with sym(fn) reg: pin to stack


## Syntax - easy to parse but unergonomic s-exprs

### v1
(atom sym)
(quote thing)
(car (quote (list of things)))
(cdr (quote (list of things)))
(cons thing (quote (list of things)))
(eq thing1 thing2)

### vn
(defn fib (n)
    (match n
        ((0) 1)
        ((n) (+ n (fib (- n 1))))))

(defn function (x y)
    (let (variable expr)
        (* x y)))

(defn mult () function)
(mult 2 3)

(let (square (lambda (x) (* x x))))

(with (io.file "name" 'r) f
    (let (content (f.read))))

(deftype Option (
    (Some (value))
    (None)))



### Type System

Trait based?  Duck typed?

Dispatch?
 * Single dispatch: x.y() === X::y(x)


### Lambda lifting and partial applications

def make_adder(x):
    def adder(y):
        return x + y
    return adder

becomes

def adder(x, y):
    return x + y

def make_adder(x):
    return partial(adder, x)
