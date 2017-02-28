
use error::{ParseError, SourcePos};


// key characters
const OPEN_PAREN: char = '(';
const CLOSE_PAREN: char = ')';
const SPACE: char = ' ';
const TAB: char = '\t';
const CR: char = '\r';
const LF: char = '\n';
const DOT: char = '.';


#[derive(Debug, PartialEq)]
pub enum TokenType {
    OpenParen,
    CloseParen,
    Symbol(String),
    Dot,
}


#[derive(Debug, PartialEq)]
pub struct Token {
    pub pos: SourcePos,
    pub token: TokenType,
}


impl Token {
    fn new(pos: SourcePos, token: TokenType) -> Token {
        Token {
            pos: pos,
            token: token,
        }
    }

    pub fn token_type(&self) -> &TokenType {
        &self.token
    }
}


// tokenize a String
pub fn tokenize(input: String) -> Result<Vec<Token>, ParseError> {

    use self::TokenType::*;

    // characters that terminate a symbol
    let terminating = [OPEN_PAREN, CLOSE_PAREN, SPACE, TAB, CR, LF];
    let is_terminating = |c: char| terminating.iter().any(|t| c == *t);

    // return value
    let mut tokens = Vec::new();

    // start line numbering at 1, the first character of each line being number 0
    let mut lineno = 1;
    let mut charno = 0;

    let mut chars = input.chars();
    let mut current = chars.next();

    loop {
        match current {
            Some(TAB) =>
                return Err(ParseError::new(
                    (lineno, charno),
                    String::from("tabs are not valid whitespace"))),

            Some(SPACE) => current = chars.next(),

            Some(CR) => {
                current = chars.next();

                // consume \n if it follows \r
                if let Some(LF) = current {
                    current = chars.next();
                }

                lineno += 1;
                charno = 0;
                continue;
            }

            Some(LF) => {
                current = chars.next();
                lineno += 1;
                charno = 0;
                continue;
            }

            // this is not correct because it doesn't allow for a . to begin a number
            // or a symbol. Will have to fix later.
            Some(DOT) => {
                tokens.push(Token::new((lineno, charno), Dot));
                current = chars.next();
            }

            Some(OPEN_PAREN) => {
                tokens.push(Token::new((lineno, charno), OpenParen));
                current = chars.next();
            }

            Some(CLOSE_PAREN) => {
                tokens.push(Token::new((lineno, charno), CloseParen));
                current = chars.next();
            }

            Some(non_terminating) => {
                let symbol_begin = charno;

                let mut symbol = String::from("");
                symbol.push(non_terminating);

                // consume symbol
                loop {
                    current = chars.next();
                    if let Some(c) = current {
                        if is_terminating(c) {
                            break;
                        } else {
                            symbol.push(c);
                            charno += 1;
                        }
                    } else {
                        break;
                    }
                }

                // complete symbol
                tokens.push(Token::new((lineno, symbol_begin), Symbol(symbol)));
            }

            // EOL
            None => break,
        }

        charno += 1;
    }

    Ok(tokens)
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lexer_empty_string() {
        if let Ok(tokens) = tokenize(String::from("")) {
            assert!(tokens.len() == 0);
        } else {
            assert!(false, "unexpected error");
        }
    }

    #[test]
    fn lexer_one_line() {
        if let Ok(tokens) = tokenize(String::from("(foo bar baz)")) {
            assert!(tokens.len() == 5);
            assert_eq!(tokens[0], Token::new((1, 0), TokenType::OpenParen));
            assert_eq!(tokens[1], Token::new((1, 1), TokenType::Symbol(String::from("foo"))));
            assert_eq!(tokens[2], Token::new((1, 5), TokenType::Symbol(String::from("bar"))));
            assert_eq!(tokens[3], Token::new((1, 9), TokenType::Symbol(String::from("baz"))));
            assert_eq!(tokens[4], Token::new((1, 12), TokenType::CloseParen));
        } else {
            assert!(false, "unexpected error");
        }
    }

    #[test]
    fn lexer_multi_line() {
        if let Ok(tokens) = tokenize(String::from("( foo\nbar\nbaz\n)")) {
            assert!(tokens.len() == 5);
            assert_eq!(tokens[0], Token::new((1, 0), TokenType::OpenParen));
            assert_eq!(tokens[1], Token::new((1, 2), TokenType::Symbol(String::from("foo"))));
            assert_eq!(tokens[2], Token::new((2, 0), TokenType::Symbol(String::from("bar"))));
            assert_eq!(tokens[3], Token::new((3, 0), TokenType::Symbol(String::from("baz"))));
            assert_eq!(tokens[4], Token::new((4, 0), TokenType::CloseParen));
        } else {
            assert!(false, "unexpected error");
        }
    }

    #[test]
    fn lexer_bad_whitespace() {
        if let Err(e) = tokenize(String::from("(foo\n\t(bar))")) {
            assert_eq!(e.lineno(), 2);
            assert_eq!(e.charno(), 0);
        } else {
            assert!(false, "expected ParseError for tab character");
        }
    }
}
