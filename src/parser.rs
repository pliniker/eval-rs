
use error::{ParseError, SourcePos};
use lexer::{tokenize, Token, TokenType};
use memory::{Arena, Ptr};


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


fn expression<'a, I>(mem: &mut Arena, tokens: &mut I) -> Result<Value, ParseError>
    where I: Iterator<Item = Token>
{
    use self::TokenType::*;

    let mut token = tokens.next();

    // immediate close paren means empty-list/nil
    if let Some(Token { token: CloseParen, pos: _ }) = token {
        return Ok(Value::Nil);
    }

    // otherwise this is a list
    let head = Pair::alloc(mem);
    // make a tail to append things into
    let mut tail = head;

    // loop state variables
    let mut first_token = true;
    let mut after_dot = false;
    let mut expect_closeparen = false;
    let mut expect_list = true;

    loop {
        println!("{:?}", token);
        match token {
            // Open parenthesis
            Some(Token { token: OpenParen, pos }) => {
                if expect_closeparen {
                    return Err(ParseError::new(
                        pos, String::from("expected close-paren")));
                }

                if first_token {
                    tail.set(expression(mem, tokens)?);
                    first_token = false;
                } else if after_dot {
                    tail.dot(expression(mem, tokens)?);
                    expect_closeparen = true;
                } else {
                    let expr = expression(mem, tokens)?;
                    tail = tail.append(mem, expr);
                }
            },

            // Symbol
            Some(Token { token: Symbol(sym), pos }) => {
                if expect_closeparen {
                    return Err(ParseError::new(
                        pos, String::from("expected close-paren")));
                }

                if first_token {
                    tail.set(Value::Symbol(pos));
                    first_token = false;
                } else if after_dot {
                    tail.dot(Value::Symbol(pos));
                    expect_closeparen = true;
                } else {
                    tail = tail.append(mem, Value::Symbol(pos));
                }
            },

            // Dot: something . something)
            Some(Token { token: Dot, pos }) => {
                if expect_closeparen {
                    return Err(ParseError::new(
                        pos, String::from("expected close-paren")));
                }

                after_dot = true;
            }

            // Close parenthesis
            Some(Token { token: CloseParen, pos: _ }) => {
                expect_list = false;
                break;
            }

            // end of tokens
            None => {
                if expect_list {
                    return Err(ParseError::new(
                        (0, 0), String::from("unexpected end of stream")));
                } else {
                    break;
                }
            },
        }

        token = tokens.next();
    }

    Ok(Value::Pair(head))
}


fn parse_tokens(mem: &mut Arena, mut tokens: Vec<Token>) -> Result<Value, ParseError> {
    let mut iter = tokens.drain(..);
    expression(mem, &mut iter)
}


pub fn parse(mem: &mut Arena, input: String) -> Result<Value, ParseError> {
    parse_tokens(mem, tokenize(input)?)
}
