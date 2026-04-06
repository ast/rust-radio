pub mod fir;
pub mod ringbuffer;
pub mod rotate;
pub mod traits;

// Trait re-exports
pub use traits::{Decimator, DelayLine, Filter};

// FIR re-exports
pub use fir::chain::{ChainDecimator, ChainableDecimator};
pub use fir::constant::FirFilter;
pub use fir::decimator::FirDecimator;
pub use fir::dot_product;
pub use fir::dynamic::DynFirFilter;
pub use fir::kernels;
pub use fir::naive::NaiveFirFilter;
