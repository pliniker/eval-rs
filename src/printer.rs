
use lexer::{Token, TokenType};


pub fn print(tokens: &Vec<Token>) {
    use self::TokenType::*;

    let mut last_was_symbol = false;
    let mut last_was_close = false;

    for token in tokens {
        match token.token_type() {
            &OpenParen => {
                if last_was_symbol {
                    print!(" (");
                } else {
                    print!("(");
                }

                last_was_symbol = false;
                last_was_close = false;
            },

            &CloseParen => {
                print!(")");
                last_was_symbol = false;
                last_was_close = true;
            },

            &Symbol(ref sym) => {
                if last_was_symbol || last_was_close {
                    print!(" {}", sym);
                } else {
                    print!("{}", sym);
                }

                last_was_symbol = true;
                last_was_close = false;
            },

            &Dot => {
                print!(" . ");
                last_was_symbol = false;
                last_was_close = false;
            }
        }
    }

    println!("");
}
