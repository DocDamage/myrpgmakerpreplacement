//! Platform-specific modules
//!
//! This module provides platform-specific implementations for different targets.

/// WASM platform support
#[cfg(target_arch = "wasm32")]
pub mod wasm;

/// WASM platform support (re-export for non-WASM builds)
#[cfg(not(target_arch = "wasm32"))]
pub mod wasm;

pub use wasm::*;
