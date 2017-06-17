use memory::Allocator;
use types::Value;


pub fn print<'a, A: 'a + Allocator>(value: &Value<'a, A>) -> String {
    format!("{}", value)
}


pub fn debug<'a, A: 'a + Allocator>(value: &Value<'a, A>) -> String {
    format!("{:?}", value)
}
