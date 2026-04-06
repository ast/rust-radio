pub mod chain;
pub mod constant;
pub mod decimator;
pub mod dynamic;
pub mod kernels;
pub mod naive;

/// Dot product of FIR coefficients and delay line samples.
#[inline]
pub fn dot_product<T>(h: &[f32], z: &[T]) -> T
where
    T: Copy + Default + std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T>,
{
    h.iter()
        .zip(z.iter())
        .fold(T::default(), |acc, (&coeff, &sample)| acc + sample * coeff)
}
