use std::iter::Peekable;

use environment::Environment;
use error::ParseError;
use lexer::{tokenize, Token, TokenType};
use memory::{Allocator, Ptr};
use symbolmap::SymbolMapper;
use types::{Pair, Value};


// I implemented a Linked List! This type is internal to the parser to
// simplify the code and is not stored in managed memory.
struct PairList<'a, A: 'a + Allocator> {
    head: Option<Ptr<'a, Pair<'a, A>, A>>,
    tail: Option<Ptr<'a, Pair<'a, A>, A>>,
}


impl<'a, A: 'a + Allocator> PairList<'a, A> {
    /// Create a new empty list
    fn open() -> PairList<'a, A> {
        PairList {
            head: None,
            tail: None,
        }
    }

    /// Move the given value to managed memory and append it to the list
    fn push(&mut self, value: Value<'a, A>, mem: &'a A)
    {
        if let Some(mut old_tail) = self.tail {
            let new_tail = old_tail.append(value, mem);
            self.tail = Some(new_tail);
        } else {
            let mut pair = mem.alloc(Pair::new());
            pair.set(value);
            self.head = Some(pair);
            self.tail = self.head;
        }
    }

    /// Apply dot-notation to set the second value of the last pair of the list
    fn dot(&mut self, value: Value<'a, A>) {
        if let Some(mut old_tail) = self.tail {
            old_tail.dot(value);
        } else {
            panic!("cannot dot an empty PairList!")
        }
    }

    /// Consume the list and return the pair at the head
    fn close(self) -> Ptr<'a, Pair<'a, A>, A> {
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
fn parse_list<'i, 'a, I, A>(tokens: &mut Peekable<I>,
                            env: &'a Environment<'a, A>) -> Result<Value<'a, A>, ParseError>
    where I: Iterator<Item = &'i Token>,
          A: 'a + Allocator
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
                list.push(parse_list(tokens, env)?, &env.mem);
            }

            Some(&&Token { token: Symbol(ref name), pos }) => {
                tokens.next();
                let sym = env.syms.lookup(name);
                list.push(Value::Symbol(sym, pos), &env.mem);
            }

            Some(&&Token { token: Dot, pos }) => {
                // the only valid sequence here on out is Dot s-expression CloseParen
                tokens.next();
                list.dot(parse_sexpr(tokens, env)?);

                match tokens.peek() {
                    Some(&&Token { token: CloseParen, pos: _ }) => (),
                    _ => {
                        return Err(ParseError::new(pos,
                                                   String::from("s-expr after . must be \
                                                                 followed by close parenthesis")))
                    }
                }
            }

            Some(&&Token { token: CloseParen, pos: _ }) => {
                tokens.next();
                break;
            }

            None => {
                return Err(ParseError::new((0, 0), String::from("unexpected end of stream")));
            }
        }
    }

    Ok(Value::Pair(list.close()))
}


// Parse a single s-expression
fn parse_sexpr<'i, 'a, I, A>(tokens: &mut Peekable<I>,
                             env: &'a Environment<'a, A>) -> Result<Value<'a, A>, ParseError>
    where I: Iterator<Item = &'i Token>,
          A: 'a + Allocator
{
    use self::TokenType::*;

    match tokens.peek() {
        Some(&&Token { token: OpenParen, pos: _ }) => {
            tokens.next();
            parse_list(tokens, env)
        }

        Some(&&Token { token: Symbol(ref name), pos }) => {
            tokens.next();
            let sym = env.syms.lookup(name);
            Ok(Value::Symbol(sym, pos))
        }

        Some(&&Token { token: CloseParen, pos }) => {
            Err(ParseError::new(pos, String::from("unmatched close parenthesis")))
        }

        Some(&&Token { token: Dot, pos }) => {
            Err(ParseError::new(pos, String::from("invalid symbol '.'")))
        }

        None => {
            tokens.next();
            Ok(Value::Nil)
        }
    }
}


fn parse_tokens<'a, A>(tokens: Vec<Token>,
                       env: &'a Environment<'a, A>) -> Result<Value<'a, A>, ParseError>
    where A: 'a + Allocator
{
    let mut tokenstream = tokens.iter().peekable();
    parse_sexpr(&mut tokenstream, env)
}


pub fn parse<'a, A>(input: String,
                    env: &'a Environment<'a, A>) -> Result<Value<'a, A>, ParseError>
    where A: 'a + Allocator
{
    parse_tokens(tokenize(input)?, env)
}


#[cfg(test)]
mod test {
    use super::*;
    use environment::Environment;
    use printer::print;

    fn check(input: String, expect: String) {
        let mut env = Environment::new(1024);
        let ast = parse(input, &mut env).unwrap();
        println!("expect: {}\n\tgot:    {}\n\tdebug:  {:?}",
                 &expect,
                 &ast,
                 &ast);
        assert!(print(&ast) == expect);
    }

    #[test]
    fn parse_empty_list() {
        let input = String::from("()");
        let expect = input.clone();
        check(input, expect);
    }

    #[test]
    fn parse_symbol() {
        let input = String::from("a");
        let expect = input.clone();
        check(input, expect);
    }

    #[test]
    fn parse_list() {
        let input = String::from("(a)");
        let expect = input.clone();
        check(input, expect);
    }

    #[test]
    fn parse_list_nested1() {
        let input = String::from("((a))");
        let expect = input.clone();
        check(input, expect);
    }

    #[test]
    fn parse_list_nested2() {
        let input = String::from("(a (b c) d)");
        let expect = input.clone();
        check(input, expect);
    }

    #[test]
    fn parse_list_nested3() {
        let input = String::from("(a b (c (d)))");
        let expect = input.clone();
        check(input, expect);
    }

    #[test]
    fn parse_longer_list() {
        let input = String::from("(a b c)");
        let expect = input.clone();
        check(input, expect);
    }

    #[test]
    fn parse_dot_notation() {
        let input = String::from("(a . b)");
        let expect = input.clone();
        check(input, expect);
    }

    #[test]
    fn parse_dot_notation_longer() {
        let input = String::from("((a . b) . (c . d))");
        let expect = String::from("((a . b) c . d)");
        check(input, expect);
    }

    #[test]
    fn parse_dot_notation_with_nil() {
        let input = String::from("(a . ())");
        let expect = String::from("(a)");
        check(input, expect);
    }
}
