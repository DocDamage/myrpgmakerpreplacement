//! WASM export for web deployment

use std::path::{Path, PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};

/// WASM export configuration
#[derive(Debug, Clone)]
pub struct WasmExportConfig {
    pub output_dir: PathBuf,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub enable_audio: bool,
    pub enable_storage: bool, // LocalStorage for saves
    pub preload_assets: bool,
    pub compression: CompressionLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionLevel {
    None,
    Low,    // Fast build, larger output
    Medium, // Balance
    High,   // Slow build, smallest output
}

impl Default for WasmExportConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("./wasm-export"),
            canvas_width: 1280,
            canvas_height: 720,
            enable_audio: true,
            enable_storage: true,
            preload_assets: true,
            compression: CompressionLevel::Medium,
        }
    }
}

/// Exports the game to WASM format
pub struct WasmExporter {
    config: WasmExportConfig,
}

impl WasmExporter {
    pub fn new(config: WasmExportConfig) -> Self {
        Self { config }
    }
    
    /// Export the game
    pub fn export(&self, project_path: &Path) -> Result<WasmExportResult, WasmExportError> {
        // Create output directory
        fs::create_dir_all(&self.config.output_dir)?;
        
        // Copy and optimize assets
        let asset_manifest = self.process_assets(project_path)?;
        
        // Generate HTML wrapper
        self.generate_html(&asset_manifest)?;
        
        // Generate JavaScript glue
        self.generate_js_glue(&asset_manifest)?;
        
        // Generate service worker for offline play
        self.generate_service_worker()?;
        
        // Generate manifest for PWA
        self.generate_pwa_manifest()?;
        
        Ok(WasmExportResult {
            output_dir: self.config.output_dir.clone(),
            total_size: self.calculate_total_size()?,
            files_created: self.list_output_files()?,
        })
    }
    
    /// Process and optimize assets for web
    fn process_assets(&self, project_path: &Path) -> Result<AssetManifest, WasmExportError> {
        let assets_dir = self.config.output_dir.join("assets");
        fs::create_dir_all(&assets_dir)?;
        
        let mut manifest = AssetManifest::default();
        
        // Copy SQLite database
        let db_path = project_path.join("world.db");
        if db_path.exists() {
            let db_output = assets_dir.join("world.db");
            fs::copy(&db_path, &db_output)?;
            manifest.database_size = fs::metadata(&db_output)?.len();
        }
        
        // Process images
        let img_dir = project_path.join("assets").join("images");
        if img_dir.exists() {
            for entry in fs::read_dir(&img_dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if ext_str == "png" || ext_str == "jpg" || ext_str == "jpeg" {
                        // Convert to WebP for smaller size
                        let output_name = format!("{}.webp", path.file_stem().unwrap().to_string_lossy());
                        let output_path = assets_dir.join(&output_name);
                        
                        self.convert_to_webp(&path, &output_path)?;
                        
                        manifest.images.push(AssetInfo {
                            name: output_name,
                            size: fs::metadata(&output_path)?.len(),
                            original_size: fs::metadata(&path)?.len(),
                        });
                    }
                }
            }
        }
        
        // Process audio
        let audio_dir = project_path.join("assets").join("audio");
        if audio_dir.exists() {
            for entry in fs::read_dir(&audio_dir)? {
                let entry = entry?;
                let path = entry.path();
                
                // Convert to Ogg Vorbis for web
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if ext_str == "wav" || ext_str == "mp3" {
                        let output_name = format!("{}.ogg", path.file_stem().unwrap().to_string_lossy());
                        let _output_path = assets_dir.join(&output_name);
                        
                        // For now, record the planned conversion
                        // Actual conversion would require additional dependencies
                        manifest.audio.push(AssetInfo {
                            name: output_name,
                            size: 0, // Will be updated after conversion
                            original_size: fs::metadata(&path)?.len(),
                        });
                    }
                }
            }
        }
        
        // Write manifest
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        fs::write(assets_dir.join("manifest.json"), manifest_json)?;
        
        Ok(manifest)
    }
    
    /// Generate HTML wrapper
    fn generate_html(&self, _manifest: &AssetManifest) -> Result<(), WasmExportError> {
        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>DDE Game</title>
    <link rel="manifest" href="manifest.json">
    <style>
        body {{
            margin: 0;
            padding: 0;
            background: #1a1a2e;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            font-family: system-ui, -apple-system, sans-serif;
        }}
        #game-container {{
            position: relative;
            box-shadow: 0 0 50px rgba(0,0,0,0.5);
        }}
        canvas {{
            display: block;
            image-rendering: pixelated;
            image-rendering: crisp-edges;
        }}
        #loading {{
            position: absolute;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            color: white;
            text-align: center;
        }}
        #loading-bar {{
            width: 200px;
            height: 4px;
            background: rgba(255,255,255,0.2);
            border-radius: 2px;
            margin-top: 10px;
            overflow: hidden;
        }}
        #loading-progress {{
            width: 0%;
            height: 100%;
            background: #4a9eff;
            transition: width 0.3s;
        }}
    </style>
</head>
<body>
    <div id="game-container">
        <canvas id="game-canvas" width="{width}" height="{height}"></canvas>
        <div id="loading">
            <div>Loading...</div>
            <div id="loading-bar">
                <div id="loading-progress"></div>
            </div>
        </div>
    </div>
    <script type="module" src="game.js"></script>
