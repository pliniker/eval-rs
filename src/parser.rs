use std::iter::Peekable;

use error::{ParseError, SourcePos};
use lexer::{tokenize, Token, TokenType};
use memory::{Arena, Ptr};
use types::{Pair, Value};


// I implemented a Linked List! This type is internal to the parser to
// simplify the code.
struct PairList {
    head: Option<Ptr<Pair>>,
    tail: Option<Ptr<Pair>>
}


impl PairList {
    fn open() -> PairList {
        PairList { head: None, tail: None }
    }

    fn push(&mut self, value: Value, mem: &mut Arena) {
        if let Some(mut old_tail) = self.tail {
            let new_tail = old_tail.append(mem, value);
            self.tail = Some(new_tail);
        } else {
            let mut pair = Pair::alloc(mem);
            pair.set(value);
            self.head = Some(pair);
            self.tail = self.head;
        }
    }

    fn dot(&mut self, value: Value) {
        if let Some(mut old_tail) = self.tail {
            old_tail.dot(value);
        } else {
            panic!("cannot dot an empty PairList!")
        }
    }

    fn close(self) -> Ptr<Pair> {
        self.head.expect("cannot close empty PairList!")
    }
}


//
// A list is either
// * empty
// * a sequence of s-expressions
//
// If a list token is a Dot, it must be followed by an s-expression and a CloseParen
//
fn parse_list<'a, I>(mem: &mut Arena, tokens: &mut Peekable<I>) -> Result<Value, ParseError>
    where I: Iterator<Item = &'a Token>
{
    use self::TokenType::*;

    if let Some(&&Token { token: CloseParen, pos: _ }) = tokens.peek() {
        tokens.next();
        return Ok(Value::Nil);
    }

    let mut list = PairList::open();

    loop {
        match tokens.peek() {
            Some(&&Token { token: OpenParen, pos: _ }) => {
                tokens.next();
                list.push(parse_list(mem, tokens)?, mem);
            },

            Some(&&Token { token: Symbol(ref _sym), pos }) => {
                tokens.next();
                list.push(Value::Symbol(pos), mem);
            },

            Some(&&Token { token: Dot, pos }) => {
                // the only valid sequence here on out is Dot s-expression CloseParen
                tokens.next();
                list.dot(parse_sexpr(mem, tokens)?);

                match tokens.peek() {
                    Some(&&Token { token: CloseParen, pos: _ }) => (),
                    _ => return Err(ParseError::new(pos, String::from("s-expr after . must be followed by close parenthesis")))
                }
            },

            Some(&&Token { token: CloseParen, pos: _ }) => {
                tokens.next();
                break;
            },

            None => {
                return Err(ParseError::new((0, 0), String::from("unexpected end of stream")));
            }
        }
    }

    Ok(Value::Pair(list.close()))
}


// Parse a single s-expression
fn parse_sexpr<'a, I>(mem: &mut Arena, tokens: &mut Peekable<I>) -> Result<Value, ParseError>
    where I: Iterator<Item = &'a Token>
{
    use self::TokenType::*;

    match tokens.peek() {
        Some(&&Token { token: OpenParen, pos: _ }) => {
            tokens.next();
            parse_list(mem, tokens)
        },

        Some(&&Token { token: Symbol(ref _sym), pos }) => {
            tokens.next();
            Ok(Value::Symbol(pos))
        },

        Some(&&Token { token: CloseParen, pos })
            => Err(ParseError::new(pos, String::from("unmatched close parenthesis"))),

        Some(&&Token { token: Dot, pos })
            => Err(ParseError::new(pos, String::from("invalid symbol '.'"))),

        None => {
            tokens.next();
            Ok(Value::Nil)
        }
    }
}


fn parse_tokens(mem: &mut Arena, tokens: Vec<Token>) -> Result<Value, ParseError> {
    let mut tokenstream = tokens.iter().peekable();
    parse_sexpr(mem, &mut tokenstream)
}


pub fn parse(mem: &mut Arena, input: String) -> Result<Value, ParseError> {
    parse_tokens(mem, tokenize(input)?)
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_empty_list() {
        let mut mem = Arena::new(256);
        if let Ok(Value::Nil) = parse(&mut mem, String::from("()")) {
            // ok
        } else {
            assert!(false);
        }
    }

    #[test]
    fn parse_symbol() {
        let mut mem = Arena::new(256);
        if let Ok(Value::Symbol(_)) = parse(&mut mem, String::from("a")) {
            // ok
        } else {
            assert!(false);
        }
    }

    #[test]
    fn parse_list() {
        let mut mem = Arena::new(256);
        if let Ok(have) = parse(&mut mem, String::from("(a)")) {
            assert!(have ==
                    Value::Pair(Pair::alloc_with_first(
                        &mut mem,
                        Value::Symbol((0, 0)))));
        } else {
            assert!(false);
        }
    }

    #[test]
    fn parse_nested_list() {
        let mut mem = Arena::new(256);
        if let Ok(have) = parse(&mut mem, String::from("((a))")) {

            let inner = Value::Pair(Pair::alloc_with_first(
                &mut mem,
                Value::Symbol((0, 0))));

            assert!(have ==
                    Value::Pair(Pair::alloc_with_first(
                        &mut mem,
                        inner)))
        } else {
            assert!(false);
        }
    }

    #[test]
    fn parse_longer_list() {
        let mut mem = Arena::new(256);
        if let Ok(have) = parse(&mut mem, String::from("(a b c)")) {
            let c = Value::Pair(Pair::alloc_with_first(&mut mem, Value::Symbol((0, 0))));
            let b = Value::Pair(Pair::alloc_with_both(&mut mem, Value::Symbol((0, 0)), c));
            let a = Value::Pair(Pair::alloc_with_both(&mut mem, Value::Symbol((0, 0)), b));

            assert!(have == a);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn parse_dot_notation() {
        let mut mem = Arena::new(256);
        if let Ok(have) = parse(&mut mem, String::from("(a . b)")) {
            assert!(have == Value::Pair(Pair::alloc_with_both(&mut mem, Value::Symbol((0, 0)), Value::Symbol((0, 0)))));
        } else {
            assert!(false);
        }
    }

    // TODO add expect-to-fail test
}
