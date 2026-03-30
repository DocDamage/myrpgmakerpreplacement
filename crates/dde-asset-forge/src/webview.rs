//! Webview Integration
//!
//! Opens the Asset Forge in a wry webview window.

/// Open a webview window for the Asset Forge
pub async fn open_webview(url: &str, _enable_devtools: bool) -> crate::Result<()> {
    #[cfg(feature = "webview")]
    {
        // Note: Full wry integration requires a window handle from the main app
        // This is a simplified version - in practice, you'd integrate with winit

        tracing::info!("Opening webview to {}", url);

        // For now, we open in the system browser as a fallback
        // Full implementation would use wry::WebViewBuilder

        open::that(url).map_err(|e| {
            crate::AssetForgeError::Webview(format!("Failed to open browser: {}", e))
        })?;

        Ok(())
    }

    #[cfg(not(feature = "webview"))]
    {
        let _ = url;
        let _ = _enable_devtools;
        tracing::warn!("Webview not enabled, opening browser instead");

        open::that(url).map_err(|e| {
            crate::AssetForgeError::Webview(format!("Failed to open browser: {}", e))
        })?;

        Ok(())
    }
}

/// Create a wry webview (requires window handle)
#[cfg(feature = "webview")]
pub fn create_webview(window: &winit::window::Window, url: &str) -> crate::Result<wry::WebView> {
    use wry::WebViewBuilder;

    let builder = WebViewBuilder::new().with_url(url).with_ipc_handler(|msg| {
        tracing::debug!("IPC message: {:?}", msg);
        // Handle IPC messages
    });

    // Build webview
    let webview = builder
        .build(window)
        .map_err(|e| crate::AssetForgeError::Webview(format!("Build failed: {}", e)))?;

    Ok(webview)
}