</body>
</html>"#,
            width = self.config.canvas_width,
            height = self.config.canvas_height
        );
        
        fs::write(self.config.output_dir.join("index.html"), html)?;
        Ok(())
    }
    
    /// Generate JavaScript glue code
    fn generate_js_glue(&self, manifest: &AssetManifest) -> Result<(), WasmExportError> {
        let js = format!(
            r#"// DDE WASM Runtime
import init, {{ run_game }} from './dde_engine.js';

const canvas = document.getElementById('game-canvas');
const loadingProgress = document.getElementById('loading-progress');
const loadingDiv = document.getElementById('loading');

// Asset preloading
const assets = {assets_json};
let loadedAssets = 0;
const totalAssets = assets.images.length + assets.audio.length + 1; // +1 for DB

function updateProgress() {{
    loadedAssets++;
    const percent = (loadedAssets / totalAssets) * 100;
    loadingProgress.style.width = percent + '%';
    
    if (loadedAssets >= totalAssets) {{
        loadingDiv.style.display = 'none';
    }}
}}

// Preload images
const imageCache = {{}};
assets.images.forEach(img => {{
    const image = new Image();
    image.src = 'assets/' + img.name;
    image.onload = updateProgress;
    imageCache[img.name] = image;
}});

// Initialize WASM
async function initGame() {{
    // Fetch database
    const dbResponse = await fetch('assets/world.db');
    const dbBuffer = await dbResponse.arrayBuffer();
    updateProgress();
    
    // Initialize WASM module
    const wasm = await init('./dde_engine_bg.wasm');
    
    // Start game
    run_game(canvas, dbBuffer, {{
        width: {width},
        height: {height},
        enableAudio: {audio},
        enableStorage: {storage},
    }});
}}

initGame().catch(console.error);

// Service Worker registration for offline play
if ('serviceWorker' in navigator) {{
    navigator.serviceWorker.register('sw.js');
}}
"#,
            assets_json = serde_json::to_string(manifest)?,
            width = self.config.canvas_width,
            height = self.config.canvas_height,
            audio = self.config.enable_audio,
            storage = self.config.enable_storage
        );
        
        fs::write(self.config.output_dir.join("game.js"), js)?;
        Ok(())
    }
    
    /// Generate service worker for offline play
    fn generate_service_worker(&self) -> Result<(), WasmExportError> {
        let sw = r#"// Service Worker for offline play
const CACHE_NAME = 'dde-game-v1';
const urlsToCache = [
    './',
    './index.html',
    './game.js',
    './dde_engine.js',
    './dde_engine_bg.wasm',
    './assets/manifest.json'
];

self.addEventListener('install', event => {
    event.waitUntil(
        caches.open(CACHE_NAME)
            .then(cache => cache.addAll(urlsToCache))
    );
});

self.addEventListener('fetch', event => {
    event.respondWith(
        caches.match(event.request)
            .then(response => {
                if (response) {
                    return response;
                }
                return fetch(event.request);
            })
    );
});
"#;
        
        fs::write(self.config.output_dir.join("sw.js"), sw)?;
        Ok(())
    }
    
    /// Generate PWA manifest
    fn generate_pwa_manifest(&self) -> Result<(), WasmExportError> {
        let manifest = serde_json::json!({
            "name": "DDE Game",
            "short_name": "DDE",
            "start_url": ".",
            "display": "standalone",
            "background_color": "#1a1a2e",
            "theme_color": "#4a9eff",
            "icons": [
                {
                    "src": "icon-192.png",
                    "sizes": "192x192",
                    "type": "image/png"
                },
                {
                    "src": "icon-512.png",
                    "sizes": "512x512",
                    "type": "image/png"
                }
            ]
        });
        
        fs::write(
            self.config.output_dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest)?
        )?;
        Ok(())
    }
    
    /// Convert image to WebP
    fn convert_to_webp(&self, input: &Path, output: &Path) -> Result<(), WasmExportError> {
        // In production, use image crate with webp feature
        // For now, copy as-is to avoid image processing dependencies
        fs::copy(input, output)?;
        Ok(())
    }
    
    fn calculate_total_size(&self) -> Result<u64, WasmExportError> {
        let mut total = 0u64;
        for entry in walkdir::WalkDir::new(&self.config.output_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total += entry.metadata()?.len();
            }
        }
        Ok(total)
    }
    
    fn list_output_files(&self) -> Result<Vec<PathBuf>, WasmExportError> {
        let mut files = Vec::new();
        for entry in walkdir::WalkDir::new(&self.config.output_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                files.push(entry.path().to_path_buf());
            }
        }
        Ok(files)
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct AssetManifest {
    pub database_size: u64,
    pub images: Vec<AssetInfo>,
    pub audio: Vec<AssetInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct AssetInfo {
    pub name: String,
    pub size: u64,
    pub original_size: u64,
}

pub struct WasmExportResult {
    pub output_dir: PathBuf,
    pub total_size: u64,
    pub files_created: Vec<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum WasmExportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("WalkDir error: {0}")]
    WalkDir(#[from] walkdir::Error),
}
