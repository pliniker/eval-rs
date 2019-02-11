use std::iter::Peekable;

use crate::error::{err_parser, err_parser_wpos, spos, RuntimeError, SourcePos};
use crate::heap::Heap;
use crate::lexer::{tokenize, Token, TokenType};
use crate::primitives::{Pair, Symbol};
use crate::symbolmap::SymbolMapper;
use crate::taggedptr::{FatPtr, TaggedPtr};

use stickyimmix::{AllocRaw, RawPtr};

// A linked list, internal to the parser to simplify the code and is not stored in managed memory
struct PairList {
    head: Option<RawPtr<Pair>>,
    tail: Option<RawPtr<Pair>>,
}

impl PairList {
    /// Create a new empty list
    fn open() -> PairList {
        PairList {
            head: None,
            tail: None,
        }
    }

    /// Move the given value to managed memory and append it to the list
    fn push(&mut self, value: FatPtr, pos: SourcePos, heap: &Heap) {
        if let Some(mut old_tail) = self.tail {
            let mut new_tail = old_tail.append(heap, value);
            self.tail = Some(new_tail);
            // set source code line/char
            new_tail.set_first_source_pos(pos);
            old_tail.set_second_source_pos(pos);
        } else {
            let mut pair = heap.alloc(Pair::new());
            pair.set(value);
            self.head = Some(pair);
            self.tail = self.head;
            // set source code line/char
            pair.set_first_source_pos(pos);
            pair.set_second_source_pos(pos);
        }
    }

    /// Apply dot-notation to set the second value of the last pair of the list
    fn dot(&mut self, value: FatPtr, pos: SourcePos) {
        if let Some(mut old_tail) = self.tail {
            old_tail.dot(value);
            // set source code line/char
            old_tail.set_second_source_pos(pos);
        } else {
            panic!("cannot dot an empty PairList!")
        }
    }

    /// Consume the list and return the pair at the head
    fn close(self) -> RawPtr<Pair> {
        self.head.expect("cannot close empty PairList!")
    }
}

//
// A list is either
// * empty
// * a sequence of s-expressions
//
// If the first list token is:
//  * a CloseParen, it's a Nil value
//  * a Dot, this is illegal
//
// If a list token is:
//  * a Dot, it must be followed by an s-expression and a CloseParen
//
fn parse_list<'i, I: 'i>(tokens: &mut Peekable<I>, env: &Heap) -> Result<FatPtr, RuntimeError>
where
    I: Iterator<Item = &'i Token>,
{
    use self::TokenType::*;

    // peek at very first token after the open-paren
    match tokens.peek() {
        Some(&&Token {
            token: CloseParen,
            pos: _,
        }) => {
            tokens.next();
            return Ok(FatPtr::Nil);
        }

        Some(&&Token { token: Dot, pos }) => {
            return Err(err_parser_wpos(
                pos,
                "Unexpected '.' dot after open-parenthesis",
            ));
        }

        _ => (),
    }

    // we have what looks like a valid list so far...
    let mut list = PairList::open();
    loop {
        match tokens.peek() {
            Some(&&Token {
                token: OpenParen,
                pos,
            }) => {
                tokens.next();
                list.push(parse_list(tokens, env)?, pos, &env.heap);
            }

            Some(&&Token {
                token: Symbol(ref name),
                pos,
            }) => {
                tokens.next();
                let sym = env.syms.lookup(name);
                list.push(FatPtr::Symbol(sym), pos, &env.heap);
            }

            Some(&&Token { token: Dot, pos }) => {
                tokens.next();
                list.dot(parse_sexpr(tokens, env)?, pos);

                // the only valid sequence here on out is Dot s-expression CloseParen
                match tokens.peek() {
                    Some(&&Token {
                        token: CloseParen,
                        pos: _,
                    }) => (),

                    Some(&&Token { token: _, pos }) => {
                        return Err(err_parser_wpos(
                            pos,
                            "Dotted pair must be closed by a ')' close-parenthesis",
                        ));
                    }

                    None => return Err(err_parser("Unexpected end of code stream")),
                }
            }

            Some(&&Token {
                token: CloseParen,
                pos: _,
            }) => {
                tokens.next();
                break;
            }

            None => {
                return Err(err_parser("Unexpected end of code stream"));
            }
        }
    }

    Ok(FatPtr::Pair(list.close()))
}

