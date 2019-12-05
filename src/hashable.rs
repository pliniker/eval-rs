// TODO
trait Hashable {
    fn hash<'guard>(&self, _guard: &'guard dyn MutatorScope) -> u64;
}
