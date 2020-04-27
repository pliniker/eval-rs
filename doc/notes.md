# Notes

## TODO

### Source Maps

 - Copy source code into Text objects?
 - reference source Text objects from Function objects?
 - keep source code map on Function objects
 - propagate source code pos objects through to Function object source maps
 - print pretty stack traces

### Compiler

 - tail calls
 - closures
 - integer math
 - arbitrary sized integers

### Types

 - object
 - dicts #{...}
 - lists (arrays) [...]
 - sets &{...}
 - record {...}
 - tuple &(...)
 - cons lists (...)
 - Custom types based on symbol combinations
 - Pattern matching

## Semantics

Glue language:
 - Immutable values and data structures, structural sharing
 - Mutable unique copy-on-write
 - Mutable concurrency-safe variables?
 - Mutable stack-pinned resource/state/io management - 'with' (needs escape analysis?)
 - Closures with lambda lifting
 - Partial application
 - Tail call recursion
 - Duck typed?
 - Pattern matching, destructuring?

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

(with (io.file "name" 'r) f
  (let 
    ((content (f.read)))
    content))

(data Option
  (Some value)
  (None))

### Partials and Currying

(def addn (a) (+ a)) -> (Partial + a b)
(def muln (x) (* x)) -> (partial * x y)

Chaining partials? Stack of partials?
(Partial div 2 (Partial add 3 (Partial mul 5)))
->
(Partial mul 5)
(Partial add 3)
(Partial div 2)
(PartialStack)

(<PartialStack> 3)
(mul 5 3): pop 3, push 15
(add 3 15): pop 15, push 18
(div 2 18): pop 18, push 9
(pop 9)

iterate until arg stack is empty

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
(set 'Maybe (object))
(def Just (a) (append '(Maybe Just) (list a))
(def Nothing () '(Maybe Nothing))

(def Maybe::fmap (self f)
  (match self
    (Nothing) (Nothing)
    (Just a) (Just (f a))))

(def Maybe::amap (self mf)
  (match self
    (Nothing) (Nothing)
    (Just a) (match mf
      (Nothing) (Nothing)
      (Just f) (Just (f a))
)))


(let 
  (
    (is (lambda (a b) (is? a b)))
    (is_not (lambda (x y) (is x y)))
  )
  (is_not 'a 'b)
)