//
// Parse a single s-expression
//
// Must be a
//  * symbol
//  * or a list
//
fn parse_sexpr<'i, I: 'i>(tokens: &mut Peekable<I>, env: &Heap) -> Result<FatPtr, RuntimeError>
where
    I: Iterator<Item = &'i Token>,
{
    use self::TokenType::*;

    match tokens.peek() {
        Some(&&Token {
            token: OpenParen,
            pos: _,
        }) => {
            tokens.next();
            parse_list(tokens, env)
        }

        Some(&&Token {
            token: Symbol(ref name),
            pos: _,
        }) => {
            tokens.next();
            let sym = env.syms.lookup(name);
            Ok(FatPtr::Symbol(sym))
        }

        Some(&&Token {
            token: CloseParen,
            pos,
        }) => Err(err_parser_wpos(pos, "Unmatched close parenthesis")),

        Some(&&Token { token: Dot, pos }) => Err(err_parser_wpos(pos, "Invalid symbol '.'")),

        None => {
            tokens.next();
            Ok(FatPtr::Nil)
        }
    }
}

fn parse_tokens(tokens: Vec<Token>, env: &Heap) -> Result<FatPtr, RuntimeError> {
    let mut tokenstream = tokens.iter().peekable();
    parse_sexpr(&mut tokenstream, env)
}

pub fn parse(input: &str, env: &Heap) -> Result<FatPtr, RuntimeError> {
    parse_tokens(tokenize(input)?, env)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::arena::Arena;
    use crate::memory::Memory;
    use crate::printer::print;

    fn check(input: &str, expect: &str) {
        let heap = Arena::new(1024);
        let mut env = Memory::new(&heap);
        let ast = parse(input, &mut env).unwrap();
        println!(
            "expect: {}\n\tgot:    {}\n\tdebug:  {:?}",
            &expect, &ast, &ast
        );
        assert!(print(&ast) == expect);
    }

    #[test]
    fn parse_empty_list() {
        let input = String::from("()");
        let expect = input.clone();
        check(&input, &expect);
    }

    #[test]
    fn parse_symbol() {
        let input = String::from("a");
        let expect = input.clone();
        check(&input, &expect);
    }

    #[test]
    fn parse_list() {
        let input = String::from("(a)");
        let expect = input.clone();
        check(&input, &expect);
    }

    #[test]
    fn parse_list_nested1() {
        let input = String::from("((a))");
        let expect = input.clone();
        check(&input, &expect);
    }

    #[test]
    fn parse_list_nested2() {
        let input = String::from("(a (b c) d)");
        let expect = input.clone();
        check(&input, &expect);
    }

    #[test]
    fn parse_list_nested3() {
        let input = String::from("(a b (c (d)))");
        let expect = input.clone();
        check(&input, &expect);
    }

    #[test]
    fn parse_longer_list() {
        let input = String::from("(a b c)");
        let expect = input.clone();
        check(&input, &expect);
    }

    #[test]
    fn parse_dot_notation() {
        let input = String::from("(a . b)");
        let expect = input.clone();
        check(&input, &expect);
    }

    #[test]
    fn parse_dot_notation_longer() {
        let input = String::from("((a . b) . (c . d))");
        let expect = String::from("((a . b) c . d)");
        check(&input, &expect);
    }

    #[test]
    fn parse_dot_notation_with_nil() {
        let input = String::from("(a . ())");
        let expect = String::from("(a)");
        check(&input, &expect);
    }
}
