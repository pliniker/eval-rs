use std::mem;

use error::{ParseError, SourcePos};
use lexer::{tokenize, Token, TokenType};
use memory::{Arena, Ptr};


//
// Composition of an iterator over Tokens and the last Token that was returned
// by the iterator, allowing repeated querying of the last Token obtained
// without advancing the iterator.
//
struct TokenStream<I: Iterator<Item = Token>> {
    tokens: I,
    peek: Option<Token>
}


impl<I: Iterator<Item = Token>> TokenStream<I> {
    fn new(mut tokens: I) -> TokenStream<I> {
        let peek = tokens.next();
        TokenStream {
            tokens: tokens,
            peek: peek
        }
    }

    // Peek at the next token in the stream. This can be called repeatedly
    // without advancing the position in the stream.
    fn peek(&self) -> &Option<Token> {
        &self.peek
    }

    // Move the token we're peeking at to the caller and get the next token
    // to peek at.
    fn consume(&mut self) -> Option<Token> {
        let mut value = self.tokens.next();
        mem::swap(&mut value, &mut self.peek);
        println!("{:?}", value);
        value
    }
}


#[derive(Copy, Clone)]
pub enum Value {
//    Symbol(String, SourcePos),
    Symbol(SourcePos),
    Pair(Ptr<Pair>),
    Nil,
}


pub struct Pair {
    pub first: Value,
    pub second: Value,
}


impl Pair {
    pub fn alloc(mem: &mut Arena) -> Ptr<Pair> {
        mem.allocate(Pair {
            first: Value::Nil,
            second: Value::Nil
        })
    }

    pub fn set(&mut self, value: Value) {
        self.first = value
    }

    pub fn dot(&mut self, value: Value) {
        self.second = value
    }

    pub fn append(&mut self, mem: &mut Arena, value: Value) -> Ptr<Pair> {
        let mut pair = Pair::alloc(mem);
        self.second = Value::Pair(pair);
        pair.first = value;
        pair
    }
}


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
fn parse_list<I>(mem: &mut Arena, tokens: &mut TokenStream<I>) -> Result<Value, ParseError>
    where I: Iterator<Item = Token>
{
    use self::TokenType::*;

    if let &Some(Token { token: CloseParen, pos: _ }) = tokens.peek() {
        tokens.consume();
        return Ok(Value::Nil);
    }

    let mut list = PairList::open();

    loop {
        match tokens.peek() {
            &Some(Token { token: OpenParen, pos: _ }) => {
                list.push(parse_list(mem, tokens)?, mem);
                tokens.consume();
            },

            &Some(Token { token: Symbol(ref _sym), pos }) => {
                list.push(Value::Symbol(pos), mem);
                tokens.consume();
            },

            &Some(Token { token: Dot, pos }) => {
                // the only valid sequence here on out is Dot s-expression CloseParen
                list.dot(parse_sexpr(mem, tokens)?);
                tokens.consume();

                match tokens.peek() {
                    &Some(Token { token: CloseParen, pos: _ }) => (),
                    _ => return Err(ParseError::new(pos, String::from("s-expr after . must be followed by close parenthesis")))
                }
            },

            &Some(Token { token: CloseParen, pos: _ }) => {
                tokens.consume();
                break;
            },

            &None => {
                return Err(ParseError::new((0, 0), String::from("unexpected end of stream")));
            }
        }
    }

    Ok(Value::Pair(list.close()))
}


// Parse a single s-expression
fn parse_sexpr<I>(mem: &mut Arena, tokens: &mut TokenStream<I>) -> Result<Value, ParseError>
    where I: Iterator<Item = Token>
{
    use self::TokenType::*;

    match tokens.peek() {
        &Some(Token { token: OpenParen, pos: _ })
            => parse_list(mem, tokens),

        &Some(Token { token: Symbol(ref _sym), pos })
            => Ok(Value::Symbol(pos)),

        &Some(Token { token: CloseParen, pos })
            => Err(ParseError::new(pos, String::from("unmatched close parenthesis"))),

        &Some(Token { token: Dot, pos })
            => Err(ParseError::new(pos, String::from("invalid symbol '.'"))),

        &None => Ok(Value::Nil)
    }
}


fn parse_tokens(mem: &mut Arena, mut tokens: Vec<Token>) -> Result<Value, ParseError> {
    let mut tokenstream = TokenStream::new(tokens.drain(..));
    parse_sexpr(mem, &mut tokenstream)
}


pub fn parse(mem: &mut Arena, input: String) -> Result<Value, ParseError> {
    parse_tokens(mem, tokenize(input)?)
}
