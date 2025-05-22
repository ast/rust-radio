/// Delay line trait
pub trait DelayLine<T> {
    fn push(&mut self, input: T);
    fn as_slice(&self) -> &[T];
}
