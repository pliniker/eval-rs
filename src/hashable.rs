// TODO

use crate::safeptr::MutatorScope;

pub type HashValue = u64;

trait Hashable {
    fn hash<'guard>(&self, _guard: &'guard dyn MutatorScope) -> HashValue;
}
