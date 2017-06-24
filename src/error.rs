
pub type SourcePos = (u32, u32);


#[derive(Debug)]
pub struct ParseError {
    pos: Option<SourcePos>,
    reason: String,
}


impl ParseError {
    pub fn error(reason: String) -> ParseError {
        ParseError {
            pos: None,
            reason: reason
        }
    }

    pub fn with_pos(pos: SourcePos, reason: String) -> ParseError {
        ParseError {
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
