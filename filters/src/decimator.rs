/// Decimator trait
pub trait Decimator<T> {
    fn decimate(&mut self, input: T) -> Option<T>;
}
