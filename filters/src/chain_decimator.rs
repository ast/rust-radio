use crate::decimator::Decimator;

/// A way to chain decimators together
pub struct ChainDecimator<D1, D2, T> {
    d1: D1,
    d2: D2,
    _marker: std::marker::PhantomData<T>,
}

impl<D1, D2, T> ChainDecimator<D1, D2, T> {
    pub fn new(d1: D1, d2: D2) -> Self {
        Self {
            d1,
            d2,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<D1, D2, T> Decimator<T> for ChainDecimator<D1, D2, T>
where
    D1: Decimator<T>,
    D2: Decimator<T>,
{
    fn decimate(&mut self, x: T) -> Option<T> {
        self.d1.decimate(x).and_then(|y| self.d2.decimate(y))
    }
}

pub trait ChainableDecimator<T>: Decimator<T> + Sized {
    fn chain_with<D2>(self, other: D2) -> ChainDecimator<Self, D2, T>
    where
        D2: Decimator<T>,
    {
        ChainDecimator::new(self, other)
    }
}

impl<T, D> ChainableDecimator<T> for D where D: Decimator<T> {}
