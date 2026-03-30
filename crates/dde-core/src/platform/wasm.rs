//! WASM platform support

/// WASM platform initialization and utilities
/// 
/// This module provides WASM-specific functionality for running the engine
/// in web browsers via WebAssembly.

#[cfg(target_arch = "wasm32")]
use web_sys::{window, HtmlCanvasElement, WebGl2RenderingContext};

/// Initialize WASM runtime with WebGL2 context
#[cfg(target_arch = "wasm32")]
pub fn init_wasm(canvas_id: &str) -> Result<WebGl2RenderingContext, WasmError> {
    let window = window().ok_or(WasmError::NoWindow)?;
    let document = window.document().ok_or(WasmError::NoDocument)?;
    
    let canvas: HtmlCanvasElement = document
        .get_element_by_id(canvas_id)
        .ok_or(WasmError::CanvasNotFound)?
        .dyn_into()
        .map_err(|_| WasmError::CanvasNotFound)?;
    
    let context: WebGl2RenderingContext = canvas
        .get_context("webgl2")
        .map_err(|_| WasmError::WebGlNotSupported)?
        .ok_or(WasmError::WebGlNotSupported)?
        .dyn_into()
        .map_err(|_| WasmError::WebGlNotSupported)?;
    
    Ok(context)
}

/// Stub for non-WASM targets
#[cfg(not(target_arch = "wasm32"))]
pub fn init_wasm(_canvas_id: &str) -> Result<(), WasmError> {
    Err(WasmError::NotWasmTarget)
}

/// WASM-specific errors
#[derive(Debug, thiserror::Error)]
pub enum WasmError {
    #[error("No window available")]
    NoWindow,
    #[error("No document available")]
    NoDocument,
    #[error("Canvas not found")]
    CanvasNotFound,
    #[error("WebGL2 not supported")]
    WebGlNotSupported,
    #[error("Not running on WASM target")]
    NotWasmTarget,
}

/// Check if running in WASM environment
pub const IS_WASM: bool = cfg!(target_arch = "wasm32");

/// Platform-specific sleep implementation
#[cfg(target_arch = "wasm32")]
pub fn sleep_ms(ms: u32) {
    // In WASM, use setTimeout via web-sys
    // For now, this is a no-op placeholder
    let _ = ms;
}

/// Platform-specific sleep implementation for native targets
#[cfg(not(target_arch = "wasm32"))]
pub fn sleep_ms(ms: u32) {
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
}
