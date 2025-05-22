/// Simple filter trait
pub trait Filter<T> {
    fn filter(&mut self, input: T) -> T;
}
