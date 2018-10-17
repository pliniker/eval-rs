
use std::error::Error;
use std::fmt;


/// Source code position
#[derive(Debug)]
struct SourcePos {
    line: u32,
    column: u32
}


#[derive(Debug)]
enum ErrorKind {
    ParseError(String),
    EvalError(String),
    OutOfMemory,
    BadAllocation,
}


/// An Eval-rs runtime error type
#[derive(Debug)]
pub struct RuntimeError {
    kind: ErrorKind,
    pos: Option<SourcePos>,
}


impl RuntimeError {
    pub fn new(kind: ErrorKind) -> RuntimeError {
        RuntimeError {
            kind: kind,
            pos: None,
        }
    }

    pub fn with_pos(kind: ErrorKind, pos: SourcePos) -> RuntimeError {
        RuntimeError {
            kind: kind,
            pos: Some(pos),
        }
    }

    pub fn error_pos(&self) -> Option<SourcePos> {
        self.pos
    }

    /// Given the relevant source code string, show the error in context
    pub fn print_with_source(&self, source: &str) {
        if let Some(pos) = self.pos {
            let mut iter = source.lines().enumerate();

            while let Some((count, line)) = iter.next() {
                // count starts at 0, line numbers start at 1
                if count + 1 == pos.line as usize {
                    println!("error: {}", self);
                    println!("{:5}|{}", pos.line, line);
                    println!("{:5}|{:width$}^", " ", " ", width = pos.char as usize);
                    println!("{:5}|", " ");
                    return;
                }
            }
        } else {
            println!("error: {}", self);
        }
    }
}


impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ErrorKind::ParseError(reason) => write!(f, "Parse error: {}", reason),
            ErrorKind::EvalError(reason) => write!(f, "Evaluation error: {}", reason),
            ErrorKind::OutOfMemory => write!(f, "Out of memory!"),
            ErrorKind::BadAllocation => write!(f, "An invalid memory size allocation was requested!"),
        }
    }
}


impl Error for RuntimeError {
    fn description(&self) -> &str {
        &self.reason
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}


/// Convenience function for building a `ParseEvalError`
pub fn err_parser(reason: &str) -> RuntimeError {
    RuntimeError::with_reason(ErrorKind::ParseError(String::from(reason)))
}


/// Convenience function for building a `ParseEvalError` including a source position
pub fn err_parser_wpos(pos: SourcePos, reason: &str) -> RuntimeError {
    RuntimeError::with_pos(ErrorKind::ParseError(String::from(reason)), pos)
}
