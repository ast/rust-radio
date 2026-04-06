//pub mod bufferline;
pub mod chain_decimator;
pub mod decimator;
pub mod delay_line;
pub mod fast_fir;
pub mod filter;
pub mod fir;
pub mod kernels;
pub mod naive_fir;
pub mod ringbuffer;
pub mod rotate;

pub mod fir_decimator;

pub use chain_decimator::ChainDecimator;
pub use chain_decimator::ChainableDecimator;

pub use decimator::Decimator;
pub use delay_line::DelayLine;
pub use filter::Filter;
pub use fir::dot_product;
pub use fir::FirFilter3;
pub use fir_decimator::FirDecimator;

pub use fast_fir::FirFilter4;
pub use naive_fir::FirFilter;
