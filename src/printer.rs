
use types::Value;


pub fn print(value: &Value) -> String {
    format!("{}", value)
}


pub fn debug(value: &Value) -> String {
    format!("{:?}", value)
}
