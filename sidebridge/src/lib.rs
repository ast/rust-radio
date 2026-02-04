// Copyright SM6WJM 2026

pub mod drivers;
pub mod traits;

pub mod civ_packet;

// Re-export core types for convenience
pub use crate::traits::{RadioMode, RadioState, RadioTrait, SpectrumFrame};

// Prelude for quick imports
pub mod prelude {
    pub use crate::traits::RadioTrait as _;
}
