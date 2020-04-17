# Notes

## TODO

### Source Maps

 - Copy source code into Text objects?
 - reference source Text objects from Function objects?
 - keep source code map on Function objects
 - propagate source code pos objects through to Function object source maps
 - print pretty stack traces

### Compiler

 - unit tests! :grimace:
 - function parameters
 - partial functions
 - lambdas
 - let
 - tail calls

### Types

 - dicts
 - lists
 - sets
 - record
 - enum

## Semantics

Glue language:
 - Immutable values and data structures, structural sharing
 - Mutable unique copy-on-write
 - Mutable concurrency-safe variables?
 - Mutable stack-pinned resource/state/io management - 'with' (needs escape analysis?)
 - Closures with lambda lifting
 - Partial application
 - Tail call recursion
 - Duck typed? Traits/interfaces
 - Pattern matching, destructuring?

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


## Syntax - easy to parse but unergonomic s-exprs

### v1

(atom sym)
(quote thing)
(car (quote (list of things)))
(cdr (quote (list of things)))
(cons thing (quote (list of things)))
(eq thing1 thing2)

### v2

(def fib (n)
    (match n
        ((0) 1)
        ((n) (+ n (fib (- n 1))))))

(def function (x y)
    (let (variable expr)
        (* x y)))

(def mult () function)
(mult 2 3)

(let (square (lambda (x) (* x x))))

(with (io.file "name" 'r) f
    (let (content (f.read))))

(deftype Option (
    (Some (value))
    (None)))

### Partials

(def addn (a) (+ a)) -> (Partial + a b)
(def muln (x) (* x)) -> (partial * x b)

Chaining partials?
(Partial add 3 (Partial mul 5)) -> (Partial mul 5 b)~>(Partial add 3 b)

### Functors, Applicative, Monads

data Maybe = Just a | Nothing

instance Functor Maybe where
  fmap :: (a -> b) -> Maybe a -> Maybe b
  fmap _ Nothing  = Nothing
  fmap g (Just a) = Just (g a)

---

(data Maybe 
    (Just a)
    (Nothing))
->
(def Maybe '(Maybe (Just a) (Nothing)))
(def Just (a) '(Maybe Just `a))
(def Nothing '(Maybe Nothing))

(def fmap (f a) (f a))
(def fmap (f (Maybe Just a)) (Just (f a)))
(def fmap (f (Maybe Nothing)) (Nothing))

(fmap (+ 3) (Just 2)) -> '(Maybe Just 6)
(fmap (+ 3) (+ 4)) -> (Partial + 3)~>(Partial + 4)
