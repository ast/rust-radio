/// Simple filter trait
pub trait Filter<T> {
    fn filter(&mut self, input: T) -> T;
}

/// Decimator trait
pub trait Decimator<T> {
    fn decimate(&mut self, input: T) -> Option<T>;
}

/// Delay line trait
pub trait DelayLine<T> {
    fn push(&mut self, input: T);
    fn as_slice(&self) -> &[T];
}
