
pub type SourcePos = (u32, u32);


#[derive(Debug)]
pub struct ParseError {
    pos: SourcePos,
    reason: String,
}


impl ParseError {
    pub fn new(pos: SourcePos, reason: String) -> ParseError {
        ParseError {
            pos: pos,
            reason: reason,
        }
    }

    pub fn lineno(&self) -> u32 {
        self.pos.0
    }

    pub fn charno(&self) -> u32 {
        self.pos.1
    }

    pub fn message(&self) -> &str {
        self.reason.as_str()
    }
}
