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

#[cfg(test)]
mod tests {
    //! Convention regression: every FIR impl in this crate treats `h` as the
    //! impulse response in natural order (`h[0]` applies to the newest
    //! sample) and reverses it once at construction. All real kernels shipped
    //! in `kernels.rs` are symmetric, so a silently-swapped convention would
    //! not surface in the "matches-naive" tests. Drive every impl with an
    //! asymmetric kernel whose output differs under correlation vs.
    //! convolution so a future regression fails loudly.

    use crate::{DynFirFilter, Filter, FirDecimator, FirFilter, NaiveFirFilter};
    use num_complex::Complex32;

    use super::dynamic_cplx::DynFirComplex;

    const ASYM: [f32; 3] = [1.0, 0.0, 0.0];
    const IN: [f32; 5] = [1.0, 2.0, 3.0, 4.0, 5.0];
    // y[n] = 1·x[n] + 0·x[n-1] + 0·x[n-2] = x[n]
    const EXPECTED: [f32; 5] = [1.0, 2.0, 3.0, 4.0, 5.0];

    fn apply<F: Filter<f32>>(mut f: F) -> Vec<f32> {
        IN.iter().map(|&x| f.filter(x)).collect()
    }

    #[test]
    fn naive_is_convolution_not_correlation() {
        let out = apply(NaiveFirFilter::<f32>::new(ASYM.to_vec()));
        assert_eq!(out, EXPECTED);
    }

    #[test]
    fn dyn_is_convolution_not_correlation() {
        let out = apply(DynFirFilter::<f32>::new(ASYM.to_vec()));
        assert_eq!(out, EXPECTED);
    }

    #[test]
    fn const_is_convolution_not_correlation() {
        let out = apply(FirFilter::<f32, 3>::new(ASYM));
        assert_eq!(out, EXPECTED);
    }

    #[test]
    fn decimator_is_convolution_not_correlation() {
        // D=1 — every input emits one output; same expected values as Filter.
        let mut d = FirDecimator::<f32, 3, 1>::new(ASYM);
        use crate::Decimator;
        let out: Vec<f32> = IN.iter().filter_map(|&x| d.decimate(x)).collect();
        assert_eq!(out, EXPECTED);
    }

    #[test]
    fn dyn_cplx_is_convolution_not_correlation() {
        // Same identity kernel, promoted to Complex32.
        let h: Vec<Complex32> = ASYM.iter().map(|&c| Complex32::new(c, 0.0)).collect();
        let mut f = DynFirComplex::new(h);
        let out: Vec<Complex32> = IN
            .iter()
            .map(|&x| f.filter(Complex32::new(x, 0.0)))
            .collect();
        for (got, want) in out.iter().zip(EXPECTED.iter()) {
            assert!((got.re - want).abs() < 1e-6 && got.im.abs() < 1e-6);
        }
    }
}
