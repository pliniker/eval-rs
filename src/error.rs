
use std::error::Error;
use std::fmt;


pub type SourcePos = (u32, u32);


#[derive(Debug)]
pub struct ParseEvalError {
    pos: Option<SourcePos>,
    reason: String,
}


impl ParseEvalError {
    pub fn error(reason: String) -> ParseEvalError {
        ParseEvalError {
            pos: None,
            reason: reason
        }
    }

    pub fn with_pos(pos: SourcePos, reason: String) -> ParseEvalError {
        ParseEvalError {
            pos: Some(pos),
            reason: reason,
        }
    }

    pub fn error_pos(&self) -> Option<SourcePos> {
        self.pos
    }

    /// Given the relevant source code string, show the error in context
    pub fn print_with_source(&self, source: &str) {
        if let Some((lineno, charno)) = self.pos {
            let mut iter = source.lines().enumerate();

            while let Some((count, line)) = iter.next() {
                // count starts at 0, line numbers start at 1
                if count + 1 == lineno as usize {
                    println!("error: {}", self.reason);
                    println!("{:5}|{}", lineno, line);
                    println!("{:5}|{:width$}^", " ", " ", width = charno as usize);
                    println!("{:5}|", " ");
                    return;
                }
            }
        } else {
            println!("error: {}", self.reason);
        }
    }
}


impl fmt::Display for ParseEvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.reason)
    }
}


impl Error for ParseEvalError {
    fn description(&self) -> &str {
        &self.reason
    }
}


pub fn err(reason: &str) -> ParseEvalError {
    ParseEvalError::error(String::from(reason))
}


pub fn err_wpos(pos: SourcePos, reason: &str) -> ParseEvalError {
    ParseEvalError::with_pos(pos, String::from(reason))
}
