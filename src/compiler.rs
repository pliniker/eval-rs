use crate::error::{ErrorKind, RuntimeError};
use crate::safeptr::ScopedPtr;

fn compile<'guard>(code: ScopedPtr<'guard>) -> Result<ScopedPtr<'guard>, RuntimeError> {
    Err(RuntimeError::new(ErrorKind::EvalError(String::from(
        "unimplemented",
    ))))
}
