//! First-run asset import flow for MCLRE.
//!
//! Handles the initial setup when `game-assets/` is absent or incomplete.
//! Prompts the user to run `assetc` or provide an LCE dump path.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Asset import status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetImportStatus {
    /// Assets are present and valid
    Complete,
    /// Assets directory exists but is incomplete
    Incomplete,
    /// Assets directory does not exist
    Missing,
    /// Assets are being imported
    Importing,
}

/// Check the status of game assets
pub fn check_asset_status(game_assets_dir: &Path) -> AssetImportStatus {
    if !game_assets_dir.exists() {
        return AssetImportStatus::Missing;
    }

    // Check for required directories/files
    let required = [
        "textures/block",
        "textures/item",
        "models/block",
        "models/item",
        "lang",
        "sounds",
        "pack.mcmeta",
    ];

    let mut all_exist = true;
    for req in &required {
        if !game_assets_dir.join(req).exists() {
            all_exist = false;
            break;
        }
    }

    if all_exist {
        AssetImportStatus::Complete
    } else {
        AssetImportStatus::Incomplete
    }
}

/// Find the assetc binary
fn find_assetc_binary() -> Option<PathBuf> {
    // First check if it's in PATH
    if let Ok(output) = Command::new("assetc").arg("--version").output() {
        if output.status.success() {
            return which::which("assetc").ok();
        }
    }

    // Check common locations
    let candidates = [
        "./target/release/assetc.exe",
        "./target/debug/assetc.exe",
        "../target/release/assetc.exe",
        "../target/debug/assetc.exe",
        "target/release/assetc",
        "target/debug/assetc",
    ];

    for candidate in &candidates {
        let path = Path::new(candidate);
        if path.exists() {
            return Some(path.to_path_buf());
        }
    }

    None
}

/// Find LCE asset source directory
fn find_lce_source() -> Option<PathBuf> {
    // Check common locations
    let candidates = [
        "reference/lce-141217/MinecraftConsoles-141217",
        "../reference/lce-141217/MinecraftConsoles-141217",
        "../../reference/lce-141217/MinecraftConsoles-141217",
    ];

    for candidate in &candidates {
        let path = Path::new(candidate);
        if path.exists() && path.join("Minecraft.Client/Common/res").exists() {
            return Some(path.to_path_buf());
        }
    }

    None
}

/// Run assetc conversion
fn run_assetc_conversion(
    assetc_bin: &Path,
    source: &Path,
    output: &Path,
    verbose: bool,
) -> Result<()> {
    println!("Running assetc conversion...");
    println!("  Source: {}", source.display());
    println!("  Output: {}", output.display());

    let mut cmd = Command::new(assetc_bin);
    cmd.arg("convert")
        .arg("--source")
        .arg(source)
        .arg("--output")
        .arg(output);

    if verbose {
        cmd.arg("--verbose");
    }

    let status = cmd.status().context("Failed to run assetc")?;

    if !status.success() {
        anyhow::bail!(
            "assetc conversion failed with exit code: {:?}",
            status.code()
        );
    }

    Ok(())
}

/// Interactive first-run import flow
pub fn run_first_import_flow(game_assets_dir: &Path, verbose: bool) -> Result<()> {
    println!("\n========================================");
    println!("MCLRE Asset Import Required");
    println!("========================================");
    println!("The game assets directory is missing or incomplete.");
    println!("MCLRE requires converted LCE assets to run.\n");

    // Try to find assetc
    let assetc_bin = match find_assetc_binary() {
        Some(path) => {
            println!("Found assetc at: {}", path.display());
            path
        }
        None => {
            println!("assetc binary not found in PATH or build directories.");
            println!("Please build assetc first: cargo build --release --bin assetc");
            return Err(anyhow::anyhow!("assetc not found"));
        }
    };

    // Try to find LCE source
    let lce_source = match find_lce_source() {
        Some(path) => {
            println!("Found LCE assets at: {}", path.display());
            path
        }
        None => {
            println!("\nLCE asset source not found automatically.");
            println!("Please provide the path to the LCE asset directory.");
            println!(
                "This should be the 'MinecraftConsoles-141217' directory from the reference source."
            );

            // In a real implementation, this would prompt the user
            // For now, we'll return an error
            return Err(anyhow::anyhow!("LCE asset source not found"));
        }
    };

    // Run conversion
    println!("\nStarting asset conversion (this may take a few minutes)...");
    run_assetc_conversion(&assetc_bin, &lce_source, game_assets_dir, verbose)?;

    // Verify result
    let status = check_asset_status(game_assets_dir);
    if status == AssetImportStatus::Complete {
        println!("\n✅ Asset import complete!");
        println!("Assets are ready at: {}", game_assets_dir.display());
        Ok(())
    } else {
        anyhow::bail!("Asset conversion completed but verification failed");
    }
}

/// Non-interactive asset import (for CI/automated setups)
pub fn run_auto_import(
    game_assets_dir: &Path,
    lce_source: Option<PathBuf>,
    verbose: bool,
) -> Result<()> {
    // Check if already complete
    if check_asset_status(game_assets_dir) == AssetImportStatus::Complete {
        return Ok(());
    }

    let assetc_bin = find_assetc_binary()
        .context("assetc binary not found. Build with: cargo build --release --bin assetc")?;

    let lce_source = lce_source.or_else(find_lce_source)
        .context("LCE asset source not found. Provide with --lce-source or ensure reference/lce-141217 exists")?;

    println!(
        "Auto-importing assets from {} to {}",
        lce_source.display(),
        game_assets_dir.display()
    );
    run_assetc_conversion(&assetc_bin, &lce_source, game_assets_dir, verbose)?;

    // Verify
    if check_asset_status(game_assets_dir) != AssetImportStatus::Complete {
        anyhow::bail!("Asset conversion completed but verification failed");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_asset_status_missing() {
        let temp_dir = tempdir().unwrap();
        let missing_dir = temp_dir.path().join("nonexistent");
        assert_eq!(check_asset_status(&missing_dir), AssetImportStatus::Missing);
    }

    #[test]
    fn test_asset_status_incomplete() {
        let temp_dir = tempdir().unwrap();
        // Create dir but not required files
        fs::create_dir_all(temp_dir.path().join("textures/block")).unwrap();
        assert_eq!(
            check_asset_status(temp_dir.path()),
            AssetImportStatus::Incomplete
        );
    }

    #[test]
    fn test_asset_status_complete() {
        let temp_dir = tempdir().unwrap();
        // Create all required dirs/files
        let required = [
            "textures/block",
            "textures/item",
            "models/block",
            "models/item",
            "lang",
            "sounds",
        ];
        for req in &required {
            fs::create_dir_all(temp_dir.path().join(req)).unwrap();
        }
        fs::write(temp_dir.path().join("pack.mcmeta"), "{}").unwrap();
        assert_eq!(
            check_asset_status(temp_dir.path()),
            AssetImportStatus::Complete
        );
    }
}
