//! Embedded Server for Asset Forge
//!
//! Serves the built Next.js app via axum.

use std::net::SocketAddr;
use std::path::PathBuf;

/// Start the embedded HTTP server
pub async fn start_server(dist_path: PathBuf, port: u16) -> crate::Result<u16> {
    #[cfg(feature = "embedded-server")]
    {
        use axum::Router;
        use tower_http::{cors::CorsLayer, services::ServeDir};

        let static_files = dist_path.join("static");
        let _server_files = dist_path.join("server");

        // Build router
        let app = Router::new()
            .fallback_service(ServeDir::new(&static_files))
            .layer(CorsLayer::permissive());

        // Bind to port
        let addr = if port == 0 {
            SocketAddr::from(([127, 0, 0, 1], 0))
        } else {
            SocketAddr::from(([127, 0, 0, 1], port))
        };

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| crate::AssetForgeError::Server(format!("Bind failed: {}", e)))?;

        let actual_port = listener
            .local_addr()
            .map_err(|e| crate::AssetForgeError::Server(format!("Get port failed: {}", e)))?
            .port();

        tracing::info!("Asset Forge server starting on port {}", actual_port);

        // Start server in background
        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .map_err(|e| tracing::error!("Server error: {}", e))
                .ok();
        });

        Ok(actual_port)
    }

    #[cfg(not(feature = "embedded-server"))]
    {
        let _ = (dist_path, port);
        Err(crate::AssetForgeError::Server(
            "embedded-server feature not enabled".to_string(),
        ))
    }
}
