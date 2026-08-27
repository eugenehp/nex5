//! Shared collections and prelude for `std` / `no_std`.

#[cfg(not(feature = "std"))]
pub use hashbrown::HashMap;
#[cfg(feature = "std")]
pub use std::collections::HashMap;

pub use alloc::string::{String, ToString};
pub use alloc::vec::Vec;

#[cfg(feature = "std")]
pub use std::{format, vec};

#[cfg(not(feature = "std"))]
pub use alloc::{format, vec};

/// Common imports for crate modules under `no_std`.
#[cfg(not(feature = "std"))]
pub mod prelude {
    pub use super::{format, vec, String, ToString, Vec};
}

/// `f64::round` for `no_std` (uses `libm`); delegates to the intrinsic in `std` builds.
#[inline]
pub fn round_f64(x: f64) -> f64 {
    #[cfg(feature = "std")]
    {
        x.round()
    }
    #[cfg(not(feature = "std"))]
    {
        libm::round(x)
    }
}
