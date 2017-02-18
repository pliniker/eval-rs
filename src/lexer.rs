
use error::{ParseError, SourcePos};


// key characters
const OPEN_BRACKET: char = '(';
const CLOSE_BRACKET: char = ')';
const SPACE: char = ' ';
const TAB: char = '\t';


#[derive(Debug, PartialEq)]
pub enum TokenType {
    OpenBracket,
    CloseBracket,
    Symbol(String),
}


#[derive(Debug, PartialEq)]
pub struct Token {
    pos: SourcePos,
    token: TokenType,
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

    pub fn source_pos(&self) -> SourcePos {
        self.pos
    }
}


// tokenize a String
pub fn tokenize(input: String) -> Result<Vec<Token>, ParseError> {

    use self::TokenType::*;

    // start line numbering at 1, the first character of each line being number 0
    let mut lineno = 1;

    // characters that terminate a symbol
    let terminating = [OPEN_BRACKET, CLOSE_BRACKET, SPACE, TAB];
    let is_terminating = |c: char| terminating.iter().any(|t| c == *t);

    // return value
    let mut tokens = Vec::new();

    for line in input.lines() {

        // start line character numbering at 0
        let mut charno = 0;
        let mut chars = line.chars();
        let mut current = chars.next();

        loop {
            match current {
                Some(TAB) =>
                    return Err(ParseError::new(
                        (lineno, charno),
                        String::from("tabs are not valid whitespace"))),

                Some(SPACE) => current = chars.next(),

                Some(OPEN_BRACKET) => {
                    tokens.push(Token::new((lineno, charno), OpenBracket));
                    current = chars.next();
                }

                Some(CLOSE_BRACKET) => {
                    tokens.push(Token::new((lineno, charno), CloseBracket));
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

        lineno += 1;
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
            assert_eq!(tokens[0], Token::new((1, 0), TokenType::OpenBracket));
            assert_eq!(tokens[1], Token::new((1, 1), TokenType::Symbol(String::from("foo"))));
            assert_eq!(tokens[2], Token::new((1, 5), TokenType::Symbol(String::from("bar"))));
            assert_eq!(tokens[3], Token::new((1, 9), TokenType::Symbol(String::from("baz"))));
            assert_eq!(tokens[4], Token::new((1, 12), TokenType::CloseBracket));
        } else {
            assert!(false, "unexpected error");
        }
    }

    #[test]
    fn lexer_multi_line() {
        if let Ok(tokens) = tokenize(String::from("( foo\nbar\nbaz\n)")) {
            assert!(tokens.len() == 5);
            assert_eq!(tokens[0], Token::new((1, 0), TokenType::OpenBracket));
            assert_eq!(tokens[1], Token::new((1, 2), TokenType::Symbol(String::from("foo"))));
            assert_eq!(tokens[2], Token::new((2, 0), TokenType::Symbol(String::from("bar"))));
            assert_eq!(tokens[3], Token::new((3, 0), TokenType::Symbol(String::from("baz"))));
            assert_eq!(tokens[4], Token::new((4, 0), TokenType::CloseBracket));
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
