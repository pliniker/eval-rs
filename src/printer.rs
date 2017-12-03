use memory::Heap;
use types::Value;


pub fn print<'a, A: 'a + Heap>(value: &Value<'a, A>) -> String {
    format!("{}", value)
}


pub fn debug<'a, A: 'a + Heap>(value: &Value<'a, A>) -> String {
    format!("{:?}", value)
}
