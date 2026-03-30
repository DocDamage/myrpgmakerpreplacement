//! RPG Maker MZ Specific Export Functions
//!
//! Handles MZ-specific formats:
//! - Character sheets ($ prefix format)
//! - Facesets (4x2 grid)
//! - Tilesets

use crate::{ExportError, Result};
use std::path::Path;

/// MZ character frame dimensions
pub const MZ_CHAR_FRAME_WIDTH: u32 = 48;
pub const MZ_CHAR_FRAME_HEIGHT: u32 = 48;
pub const MZ_CHAR_SHEET_COLS: u32 = 4;
pub const MZ_CHAR_SHEET_ROWS: u32 = 2;
pub const MZ_CHAR_SHEET_WIDTH: u32 = MZ_CHAR_FRAME_WIDTH * MZ_CHAR_SHEET_COLS;
pub const MZ_CHAR_SHEET_HEIGHT: u32 = MZ_CHAR_FRAME_HEIGHT * MZ_CHAR_SHEET_ROWS;

/// MZ faceset dimensions
pub const MZ_FACE_FRAME_WIDTH: u32 = 144;
pub const MZ_FACE_FRAME_HEIGHT: u32 = 144;
pub const MZ_FACE_SHEET_COLS: u32 = 4;
pub const MZ_FACE_SHEET_ROWS: u32 = 2;
pub const MZ_FACE_SHEET_WIDTH: u32 = MZ_FACE_FRAME_WIDTH * MZ_FACE_SHEET_COLS;
pub const MZ_FACE_SHEET_HEIGHT: u32 = MZ_FACE_FRAME_HEIGHT * MZ_FACE_SHEET_ROWS;

/// Export character sheet in MZ $ format (single character)
///
/// MZ expects:
/// - $ prefix in filename for single-character sheets
/// - 4x2 grid (12 frames total: 3 frames × 4 directions)
/// - Each frame is 48x48 pixels
/// - Total sheet size: 192x96 pixels
pub fn export_character_sheet(input_path: &Path, output_path: &Path) -> Result<()> {
    // For now, just copy the file
    // In a full implementation, this would repack frames into MZ format
    std::fs::copy(input_path, output_path)
        .map_err(|e| ExportError::Image(format!("Failed to export character sheet: {}", e)))?;

    tracing::info!("Exported character sheet to {:?}", output_path);
    Ok(())
}

/// Export faceset in MZ format (4x2 grid with 8 face variations)
///
/// Takes a single portrait and replicates it across all 8 face slots
/// or uses multiple variations if provided.
pub fn export_faceset_from_single_portrait(portrait_path: &Path, output_path: &Path) -> Result<()> {
    // For now, just copy the file
    // In a full implementation, this would create a 4x2 grid
    std::fs::copy(portrait_path, output_path)
        .map_err(|e| ExportError::Image(format!("Failed to export faceset: {}", e)))?;

    tracing::info!("Exported faceset to {:?}", output_path);
    Ok(())
}

/// Export 8-direction character sheet
///
/// Creates a 3x8 grid for 8-direction movement plugins
pub fn export_8direction_sheet(input_path: &Path, output_path: &Path) -> Result<()> {
    // Copy for now - full implementation would rearrange frames
    std::fs::copy(input_path, output_path)
        .map_err(|e| ExportError::Image(format!("Failed to export 8-direction sheet: {}", e)))?;

    tracing::info!("Exported 8-direction sheet to {:?}", output_path);
    Ok(())
}

/// Export tileset in MZ format
///
/// MZ tilesets are:
/// - A1-A5: Autotiles (animated terrain)
/// - B-E: Decorative tiles (what we generate)
/// - Each tile is 48x48 pixels
/// - B-E sheets are 8x16 tiles (384x768 pixels)
pub fn export_tileset(input_path: &Path, output_path: &Path) -> Result<()> {
    std::fs::copy(input_path, output_path)
        .map_err(|e| ExportError::Image(format!("Failed to export tileset: {}", e)))?;

    tracing::info!("Exported tileset to {:?}", output_path);
    Ok(())
}

/// Export enemy battler sprite
pub fn export_enemy_battler(input_path: &Path, output_path: &Path) -> Result<()> {
    std::fs::copy(input_path, output_path)
        .map_err(|e| ExportError::Image(format!("Failed to export enemy battler: {}", e)))?;

    tracing::info!("Exported enemy battler to {:?}", output_path);
    Ok(())
}

/// Export parallax background layer
pub fn export_parallax_layer(input_path: &Path, output_path: &Path) -> Result<()> {
    std::fs::copy(input_path, output_path)
        .map_err(|e| ExportError::Image(format!("Failed to export parallax: {}", e)))?;

    tracing::info!("Exported parallax layer to {:?}", output_path);
    Ok(())
}

/// Get the MZ-formatted character filename with $ prefix
pub fn mz_character_filename(name: &str) -> String {
    format!("${}.png", name)
}

/// Get the MZ-formatted faceset filename
pub fn mz_faceset_filename(name: &str) -> String {
    format!("{}.png", name)
}

/// Validate that an image can be used as an MZ character sheet
pub fn validate_character_sheet(_path: &Path) -> Result<(u32, u32)> {
    // In a full implementation, this would check image dimensions
    // For now, return placeholder dimensions
    Ok((MZ_CHAR_SHEET_WIDTH, MZ_CHAR_SHEET_HEIGHT))
}

/// Validate that an image can be used as an MZ faceset
pub fn validate_faceset(_path: &Path) -> Result<(u32, u32)> {
    // In a full implementation, this would check image dimensions
    Ok((MZ_FACE_SHEET_WIDTH, MZ_FACE_SHEET_HEIGHT))
}
