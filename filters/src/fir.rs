pub mod chain;
pub mod constant;
pub mod decimator;
pub mod dynamic;
pub mod dynamic_cplx;
pub mod kernels;
pub mod naive;

/// Dot product using a single accumulator — simple reference implementation.
/// Kept for testing; the serial dependency chain limits throughput.
#[inline]
pub fn dot_product_naive<T>(h: &[f32], z: &[T]) -> T
where
    T: Copy + Default + std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T>,
{
    h.iter()
        .zip(z.iter())
        .fold(T::default(), |acc, (&coeff, &sample)| acc + sample * coeff)
}

/// Dot product with 4 independent accumulators and no bounds checks.
/// Breaks the serial dependency chain so the CPU can pipeline multiply-adds,
/// and eliminates per-element bounds checks in the inner loop.
#[inline]
pub fn dot_product<T>(h: &[f32], z: &[T]) -> T
where
    T: Copy + Default + std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T>,
{
    let len = h.len().min(z.len());

    let mut acc0 = T::default();
    let mut acc1 = T::default();
    let mut acc2 = T::default();
    let mut acc3 = T::default();

    let hp = h.as_ptr();
    let zp = z.as_ptr();

    let chunks = len / 4;
    for i in 0..chunks {
        let base = i * 4;
        unsafe {
            acc0 = acc0 + *zp.add(base) * *hp.add(base);
            acc1 = acc1 + *zp.add(base + 1) * *hp.add(base + 1);
            acc2 = acc2 + *zp.add(base + 2) * *hp.add(base + 2);
            acc3 = acc3 + *zp.add(base + 3) * *hp.add(base + 3);
        }
    }

    let mut acc = (acc0 + acc1) + (acc2 + acc3);
    for i in (chunks * 4)..len {
        unsafe {
            acc = acc + *zp.add(i) * *hp.add(i);
        }
    }
    acc
}
