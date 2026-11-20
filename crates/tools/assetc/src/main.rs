//! assetc: offline importer/converter for LCE assets (archives, textures, audio, languages).
//!
//! This tool converts Legacy Console Edition assets to modern Java Edition-compatible formats:
//! - Textures: DDS/PNG â†’ PNG/KTX2 (with mipmaps)
//! - Language files: LCE format â†’ Java JSON (`namespace:key`)
//! - Models: LCE format â†’ Java JSON block/item models
//! - Audio: XWB/XSB â†’ Ogg Vorbis (via vgmstream)
//! - Pack metadata: LCE â†’ Java `pack.mcmeta` + `pack.toml`
//!
//! Usage:
//!   assetc convert --source <LCE_ASSETS_DIR> --output <GAME_ASSETS_DIR> [--texture-format png|ktx2]
//!   assetc pack --input <GAME_ASSETS_DIR> --name "MCLRE Vanilla" --description "Converted LCE assets"
//!   assetc extract-arc --archive <ARC_FILE> --output <OUTPUT_DIR>

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use image::{DynamicImage, ImageBuffer, Rgba};
use mclre_assets::{AssetManager, AssetVfs, Texture, TextureAtlas};
use mclre_core::Identifier;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "assetc", version, about = "LCE Asset Converter for MCLRE")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Convert LCE assets to modern formats
    Convert {
        /// Source directory containing LCE assets (reference/lce-141217/)
        #[arg(short, long)]
        source: PathBuf,

        /// Output directory for converted assets (game-assets/)
        #[arg(short, long)]
        output: PathBuf,

        /// Texture output format
        #[arg(long, default_value = "png")]
        texture_format: TextureFormat,

        /// Generate mipmaps for textures
        #[arg(long, default_value = "true")]
        mipmaps: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Create a resource pack from converted assets
    Pack {
        /// Input directory (game-assets/)
        #[arg(short, long)]
        input: PathBuf,

        /// Output pack file (ZIP) or directory
        #[arg(short, long)]
        output: PathBuf,

        /// Pack name
        #[arg(long, default_value = "MCLRE Vanilla")]
        name: String,

        /// Pack description
        #[arg(long, default_value = "Converted LCE assets for MCLRE")]
        description: String,

        /// Pack format version (matches Java Edition)
        #[arg(long, default_value = "15")]
        pack_format: i32,
    },

    /// Build texture atlas from individual textures
    Atlas {
        /// Input directory with textures
        #[arg(short, long)]
        input: PathBuf,

        /// Output atlas texture
        #[arg(short, long)]
        output: PathBuf,

        /// Atlas size (width/height)
        #[arg(long, default_value = "2048")]
        size: u32,

        /// Padding between textures
        #[arg(long, default_value = "1")]
        padding: u32,
    },

    /// Validate asset directory structure
    Validate {
        /// Directory to validate
        #[arg(short, long)]
        path: PathBuf,
    },

    /// Extract .arc archive
    ExtractArc {
        /// Archive file to extract
        #[arg(short, long)]
        archive: PathBuf,

        /// Output directory
        #[arg(short, long)]
        output: PathBuf,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Extract .pck (texture pack) file
    ExtractPck {
        /// PCK file to extract
        #[arg(short, long)]
        pck: PathBuf,

        /// Output directory
        #[arg(short, long)]
        output: PathBuf,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Convert audio from XWB/XSB to Ogg Vorbis
    ConvertAudio {
        /// Source audio directory
        #[arg(short, long)]
        source: PathBuf,

        /// Output directory
        #[arg(short, long)]
        output: PathBuf,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum TextureFormat {
    Png,
    Ktx2,
}

impl std::str::FromStr for TextureFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "png" => Ok(TextureFormat::Png),
            "ktx2" => Ok(TextureFormat::Ktx2),
            _ => Err(format!("Unknown texture format: {}", s)),
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Convert {
            source,
            output,
            texture_format,
            mipmaps,
            verbose,
        } => convert_command(source, output, texture_format, mipmaps, verbose),
        Command::Pack {
            input,
            output,
            name,
            description,
            pack_format,
        } => pack_command(input, output, name, description, pack_format),
        Command::Atlas {
            input,
            output,
            size,
            padding,
        } => atlas_command(input, output, size, padding),
        Command::Validate { path } => validate_command(path),
        Command::ExtractArc {
            archive,
            output,
            verbose,
        } => extract_arc_command(archive, output, verbose),
        Command::ExtractPck {
            pck,
            output,
            verbose,
        } => extract_pck_command(pck, output, verbose),
        Command::ConvertAudio {
            source,
            output,
            verbose,
        } => convert_audio_command(source, output, verbose),
    }
}

// ========================================================================
// ARC Archive Extraction
// ========================================================================

fn extract_arc_command(archive: PathBuf, output: PathBuf, verbose: bool) -> Result<()> {
    println!("Extracting ARC archive: {:?} -> {:?}", archive, output);
    std::fs::create_dir_all(&output)?;

    // Read the archive file
    let data = std::fs::read(&archive)
        .with_context(|| format!("Failed to read archive: {:?}", archive))?;

    // Parse ARC header
    // LCE ARC format: magic (4 bytes) + file count (4 bytes) + file entries
    if data.len() < 8 {
        return Err(anyhow::anyhow!("Archive too small to be valid ARC"));
    }

    let magic = &data[0..4];
    if magic != b"ARC\x00" && magic != b"ARC " {
        // Try alternative magic
        println!("Warning: Unknown ARC magic: {:?}", magic);
    }

    let file_count = u32::from_le_bytes(data[4..8].try_into().unwrap());
    println!("ARC file count: {}", file_count);

    if verbose {
        println!("Archive size: {} bytes", data.len());
    }

    // For now, just copy the archive to output for manual inspection
    // Full ARC parsing would require reverse-engineering the exact format
    std::fs::write(output.join("archive_copy.arc"), &data)?;
    println!("Archive copied to output for inspection");

    Ok(())
}

// ========================================================================
// PCK (Texture Pack) Extraction
// ========================================================================

fn extract_pck_command(pck: PathBuf, output: PathBuf, verbose: bool) -> Result<()> {
    println!("Extracting PCK: {:?} -> {:?}", pck, output);
    std::fs::create_dir_all(&output)?;

    // PCK files in LCE are texture pack containers
    // Format varies by platform (x16Data.pck = 16x16, x32Data.pck = 32x32, TexturePack.pck = HD)
    let data = std::fs::read(&pck).with_context(|| format!("Failed to read PCK: {:?}", pck))?;

    println!("PCK size: {} bytes", data.len());

    // For now, copy for inspection
    std::fs::write(output.join("pack_copy.pck"), &data)?;
    println!("PCK copied to output for inspection");

    Ok(())
}

// ========================================================================
// Audio Conversion (XWB/XSB -> Ogg Vorbis)
// ========================================================================

fn convert_audio_command(source: PathBuf, output: PathBuf, verbose: bool) -> Result<()> {
    println!("Converting audio: {:?} -> {:?}", source, output);
    std::fs::create_dir_all(&output)?;

    // LCE uses XACT (XACT Audio Creation Tool) format:
    // - .xgs = project file (XML-like binary)
    // - .xsb = sound bank (binary)
    // - .xwb = wave bank (binary audio data)
    //
    // Conversion requires vgmstream or xact3tool
    // For now, we'll use vgmstream-cli if available

    let xgs_files: Vec<PathBuf> = WalkDir::new(&source)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|e| e == "xgs").unwrap_or(false))
        .map(|e| e.path().to_path_buf())
        .collect();

    let xsb_files: Vec<PathBuf> = WalkDir::new(&source)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|e| e == "xsb").unwrap_or(false))
        .map(|e| e.path().to_path_buf())
        .collect();

    let xwb_files: Vec<PathBuf> = WalkDir::new(&source)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|e| e == "xwb").unwrap_or(false))
        .map(|e| e.path().to_path_buf())
        .collect();

    println!(
        "Found {} .xgs, {} .xsb, {} .xwb files",
        xgs_files.len(),
        xsb_files.len(),
        xwb_files.len()
    );

    for xgs in &xgs_files {
        println!("  XGS: {:?}", xgs);
    }
    for xsb in &xsb_files {
        println!("  XSB: {:?}", xsb);
    }
    for xwb in &xwb_files {
        println!("  XWB: {:?} ({} bytes)", xwb, std::fs::metadata(xwb)?.len());
    }

    // Try to use vgmstream-cli if available
    let vgmstream_available = std::process::Command::new("vgmstream-cli")
        .arg("--version")
        .output()
        .is_ok();

    if vgmstream_available {
        println!("vgmstream-cli found, attempting conversion...");

        for xwb in &xwb_files {
            let output_dir = output.join(xwb.file_stem().unwrap_or_default());
            std::fs::create_dir_all(&output_dir)?;

            let status = std::process::Command::new("vgmstream-cli")
                .args(["-o", output_dir.to_str().unwrap(), xwb.to_str().unwrap()])
                .status()?;

            if status.success() {
                println!("  Converted: {:?}", xwb);
            } else {
                println!("  Failed to convert: {:?}", xwb);
            }
        }
    } else {
        println!("vgmstream-cli not found. Please install vgmstream for audio conversion.");
        println!("  Ubuntu: sudo apt install vgmstream-cli");
        println!("  Windows: Download from https://github.com/vgmstream/vgmstream/releases");
        println!("  Manual conversion needed for XWB/XSB -> Ogg Vorbis");
    }

    // Copy XGS/XSB for reference
    for xgs in &xgs_files {
        std::fs::copy(xgs, output.join(xgs.file_name().unwrap()))?;
    }
    for xsb in &xsb_files {
        std::fs::copy(xsb, output.join(xsb.file_name().unwrap()))?;
    }

    Ok(())
}

// ========================================================================
// Main Conversion Command
// ========================================================================

fn convert_command(
    source: PathBuf,
    output: PathBuf,
    texture_format: TextureFormat,
    mipmaps: bool,
    verbose: bool,
) -> Result<()> {
    println!("Converting LCE assets from {:?} to {:?}", source, output);
    println!("Texture format: {:?}, mipmaps: {}", texture_format, mipmaps);

    // Create output directory structure
    std::fs::create_dir_all(&output)?;
    std::fs::create_dir_all(output.join("textures/block"))?;
    std::fs::create_dir_all(output.join("textures/item"))?;
    std::fs::create_dir_all(output.join("models/block"))?;
    std::fs::create_dir_all(output.join("models/item"))?;
    std::fs::create_dir_all(output.join("lang"))?;
    std::fs::create_dir_all(output.join("sounds"))?;
    std::fs::create_dir_all(output.join("font"))?;
    std::fs::create_dir_all(output.join("shaders"))?;

    // Create VFS for source assets
    let mut vfs = AssetVfs::new();
    vfs.add_root(source.clone());

    // Create asset manager
    let mut asset_manager = AssetManager::new();
    asset_manager.add_root(source.clone());

    // ========================================================================
    // 1. Extract and convert textures from Common/res
    // ========================================================================
    println!("\n=== Converting Textures ===");
    let texture_files = scan_lce_textures(&vfs, &source)?;
    println!("Found {} texture files", texture_files.len());

    // Convert textures (consume texture_files)
    for (id, path) in texture_files {
        if verbose {
            println!("Converting texture: {} ({})", id, path.display());
        }
        convert_texture(
            &mut asset_manager,
            &id,
            &path,
            &source,
            texture_format,
            mipmaps,
        )?;
    }

    // Re-scan for copying files (we need the texture files again for copying)
    let texture_files_for_copy = scan_lce_textures(&vfs, &source)?;

    // ========================================================================
    // 2. Convert language files
    // ========================================================================
    println!("\n=== Converting Language Files ===");
    let lang_files = scan_lce_language_files(&vfs, &source)?;
    println!("Found {} language files", lang_files.len());

    for (lang_code, path) in lang_files {
        if verbose {
            println!("Converting language: {} ({})", lang_code, path.display());
        }
        convert_language_file(&mut asset_manager, &lang_code, &path)?;
    }

    // ========================================================================
    // 3. Build texture atlas
    // ========================================================================
    println!("\n=== Building Texture Atlas ===");
    if verbose {
        println!("Building texture atlas...");
    }
    build_atlas(&mut asset_manager)?;

    // ========================================================================
    // 4. Save texture atlas image and copy texture files
    // ========================================================================
    println!("\n=== Saving Texture Atlas ===");
    save_texture_atlas(&asset_manager, &output)?;

    // Copy individual texture files to output for direct loading
    println!("\n=== Copying Texture Files ===");
    copy_texture_files(&vfs, &source, &output, &texture_files_for_copy, verbose)?;

    // ========================================================================
    // 5. Handle audio conversion
    // ========================================================================
    println!("\n=== Audio Conversion ===");
    let audio_source = source.join("Minecraft.Client/Common/res/audio");
    if audio_source.exists() {
        let audio_output = output.join("sounds");
        println!("Converting audio from {:?}", audio_source);
        convert_audio_command(audio_source, audio_output, verbose)?;
    } else {
        println!("No audio directory found at {:?}", audio_source);
    }

    // ========================================================================
    // 4. Extract DLC texture packs
    // ========================================================================
    println!("\n=== Extracting DLC Packs ===");
    let dlc_source = source.join("Minecraft.Client/Common/res/TitleUpdate/DLC");
    if dlc_source.exists() {
        extract_dlc_packs(&dlc_source, &output, verbose)?;
    } else {
        println!("No DLC directory found at {:?}", dlc_source);
    }

    // ========================================================================
    // 5. Write pack metadata
    // ========================================================================
    println!("\n=== Writing Pack Metadata ===");
    write_pack_mcmeta(&output, 15, "MCLRE Converted Assets")?;
    write_pack_toml(&output)?;

    // Save asset manifest
    save_manifest(&output, &asset_manager)?;

    println!("\nâœ“ Conversion complete! Output: {:?}", output);
    Ok(())
}

// ========================================================================
// LCE Texture Scanning
// ========================================================================

fn scan_lce_textures(vfs: &AssetVfs, source: &Path) -> Result<Vec<(Identifier, PathBuf)>> {
    let mut textures = Vec::new();

    // LCE texture locations
    let texture_dirs = [
        "Minecraft.Client/Common/res/terrain",
        "Minecraft.Client/Common/res/item",
        "Minecraft.Client/Common/res/art",
        "Minecraft.Client/Common/res/environment",
        "Minecraft.Client/Common/res/gui",
        "Minecraft.Client/Common/res/mob",
        "Minecraft.Client/Common/res/misc",
        "Minecraft.Client/Common/res/armor",
        "Minecraft.Client/Common/res/achievement",
        // TitleUpdate texture directories (animated textures, etc.)
        "Minecraft.Client/Common/res/TitleUpdate/res/textures/blocks",
        "Minecraft.Client/Common/res/TitleUpdate/res/textures/items",
        "Minecraft.Client/Common/res/TitleUpdate/res/mob",
        "Minecraft.Client/Common/res/TitleUpdate/res/armor",
        "Minecraft.Client/Common/res/TitleUpdate/res/art",
        "Minecraft.Client/Common/res/TitleUpdate/res/environment",
        "Minecraft.Client/Common/res/TitleUpdate/res/gui",
        "Minecraft.Client/Common/res/TitleUpdate/res/misc",
        "Minecraft.Client/Common/res/TitleUpdate/res/item",
    ];

    for dir in texture_dirs {
        if let Ok(files) = vfs.list_dir(dir) {
            for file in files {
                let ext = file.to_lowercase();
                if ext.ends_with(".png") || ext.ends_with(".dds") || ext.ends_with(".tga") {
                    let full_path = format!("{}/{}", dir, file);
                    if let Some(data) = vfs.read_file(&full_path).ok() {
                        if image::load_from_memory(&data).is_ok() {
                            let name = Path::new(&file)
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();

                            let (namespace, path_type) = if dir.contains("item") {
                                ("minecraft", "item")
                            } else if dir.contains("gui")
                                || dir.contains("environment")
                                || dir.contains("art")
                                || dir.contains("mob")
                                || dir.contains("misc")
                                || dir.contains("armor")
                                || dir.contains("achievement")
                            {
                                ("minecraft", "misc")
                            } else {
                                ("minecraft", "block")
                            };

                            let id_str = format!("{}:{}/{}", namespace, path_type, name);
                            if let Ok(id) = Identifier::parse(&id_str) {
                                let temp_path = source.join(&full_path);
                                textures.push((id, temp_path));
                            }
                        }
                    }
                }
            }
        }
    }

    // Extract textures from terrain.png atlas (LCE stores block textures in a single atlas file)
    // PS3 DefaultTexturePack resolves terrain.png from Common/res/TitleUpdate/res.
    // Use the TU atlas first and fall back to the base atlas only when it is absent.
    let terrain_atlas_path = [
        "Minecraft.Client/Common/res/TitleUpdate/res/terrain.png",
        "Minecraft.Client/Common/res/terrain.png",
    ]
    .into_iter()
    .find(|path| vfs.exists(path));

    if let Some(atlas_path) = terrain_atlas_path {
        let data = vfs.read_file(atlas_path)?;
        let atlas_img = image::load_from_memory(&data)
            .with_context(|| format!("Failed to decode terrain atlas: {atlas_path}"))?;
        let extracted = extract_terrain_textures(atlas_img.to_rgba8(), source.join(atlas_path))?;
        textures.extend(extracted);
    } else {
        println!("  Warning: no terrain.png found in TU or base resource paths");
    }

    // Extract items.png (item textures)
    let items_atlas_path = [
        "Minecraft.Client/Common/res/TitleUpdate/res/items.png",
        "Minecraft.Client/Common/res/gui/items.png",
        "Minecraft.Client/Common/res/1_2_2/gui/items.png",
    ]
    .into_iter()
    .find(|path| vfs.exists(path));

    if let Some(atlas_path) = items_atlas_path {
        let data = vfs.read_file(atlas_path)?;
        let atlas_img = image::load_from_memory(&data)
            .with_context(|| format!("Failed to decode items atlas: {atlas_path}"))?;
        let extracted = extract_item_textures(atlas_img.to_rgba8(), source.join(atlas_path))?;
        textures.extend(extracted);
    } else {
        println!("  Warning: no items.png found in TU, base, or 1_2_2 resource paths");
    }

    // Also scan DLC texture packs
    let dlc_dir = "Minecraft.Client/Common/res/TitleUpdate/DLC";
    if let Ok(dlc_dirs) = vfs.list_dir(dlc_dir) {
        for dlc in dlc_dirs {
            let data_path = format!("{}/{}/Data", dlc_dir, dlc);
            if let Ok(files) = vfs.list_dir(&data_path) {
                for file in files {
                    if file.to_lowercase().ends_with(".png")
                        || file.to_lowercase().ends_with(".dds")
                    {
                        let full_path = format!("{}/{}", data_path, file);
                        if let Some(data) = vfs.read_file(&full_path).ok() {
                            if image::load_from_memory(&data).is_ok() {
                                let name = Path::new(&file)
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();

                                let id_str = format!("minecraft:dlc/{}/{}", dlc, name);
                                if let Ok(id) = Identifier::parse(&id_str) {
                                    let temp_path = source.join(&full_path);
                                    textures.push((id, temp_path));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(textures)
}

/// Extracts regions from the LCE terrain atlas using the authoritative
/// PreStitchedTextureMap::loadUVs() 16-column by 32-row UV grid.
/// Cell dimensions are derived from the actual image, so HD texture packs work too.
/// Some original icons (water_flow and lava_flow) span 2x2 cells.
fn extract_terrain_textures(
    atlas: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    source_path: PathBuf,
) -> Result<Vec<(Identifier, PathBuf)>> {
    let mut textures = Vec::new();
    let (width, height) = atlas.dimensions();

    const GRID_COLUMNS: u32 = 16;
    const GRID_ROWS: u32 = 32;

    if width == 0 || height == 0 || width % GRID_COLUMNS != 0 || height % GRID_ROWS != 0 {
        anyhow::bail!(
            "Invalid terrain atlas dimensions {}x{}; expected dimensions divisible by 16x32",
            width,
            height
        );
    }

    let cell_width = width / GRID_COLUMNS;
    let cell_height = height / GRID_ROWS;
    let tiles_x = GRID_COLUMNS;
    let tiles_y = GRID_ROWS;

    println!(
        "  Extracting {}x{} = {} textures from terrain atlas ({}x{})",
        tiles_x,
        tiles_y,
        tiles_x * tiles_y,
        width,
        height
    );

    // This coordinate table follows PreStitchedTextureMap.cpp's terrain branch.
    // Format: (row, column) -> Java Edition-compatible texture basename.
    let mut lce_terrain_map: HashMap<(u32, u32), &'static str> = HashMap::new();

    // Row 0 (y=0): grass_top, stone, dirt, grass_side, planks, slabs, brick, TNT, web, flowers, portal, sapling.
    lce_terrain_map.insert((0, 0), "grass_top");
    lce_terrain_map.insert((0, 1), "stone");
    lce_terrain_map.insert((0, 2), "dirt");
    lce_terrain_map.insert((0, 3), "grass_side");
    lce_terrain_map.insert((0, 4), "planks_oak");
    lce_terrain_map.insert((0, 5), "stone_slab_side");
    lce_terrain_map.insert((0, 6), "stone_slab_top");
    lce_terrain_map.insert((0, 7), "brick");
    lce_terrain_map.insert((0, 8), "tnt_side");
    lce_terrain_map.insert((0, 9), "tnt_top");
    lce_terrain_map.insert((0, 10), "tnt_bottom");
    lce_terrain_map.insert((0, 11), "web");
    lce_terrain_map.insert((0, 12), "flower_rose");
    lce_terrain_map.insert((0, 13), "flower_dandelion");
    lce_terrain_map.insert((0, 14), "portal");
    lce_terrain_map.insert((0, 15), "sapling");

    // Row 1 (y=1): cobblestone, bedrock, sand, gravel, log, log_top, iron, gold, diamond, emerald, redstone, ?, ?, ?, fire
    lce_terrain_map.insert((1, 0), "cobblestone");
    lce_terrain_map.insert((1, 1), "bedrock");
    lce_terrain_map.insert((1, 2), "sand");
    lce_terrain_map.insert((1, 3), "gravel");
    lce_terrain_map.insert((1, 4), "log_oak");
    lce_terrain_map.insert((1, 5), "log_oak_top");
    lce_terrain_map.insert((1, 6), "iron_block");
    lce_terrain_map.insert((1, 7), "gold_block");
    lce_terrain_map.insert((1, 8), "diamond_block");
    lce_terrain_map.insert((1, 9), "emerald_block");
    lce_terrain_map.insert((1, 10), "redstone_block");
    lce_terrain_map.insert((1, 11), "dropper_front_horizontal");
    lce_terrain_map.insert((1, 12), "mushroom_red");
    lce_terrain_map.insert((1, 13), "mushroom_brown");
    lce_terrain_map.insert((1, 14), "sapling_jungle");
    lce_terrain_map.insert((1, 15), "fire_0");

    // Row 2 (y=2): ores, bookshelf, mossy, obsidian, grass_overlay, tallgrass, dispenser, beacon, dropper, workbench, furnace x2, fire
    lce_terrain_map.insert((2, 0), "gold_ore");
    lce_terrain_map.insert((2, 1), "iron_ore");
    lce_terrain_map.insert((2, 2), "coal_ore");
    lce_terrain_map.insert((2, 3), "bookshelf");
    lce_terrain_map.insert((2, 4), "cobblestone_mossy");
    lce_terrain_map.insert((2, 5), "obsidian");
    lce_terrain_map.insert((2, 6), "grass_side_overlay");
    lce_terrain_map.insert((2, 7), "tallgrass");
    lce_terrain_map.insert((2, 8), "dispenser_front_vertical");
    lce_terrain_map.insert((2, 9), "beacon");
    lce_terrain_map.insert((2, 10), "dropper_front_vertical");
    lce_terrain_map.insert((2, 11), "workbench_top");
    lce_terrain_map.insert((2, 12), "furnace_front");
    lce_terrain_map.insert((2, 13), "furnace_side");
    lce_terrain_map.insert((2, 14), "dispenser_front");
    lce_terrain_map.insert((2, 15), "fire_1");

    // Row 3 (y=3): sponge, glass, diamond_ore, redstone_ore, leaves x2, stonebrick, deadbush, fern, daylight, workbench, furnace, sapling
    lce_terrain_map.insert((3, 0), "sponge");
    lce_terrain_map.insert((3, 1), "glass");
    lce_terrain_map.insert((3, 2), "diamond_ore");
    lce_terrain_map.insert((3, 3), "redstone_ore");
    lce_terrain_map.insert((3, 4), "leaves");
    lce_terrain_map.insert((3, 5), "leaves_opaque");
    lce_terrain_map.insert((3, 6), "stonebrick");
    lce_terrain_map.insert((3, 7), "deadbush");
    lce_terrain_map.insert((3, 8), "fern");
    lce_terrain_map.insert((3, 9), "daylight_detector_top");
    lce_terrain_map.insert((3, 10), "daylight_detector_side");
    lce_terrain_map.insert((3, 11), "workbench_side");
    lce_terrain_map.insert((3, 12), "workbench_front");
    lce_terrain_map.insert((3, 13), "furnace_front_lit");
    lce_terrain_map.insert((3, 14), "furnace_top");
    lce_terrain_map.insert((3, 15), "sapling_spruce");

    // Row 4 (y=4): wool_white, mob_spawner, snow, ice, snow_side, cactus x3, clay, reeds, jukebox x2, waterlily, mycel x2, sapling_birch
    lce_terrain_map.insert((4, 0), "wool_colored_white");
    lce_terrain_map.insert((4, 1), "mob_spawner");
    lce_terrain_map.insert((4, 2), "snow");
    lce_terrain_map.insert((4, 3), "ice");
    lce_terrain_map.insert((4, 4), "snow_side");
    lce_terrain_map.insert((4, 5), "cactus_top");
    lce_terrain_map.insert((4, 6), "cactus_side");
    lce_terrain_map.insert((4, 7), "cactus_bottom");
    lce_terrain_map.insert((4, 8), "clay");
    lce_terrain_map.insert((4, 9), "reeds");
    lce_terrain_map.insert((4, 10), "jukebox_side");
    lce_terrain_map.insert((4, 11), "jukebox_top");
    lce_terrain_map.insert((4, 12), "waterlily");
    lce_terrain_map.insert((4, 13), "mycel_side");
    lce_terrain_map.insert((4, 14), "mycel_top");
    lce_terrain_map.insert((4, 15), "sapling_birch");

    // Row 5 (y=5): torch, doors x2, ladder, trapdoor, iron_bars, farmland x2, crops x8
    lce_terrain_map.insert((5, 0), "torch_on");
    lce_terrain_map.insert((5, 1), "door_wood_upper");
    lce_terrain_map.insert((5, 2), "door_iron_upper");
    lce_terrain_map.insert((5, 3), "ladder");
    lce_terrain_map.insert((5, 4), "trapdoor");
    lce_terrain_map.insert((5, 5), "iron_bars");
    lce_terrain_map.insert((5, 6), "farmland_wet");
    lce_terrain_map.insert((5, 7), "farmland_dry");
    lce_terrain_map.insert((5, 8), "crops_0");
    lce_terrain_map.insert((5, 9), "crops_1");
    lce_terrain_map.insert((5, 10), "crops_2");
    lce_terrain_map.insert((5, 11), "crops_3");
    lce_terrain_map.insert((5, 12), "crops_4");
    lce_terrain_map.insert((5, 13), "crops_5");
    lce_terrain_map.insert((5, 14), "crops_6");
    lce_terrain_map.insert((5, 15), "crops_7");

    // Row 6 (y=6): lever, doors x2, redstone_torch_on, stonebrick x2, pumpkin_top, netherrack, soul_sand, glowstone, piston x4, stem
    lce_terrain_map.insert((6, 0), "lever");
    lce_terrain_map.insert((6, 1), "door_wood_lower");
    lce_terrain_map.insert((6, 2), "door_iron_lower");
    lce_terrain_map.insert((6, 3), "redstone_torch_on");
    lce_terrain_map.insert((6, 4), "stonebrick_mossy");
    lce_terrain_map.insert((6, 5), "stonebrick_cracked");
    lce_terrain_map.insert((6, 6), "pumpkin_top");
    lce_terrain_map.insert((6, 7), "netherrack");
    lce_terrain_map.insert((6, 8), "soul_sand");
    lce_terrain_map.insert((6, 9), "glowstone");
    lce_terrain_map.insert((6, 10), "piston_top_sticky");
    lce_terrain_map.insert((6, 11), "piston_top");
    lce_terrain_map.insert((6, 12), "piston_side");
    lce_terrain_map.insert((6, 13), "piston_bottom");
    lce_terrain_map.insert((6, 14), "piston_inner_top");
    lce_terrain_map.insert((6, 15), "stem_straight");

    // Row 7 (y=7): rail_turned, wool_black, wool_gray, redstone_torch_off, logs x2, pumpkin x3, cake x4, mushroom x2, stem
    lce_terrain_map.insert((7, 0), "rail_normal_turned");
    lce_terrain_map.insert((7, 1), "wool_colored_black");
    lce_terrain_map.insert((7, 2), "wool_colored_gray");
    lce_terrain_map.insert((7, 3), "redstone_torch_off");
    lce_terrain_map.insert((7, 4), "log_spruce");
    lce_terrain_map.insert((7, 5), "log_birch");
    lce_terrain_map.insert((7, 6), "pumpkin_side");
    lce_terrain_map.insert((7, 7), "pumpkin_face_off");
    lce_terrain_map.insert((7, 8), "pumpkin_face_on");
    lce_terrain_map.insert((7, 9), "cake_top");
    lce_terrain_map.insert((7, 10), "cake_side");
    lce_terrain_map.insert((7, 11), "cake_inner");
    lce_terrain_map.insert((7, 12), "cake_bottom");
    lce_terrain_map.insert((7, 13), "mushroom_block_skin_red");
    lce_terrain_map.insert((7, 14), "mushroom_block_skin_brown");
    lce_terrain_map.insert((7, 15), "stem_bent");

    // Row 8 (y=8): rail, wool_red, wool_pink, repeater_off, leaves x2, bed x2, melon x2, cauldron x2, mushroom, vine
    lce_terrain_map.insert((8, 0), "rail_normal");
    lce_terrain_map.insert((8, 1), "wool_colored_red");
    lce_terrain_map.insert((8, 2), "wool_colored_pink");
    lce_terrain_map.insert((8, 3), "repeater_off");
    lce_terrain_map.insert((8, 4), "leaves_spruce");
    lce_terrain_map.insert((8, 5), "leaves_spruce_opaque");
    lce_terrain_map.insert((8, 6), "bed_feet_top");
    lce_terrain_map.insert((8, 7), "bed_head_top");
    lce_terrain_map.insert((8, 8), "melon_side");
    lce_terrain_map.insert((8, 9), "melon_top");
    lce_terrain_map.insert((8, 10), "cauldron_top");
    lce_terrain_map.insert((8, 11), "cauldron_inner");
    lce_terrain_map.insert((8, 13), "mushroom_block_skin_stem");
    lce_terrain_map.insert((8, 14), "mushroom_block_inside");
    lce_terrain_map.insert((8, 15), "vine");

    // Row 9 (y=9): lapis_block CONFIRMED BLUE, wool_green, wool_lime, repeater_on, glass_pane, bed x4, log_jungle, cauldron x3, brewing x2, endframe x2
    lce_terrain_map.insert((9, 0), "lapis_block");
    lce_terrain_map.insert((9, 1), "wool_colored_green");
    lce_terrain_map.insert((9, 2), "wool_colored_lime");
    lce_terrain_map.insert((9, 3), "repeater_on");
    lce_terrain_map.insert((9, 4), "glass_pane_top");
    lce_terrain_map.insert((9, 5), "bed_feet_end");
    lce_terrain_map.insert((9, 6), "bed_feet_side");
    lce_terrain_map.insert((9, 7), "bed_head_side");
    lce_terrain_map.insert((9, 8), "bed_head_end");
    lce_terrain_map.insert((9, 9), "log_jungle");
    lce_terrain_map.insert((9, 10), "cauldron_side");
    lce_terrain_map.insert((9, 11), "cauldron_bottom");
    lce_terrain_map.insert((9, 12), "brewing_stand_base");
    lce_terrain_map.insert((9, 13), "brewing_stand");
    lce_terrain_map.insert((9, 14), "endframe_top");
    lce_terrain_map.insert((9, 15), "endframe_side");

    // Row 10 (y=10): lapis_ore, wool_brown, wool_yellow, rail_golden, redstone_dust x2, enchantment, dragon_egg, cocoa x3, emerald_ore, tripwire x2, endframe, end_stone
    lce_terrain_map.insert((10, 0), "lapis_ore");
    lce_terrain_map.insert((10, 1), "wool_colored_brown");
    lce_terrain_map.insert((10, 2), "wool_colored_yellow");
    lce_terrain_map.insert((10, 3), "rail_golden");
    lce_terrain_map.insert((10, 4), "redstone_dust_cross");
    lce_terrain_map.insert((10, 5), "redstone_dust_line");
    lce_terrain_map.insert((10, 6), "enchantment_top");
    lce_terrain_map.insert((10, 7), "dragon_egg");
    lce_terrain_map.insert((10, 8), "cocoa_2");
    lce_terrain_map.insert((10, 9), "cocoa_1");
    lce_terrain_map.insert((10, 10), "cocoa_0");
    lce_terrain_map.insert((10, 11), "emerald_ore");
    lce_terrain_map.insert((10, 12), "trip_wire_source");
    lce_terrain_map.insert((10, 13), "trip_wire");
    lce_terrain_map.insert((10, 14), "endframe_eye");
    lce_terrain_map.insert((10, 15), "end_stone");

    // Row 11 (y=11): sandstone_top, wool_blue, wool_light_blue, rail_golden_powered, redstone_overlay x2, enchantment x2, command_block, itemframe, flower_pot, comparator x2, rail_activator x2, quartz_ore
    lce_terrain_map.insert((11, 0), "sandstone_top");
    lce_terrain_map.insert((11, 1), "wool_colored_blue");
    lce_terrain_map.insert((11, 2), "wool_colored_light_blue");
    lce_terrain_map.insert((11, 3), "rail_golden_powered");
    lce_terrain_map.insert((11, 4), "redstone_dust_cross_overlay");
    lce_terrain_map.insert((11, 5), "redstone_dust_line_overlay");
    lce_terrain_map.insert((11, 6), "enchantment_side");
    lce_terrain_map.insert((11, 7), "enchantment_bottom");
    lce_terrain_map.insert((11, 8), "command_block");
    lce_terrain_map.insert((11, 9), "itemframe_back");
    lce_terrain_map.insert((11, 10), "flower_pot");
    lce_terrain_map.insert((11, 11), "comparator_off");
    lce_terrain_map.insert((11, 12), "comparator_on");
    lce_terrain_map.insert((11, 13), "rail_activator");
    lce_terrain_map.insert((11, 14), "rail_activator_powered");
    lce_terrain_map.insert((11, 15), "quartz_ore");

    // Row 12 (y=12): sandstone_side, wool_purple, wool_magenta, detector_rail, leaves_jungle x2, planks x2, carrots x4, water x2
    lce_terrain_map.insert((12, 0), "sandstone_side");
    lce_terrain_map.insert((12, 1), "wool_colored_purple");
    lce_terrain_map.insert((12, 2), "wool_colored_magenta");
    lce_terrain_map.insert((12, 3), "detector_rail");
    lce_terrain_map.insert((12, 4), "leaves_jungle");
    lce_terrain_map.insert((12, 5), "leaves_jungle_opaque");
    lce_terrain_map.insert((12, 6), "planks_spruce");
    lce_terrain_map.insert((12, 7), "planks_jungle");
    lce_terrain_map.insert((12, 8), "carrots_stage_0");
    lce_terrain_map.insert((12, 9), "carrots_stage_1");
    lce_terrain_map.insert((12, 10), "carrots_stage_2");
    lce_terrain_map.insert((12, 11), "carrots_stage_3");
    lce_terrain_map.insert((12, 13), "water_still");
    lce_terrain_map.insert((12, 14), "water_flow"); // Original ADD_ICON_SIZE: 2x2 cells

    // Row 13 (y=13): sandstone_bottom, wool_cyan, wool_orange, redstone_lamp x2, stonebrick_carved, planks_birch, anvil x2, quartz x3, hopper, detector_rail_on
    lce_terrain_map.insert((13, 0), "sandstone_bottom");
    lce_terrain_map.insert((13, 1), "wool_colored_cyan");
    lce_terrain_map.insert((13, 2), "wool_colored_orange");
    lce_terrain_map.insert((13, 3), "redstone_lamp_off");
    lce_terrain_map.insert((13, 4), "redstone_lamp_on");
    lce_terrain_map.insert((13, 5), "stonebrick_carved");
    lce_terrain_map.insert((13, 6), "planks_birch");
    lce_terrain_map.insert((13, 7), "anvil_base");
    lce_terrain_map.insert((13, 8), "anvil_top_damaged_1");
    lce_terrain_map.insert((13, 9), "quartz_block_chiseled_top");
    lce_terrain_map.insert((13, 10), "quartz_block_lines_top");
    lce_terrain_map.insert((13, 11), "quartz_block_top");
    lce_terrain_map.insert((13, 12), "hopper_outside");
    lce_terrain_map.insert((13, 13), "detector_rail_on");

    // Row 14 (y=14): nether_brick, wool_silver, nether_wart x3, sandstone x2, anvil x2, quartz x3, hopper, lava x2
    lce_terrain_map.insert((14, 0), "nether_brick");
    lce_terrain_map.insert((14, 1), "wool_colored_silver");
    lce_terrain_map.insert((14, 2), "nether_wart_stage_0");
    lce_terrain_map.insert((14, 3), "nether_wart_stage_1");
    lce_terrain_map.insert((14, 4), "nether_wart_stage_2");
    lce_terrain_map.insert((14, 5), "sandstone_carved");
    lce_terrain_map.insert((14, 6), "sandstone_smooth");
    lce_terrain_map.insert((14, 7), "anvil_top");
    lce_terrain_map.insert((14, 8), "anvil_top_damaged_2");
    lce_terrain_map.insert((14, 9), "quartz_block_chiseled");
    lce_terrain_map.insert((14, 10), "quartz_block_lines");
    lce_terrain_map.insert((14, 11), "quartz_block_side");
    lce_terrain_map.insert((14, 12), "hopper_inside");
    lce_terrain_map.insert((14, 13), "lava_still");
    lce_terrain_map.insert((14, 14), "lava_flow"); // Original ADD_ICON_SIZE: 2x2 cells

    // Row 15 (y=15): destroy x10, hay_block x2, quartz_bottom, hopper_top
    lce_terrain_map.insert((15, 0), "destroy_0");
    lce_terrain_map.insert((15, 1), "destroy_1");
    lce_terrain_map.insert((15, 2), "destroy_2");
    lce_terrain_map.insert((15, 3), "destroy_3");
    lce_terrain_map.insert((15, 4), "destroy_4");
    lce_terrain_map.insert((15, 5), "destroy_5");
    lce_terrain_map.insert((15, 6), "destroy_6");
    lce_terrain_map.insert((15, 7), "destroy_7");
    lce_terrain_map.insert((15, 8), "destroy_8");
    lce_terrain_map.insert((15, 9), "destroy_9");
    lce_terrain_map.insert((15, 10), "hay_block_side");
    lce_terrain_map.insert((15, 11), "quartz_block_bottom");
    lce_terrain_map.insert((15, 12), "hopper_top");
    lce_terrain_map.insert((15, 13), "hay_block_top");

    // Row 16 (y=16): coal_block, hardened_clay, noteblock, potatoes x4, log tops x3
    lce_terrain_map.insert((16, 0), "coal_block");
    lce_terrain_map.insert((16, 1), "hardened_clay");
    lce_terrain_map.insert((16, 2), "noteblock");
    lce_terrain_map.insert((16, 9), "potatoes_stage_0");
    lce_terrain_map.insert((16, 10), "potatoes_stage_1");
    lce_terrain_map.insert((16, 11), "potatoes_stage_2");
    lce_terrain_map.insert((16, 12), "potatoes_stage_3");
    lce_terrain_map.insert((16, 13), "log_spruce_top");
    lce_terrain_map.insert((16, 14), "log_jungle_top");
    lce_terrain_map.insert((16, 15), "log_birch_top");

    // Row 17 - Hardened clay stained variants (16 colors)
    lce_terrain_map.insert((17, 0), "hardened_clay_stained_black");
    lce_terrain_map.insert((17, 1), "hardened_clay_stained_blue");
    lce_terrain_map.insert((17, 2), "hardened_clay_stained_brown");
    lce_terrain_map.insert((17, 3), "hardened_clay_stained_cyan");
    lce_terrain_map.insert((17, 4), "hardened_clay_stained_gray");
    lce_terrain_map.insert((17, 5), "hardened_clay_stained_green");
    lce_terrain_map.insert((17, 6), "hardened_clay_stained_light_blue");
    lce_terrain_map.insert((17, 7), "hardened_clay_stained_lime");
    lce_terrain_map.insert((17, 8), "hardened_clay_stained_magenta");
    lce_terrain_map.insert((17, 9), "hardened_clay_stained_orange");
    lce_terrain_map.insert((17, 10), "hardened_clay_stained_pink");
    lce_terrain_map.insert((17, 11), "hardened_clay_stained_purple");
    lce_terrain_map.insert((17, 12), "hardened_clay_stained_red");
    lce_terrain_map.insert((17, 13), "hardened_clay_stained_silver");
    lce_terrain_map.insert((17, 14), "hardened_clay_stained_white");
    lce_terrain_map.insert((17, 15), "hardened_clay_stained_yellow");

    // Row 18 - Stained glass (semi-transparent)
    lce_terrain_map.insert((18, 0), "glass_black");
    lce_terrain_map.insert((18, 1), "glass_blue");
    lce_terrain_map.insert((18, 2), "glass_brown");
    lce_terrain_map.insert((18, 3), "glass_cyan");
    lce_terrain_map.insert((18, 4), "glass_gray");
    lce_terrain_map.insert((18, 5), "glass_green");
    lce_terrain_map.insert((18, 6), "glass_light_blue");
    lce_terrain_map.insert((18, 7), "glass_lime");
    lce_terrain_map.insert((18, 8), "glass_magenta");
    lce_terrain_map.insert((18, 9), "glass_orange");
    lce_terrain_map.insert((18, 10), "glass_pink");
    lce_terrain_map.insert((18, 11), "glass_purple");
    lce_terrain_map.insert((18, 12), "glass_red");
    lce_terrain_map.insert((18, 13), "glass_silver");
    lce_terrain_map.insert((18, 14), "glass_white");
    lce_terrain_map.insert((18, 15), "glass_yellow");

    // Row 19 - Stained glass pane tops
    lce_terrain_map.insert((19, 0), "glass_pane_top_black");
    lce_terrain_map.insert((19, 1), "glass_pane_top_blue");
    lce_terrain_map.insert((19, 2), "glass_pane_top_brown");
    lce_terrain_map.insert((19, 3), "glass_pane_top_cyan");
    lce_terrain_map.insert((19, 4), "glass_pane_top_gray");
    lce_terrain_map.insert((19, 5), "glass_pane_top_green");
    lce_terrain_map.insert((19, 6), "glass_pane_top_light_blue");
    lce_terrain_map.insert((19, 7), "glass_pane_top_lime");
    lce_terrain_map.insert((19, 8), "glass_pane_top_magenta");
    lce_terrain_map.insert((19, 9), "glass_pane_top_orange");
    lce_terrain_map.insert((19, 10), "glass_pane_top_pink");
    lce_terrain_map.insert((19, 11), "glass_pane_top_purple");
    lce_terrain_map.insert((19, 12), "glass_pane_top_red");
    lce_terrain_map.insert((19, 13), "glass_pane_top_silver");
    lce_terrain_map.insert((19, 14), "glass_pane_top_white");
    lce_terrain_map.insert((19, 15), "glass_pane_top_yellow");

    // Rows 20-31: Empty/transparent in the reference terrain.png

    // Original ADD_ICON_SIZE declarations: (row, col) -> (width_cells, height_cells).
    let multi_cell_sizes: HashMap<(u32, u32), (u32, u32)> = HashMap::from([
        ((12, 14), (2, 2)), // water_flow
        ((14, 14), (2, 2)), // lava_flow
    ]);

    // Keep generated files under source_root so AssetVfs can resolve their relative paths.
    let temp_dir = source_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(".assetc-generated/terrain");
    fs::create_dir_all(&temp_dir)?;

    // Iterate in atlas order so output and downstream atlas packing are deterministic.
    for row in 0..tiles_y {
        for col in 0..tiles_x {
            let Some(&name) = lce_terrain_map.get(&(row, col)) else {
                continue;
            };

            let (span_x, span_y) = multi_cell_sizes.get(&(row, col)).copied().unwrap_or((1, 1));

            let x = col * cell_width;
            let y = row * cell_height;
            let crop_width = span_x * cell_width;
            let crop_height = span_y * cell_height;

            if x + crop_width > width || y + crop_height > height {
                anyhow::bail!("Terrain icon {name} at ({row}, {col}) spans outside atlas bounds");
            }

            let tile = image::imageops::crop_imm(&atlas, x, y, crop_width, crop_height).to_image();

            let temp_path = temp_dir.join(format!("{}.png", name));
            tile.save(&temp_path)
                .with_context(|| format!("Failed to save extracted terrain texture {name}"))?;

            let id_str = format!("minecraft:block/{}", name);
            let id = Identifier::parse(&id_str)
                .with_context(|| format!("Invalid texture identifier: {id_str}"))?;
            textures.push((id, temp_path));
        }
    }

    println!(
        "  Extracted {} mapped textures from terrain atlas",
        textures.len()
    );
    Ok(textures)
}

/// Extracts individual 16x16 item textures from a items.png atlas.
/// LCE uses a 256x256 items.png (16x16 tiles = 256 item textures) in gui/items.png.
/// The texture layout is defined in PreStitchedTextureMap::loadUVs() for TYPE_ITEM.
fn extract_item_textures(
    atlas: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    source_path: PathBuf,
) -> Result<Vec<(Identifier, PathBuf)>> {
    let mut textures = Vec::new();
    let (width, height) = atlas.dimensions();

    let tile_size = 16;
    let tiles_x = width / tile_size;
    let tiles_y = height / tile_size;

    println!(
        "  Extracting {}x{} = {} textures from items atlas ({}x{})",
        tiles_x,
        tiles_y,
        tiles_x * tiles_y,
        width,
        height
    );

    // LCE items.png texture mapping from PreStitchedTextureMap::loadUVs() for TYPE_ITEM
    // The items.png is 16 columns x 16 rows = 256 item textures
    // Format: (row, col) -> icon_name
    let mut lce_items_map: HashMap<(u32, u32), &'static str> = HashMap::new();

    // Row 0
    lce_items_map.insert((0, 0), "helmet_leather");
    lce_items_map.insert((0, 1), "helmet_chain");
    lce_items_map.insert((0, 2), "helmet_iron");
    lce_items_map.insert((0, 3), "helmet_diamond");
    lce_items_map.insert((0, 4), "helmet_gold");
    lce_items_map.insert((0, 5), "flint_and_steel");
    lce_items_map.insert((0, 6), "flint");
    lce_items_map.insert((0, 7), "coal");
    lce_items_map.insert((0, 8), "string");
    lce_items_map.insert((0, 9), "seeds");
    lce_items_map.insert((0, 10), "apple");
    lce_items_map.insert((0, 11), "apple_gold");
    lce_items_map.insert((0, 12), "egg");
    lce_items_map.insert((0, 13), "sugar");
    lce_items_map.insert((0, 14), "snowball");
    lce_items_map.insert((0, 15), "slot_empty_helmet");

    // Row 1
    lce_items_map.insert((1, 0), "chestplate_leather");
    lce_items_map.insert((1, 1), "chestplate_chain");
    lce_items_map.insert((1, 2), "chestplate_iron");
    lce_items_map.insert((1, 3), "chestplate_diamond");
    lce_items_map.insert((1, 4), "chestplate_gold");
    lce_items_map.insert((1, 5), "bow");
    lce_items_map.insert((1, 6), "brick");
    lce_items_map.insert((1, 7), "iron_ingot");
    lce_items_map.insert((1, 8), "feather");
    lce_items_map.insert((1, 9), "wheat");
    lce_items_map.insert((1, 10), "painting");
    lce_items_map.insert((1, 11), "reeds");
    lce_items_map.insert((1, 12), "bone");
    lce_items_map.insert((1, 13), "cake");
    lce_items_map.insert((1, 14), "slimeball");
    lce_items_map.insert((1, 15), "slot_empty_chestplate");

    // Row 2
    lce_items_map.insert((2, 0), "leggings_leather");
    lce_items_map.insert((2, 1), "leggings_chain");
    lce_items_map.insert((2, 2), "leggings_iron");
    lce_items_map.insert((2, 3), "leggings_diamond");
    lce_items_map.insert((2, 4), "leggings_gold");
    lce_items_map.insert((2, 5), "arrow");
    lce_items_map.insert((2, 6), "quiver");
    lce_items_map.insert((2, 7), "gold_ingot");
    lce_items_map.insert((2, 8), "gunpowder");
    lce_items_map.insert((2, 9), "bread");
    lce_items_map.insert((2, 10), "sign");
    lce_items_map.insert((2, 11), "door_wood");
    lce_items_map.insert((2, 12), "door_iron");
    lce_items_map.insert((2, 13), "bed");
    lce_items_map.insert((2, 14), "fireball");
    lce_items_map.insert((2, 15), "slot_empty_leggings");

    // Row 3
    lce_items_map.insert((3, 0), "boots_leather");
    lce_items_map.insert((3, 1), "boots_chain");
    lce_items_map.insert((3, 2), "boots_iron");
    lce_items_map.insert((3, 3), "boots_diamond");
    lce_items_map.insert((3, 4), "boots_gold");
    lce_items_map.insert((3, 5), "stick");
    lce_items_map.insert((3, 6), "compass");
    lce_items_map.insert((3, 7), "diamond");
    lce_items_map.insert((3, 8), "redstone");
    lce_items_map.insert((3, 9), "clay");
    lce_items_map.insert((3, 10), "paper");
    lce_items_map.insert((3, 11), "book");
    lce_items_map.insert((3, 12), "map");
    lce_items_map.insert((3, 13), "seeds_pumpkin");
    lce_items_map.insert((3, 14), "seeds_melon");
    lce_items_map.insert((3, 15), "slot_empty_boots");

    // Row 4
    lce_items_map.insert((4, 0), "sword_wood");
    lce_items_map.insert((4, 1), "sword_stone");
    lce_items_map.insert((4, 2), "sword_iron");
    lce_items_map.insert((4, 3), "sword_diamond");
    lce_items_map.insert((4, 4), "sword_gold");
    lce_items_map.insert((4, 5), "fishing_rod_uncast");
    lce_items_map.insert((4, 6), "clock");
    lce_items_map.insert((4, 7), "bowl");
    lce_items_map.insert((4, 8), "mushroom_stew");
    lce_items_map.insert((4, 9), "glowstone_dust");
    lce_items_map.insert((4, 10), "bucket");
    lce_items_map.insert((4, 11), "bucket_water");
    lce_items_map.insert((4, 12), "bucket_lava");
    lce_items_map.insert((4, 13), "milk");
    lce_items_map.insert((4, 14), "dye_black");
    lce_items_map.insert((4, 15), "dye_gray");

    // Row 5
    lce_items_map.insert((5, 0), "shovel_wood");
    lce_items_map.insert((5, 1), "shovel_stone");
    lce_items_map.insert((5, 2), "shovel_iron");
    lce_items_map.insert((5, 3), "shovel_diamond");
    lce_items_map.insert((5, 4), "shovel_gold");
    lce_items_map.insert((5, 5), "fishing_rod_cast");
    lce_items_map.insert((5, 6), "repeater");
    lce_items_map.insert((5, 7), "porkchop_raw");
    lce_items_map.insert((5, 8), "porkchop_cooked");
    lce_items_map.insert((5, 9), "fish_raw");
    lce_items_map.insert((5, 10), "fish_cooked");
    lce_items_map.insert((5, 11), "rotten_flesh");
    lce_items_map.insert((5, 12), "cookie");
    lce_items_map.insert((5, 13), "shears");
    lce_items_map.insert((5, 14), "dye_red");
    lce_items_map.insert((5, 15), "dye_pink");

    // Row 6
    lce_items_map.insert((6, 0), "pickaxe_wood");
    lce_items_map.insert((6, 1), "pickaxe_stone");
    lce_items_map.insert((6, 2), "pickaxe_iron");
    lce_items_map.insert((6, 3), "pickaxe_diamond");
    lce_items_map.insert((6, 4), "pickaxe_gold");
    lce_items_map.insert((6, 5), "bow_pull_0");
    lce_items_map.insert((6, 6), "carrot_on_a_stick");
    lce_items_map.insert((6, 7), "leather");
    lce_items_map.insert((6, 8), "saddle");
    lce_items_map.insert((6, 9), "beef_raw");
    lce_items_map.insert((6, 10), "beef_cooked");
    lce_items_map.insert((6, 11), "ender_pearl");
    lce_items_map.insert((6, 12), "blaze_rod");
    lce_items_map.insert((6, 13), "melon_slice");
    lce_items_map.insert((6, 14), "dye_green");
    lce_items_map.insert((6, 15), "dye_lime");

    // Row 7
    lce_items_map.insert((7, 0), "axe_wood");
    lce_items_map.insert((7, 1), "axe_stone");
    lce_items_map.insert((7, 2), "axe_iron");
    lce_items_map.insert((7, 3), "axe_diamond");
    lce_items_map.insert((7, 4), "axe_gold");
    lce_items_map.insert((7, 5), "bow_pull_1");
    lce_items_map.insert((7, 6), "baked_potato");
    lce_items_map.insert((7, 7), "potato");
    lce_items_map.insert((7, 8), "carrot");
    lce_items_map.insert((7, 9), "chicken_raw");
    lce_items_map.insert((7, 10), "chicken_cooked");
    lce_items_map.insert((7, 11), "ghast_tear");
    lce_items_map.insert((7, 12), "gold_nugget");
    lce_items_map.insert((7, 13), "nether_wart");
    lce_items_map.insert((7, 14), "dye_brown");
    lce_items_map.insert((7, 15), "dye_yellow");

    // Row 8
    lce_items_map.insert((8, 0), "hoe_wood");
    lce_items_map.insert((8, 1), "hoe_stone");
    lce_items_map.insert((8, 2), "hoe_iron");
    lce_items_map.insert((8, 3), "hoe_diamond");
    lce_items_map.insert((8, 4), "hoe_gold");
    lce_items_map.insert((8, 5), "bow_pull_2");
    lce_items_map.insert((8, 6), "poisonous_potato");
    lce_items_map.insert((8, 7), "minecart");
    lce_items_map.insert((8, 8), "boat");
    lce_items_map.insert((8, 9), "glistering_melon_slice");
    lce_items_map.insert((8, 10), "fermented_spider_eye");
    lce_items_map.insert((8, 11), "spider_eye");
    lce_items_map.insert((8, 12), "potion");
    lce_items_map.insert((8, 13), "potion_contents");
    lce_items_map.insert((8, 14), "dye_blue");
    lce_items_map.insert((8, 15), "dye_light_blue");

    // Row 9
    lce_items_map.insert((9, 0), "helmet_leather_overlay");
    lce_items_map.insert((9, 2), "iron_horse_armor");
    lce_items_map.insert((9, 3), "diamond_horse_armor");
    lce_items_map.insert((9, 4), "gold_horse_armor");
    lce_items_map.insert((9, 5), "comparator");
    lce_items_map.insert((9, 6), "golden_carrot");
    lce_items_map.insert((9, 7), "minecart_chest");
    lce_items_map.insert((9, 8), "pumpkin_pie");
    lce_items_map.insert((9, 9), "spawn_egg");
    lce_items_map.insert((9, 10), "splash_potion");
    lce_items_map.insert((9, 11), "ender_eye");
    lce_items_map.insert((9, 12), "cauldron");
    lce_items_map.insert((9, 13), "blaze_powder");
    lce_items_map.insert((9, 14), "dye_purple");
    lce_items_map.insert((9, 15), "dye_magenta");

    // Row 10
    lce_items_map.insert((10, 0), "chestplate_leather_overlay");
    lce_items_map.insert((10, 3), "name_tag");
    lce_items_map.insert((10, 4), "lead");
    lce_items_map.insert((10, 5), "nether_brick");
    lce_items_map.insert((10, 7), "minecart_furnace");
    lce_items_map.insert((10, 8), "charcoal");
    lce_items_map.insert((10, 9), "spawn_egg_overlay");
    lce_items_map.insert((10, 10), "ruby");
    lce_items_map.insert((10, 11), "experience_bottle");
    lce_items_map.insert((10, 12), "brewing_stand");
    lce_items_map.insert((10, 13), "magma_cream");
    lce_items_map.insert((10, 14), "dye_cyan");
    lce_items_map.insert((10, 15), "dye_orange");

    // Row 11
    lce_items_map.insert((11, 0), "leggings_leather_overlay");
    lce_items_map.insert((11, 7), "minecart_hopper");
    lce_items_map.insert((11, 8), "hopper");
    lce_items_map.insert((11, 9), "nether_star");
    lce_items_map.insert((11, 10), "emerald");
    lce_items_map.insert((11, 11), "writable_book");
    lce_items_map.insert((11, 12), "written_book");
    lce_items_map.insert((11, 13), "flower_pot");
    lce_items_map.insert((11, 14), "dye_silver");
    lce_items_map.insert((11, 15), "dye_white");

    // Row 12
    lce_items_map.insert((12, 0), "boots_leather_overlay");
    lce_items_map.insert((12, 7), "minecart_tnt");
    lce_items_map.insert((12, 9), "firework_rocket");
    lce_items_map.insert((12, 10), "firework_charge");
    lce_items_map.insert((12, 11), "firework_charge_overlay");
    lce_items_map.insert((12, 12), "nether_quartz");
    lce_items_map.insert((12, 13), "map_empty");
    lce_items_map.insert((12, 14), "item_frame");
    lce_items_map.insert((12, 15), "enchanted_book");

    // Row 13 (mostly empty)

    // Row 14
    lce_items_map.insert((14, 0), "skull_skeleton");
    lce_items_map.insert((14, 1), "skull_wither");
    lce_items_map.insert((14, 2), "skull_zombie");
    lce_items_map.insert((14, 3), "skull_player");
    lce_items_map.insert((14, 4), "skull_creeper");
    lce_items_map.insert((14, 7), "compass"); // compassP0
    lce_items_map.insert((14, 8), "compass"); // compassP1
    lce_items_map.insert((14, 9), "compass"); // compassP2
    lce_items_map.insert((14, 10), "compass"); // compassP3
    lce_items_map.insert((14, 11), "clock"); // clockP0
    lce_items_map.insert((14, 12), "clock"); // clockP1
    lce_items_map.insert((14, 13), "clock"); // clockP2
    lce_items_map.insert((14, 14), "clock"); // clockP3
    lce_items_map.insert((14, 15), "dragon_fireball");

    // Row 15 - Music discs
    lce_items_map.insert((15, 0), "record_13");
    lce_items_map.insert((15, 1), "record_cat");
    lce_items_map.insert((15, 2), "record_blocks");
    lce_items_map.insert((15, 3), "record_chirp");
    lce_items_map.insert((15, 4), "record_far");
    lce_items_map.insert((15, 5), "record_mall");
    lce_items_map.insert((15, 6), "record_mellohi");
    lce_items_map.insert((15, 7), "record_stal");
    lce_items_map.insert((15, 8), "record_strad");
    lce_items_map.insert((15, 9), "record_ward");
    lce_items_map.insert((15, 10), "record_11");
    lce_items_map.insert((15, 11), "record_wait");

    // Extract each tile and name it according to the LCE mapping
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let x = tx * tile_size;
            let y = ty * tile_size;

            // Skip if out of bounds
            if x + tile_size > width || y + tile_size > height {
                continue;
            }

            // Get the LCE icon name for this position
            let icon_name = lce_items_map.get(&(ty, tx)).copied();

            // Only extract tiles that have a known mapping
            if let Some(name) = icon_name {
                let mut tile: ImageBuffer<Rgba<u8>, Vec<u8>> =
                    ImageBuffer::new(tile_size, tile_size);
                for py in 0..tile_size {
                    for px in 0..tile_size {
                        let pixel = atlas.get_pixel(x + px, y + py);
                        tile.put_pixel(px, py, *pixel);
                    }
                }

                // Save to temporary file
                let temp_dir = std::env::temp_dir().join("mclre_items_extract");
                fs::create_dir_all(&temp_dir).ok();
                let temp_path = temp_dir.join(format!("{}.png", name));
                tile.save(&temp_path)
                    .context("Failed to save extracted item texture")?;

                let id_str = format!("minecraft:item/{}", name);
                if let Ok(id) = Identifier::parse(&id_str) {
                    textures.push((id, temp_path));
                }
            }
        }
    }

    println!(
        "  Extracted {} mapped textures from items atlas",
        textures.len()
    );
    Ok(textures)
}

// ========================================================================
// LCE Language File Scanning
// ========================================================================

fn scan_lce_language_files(vfs: &AssetVfs, source: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut lang_files = Vec::new();

    // LCE language file locations
    let lang_dirs = [
        "Minecraft.Client/Common/res/texts",
        "Minecraft.Client/Common/res/lang",
    ];

    for dir in lang_dirs {
        if let Ok(files) = vfs.list_dir(dir) {
            for file in files {
                if file.to_lowercase().ends_with(".lang") || file.to_lowercase().ends_with(".txt") {
                    let full_path = format!("{}/{}", dir, file);
                    let lang_code = Path::new(&file)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("en_us")
                        .to_string();

                    let temp_path = source.join(&full_path);
                    lang_files.push((lang_code, temp_path));
                }
            }
        }
    }

    Ok(lang_files)
}

// ========================================================================
// DLC Pack Extraction
// ========================================================================

fn extract_dlc_packs(dlc_source: &Path, output: &Path, verbose: bool) -> Result<()> {
    println!("Extracting DLC packs from {:?}", dlc_source);

    let dlc_dirs = std::fs::read_dir(dlc_source)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .collect::<Vec<_>>();

    for dlc in dlc_dirs {
        let dlc_name = dlc.file_name().to_string_lossy().to_string();
        let data_dir = dlc.path().join("Data");

        if !data_dir.exists() {
            continue;
        }

        println!("  Processing DLC: {}", dlc_name);

        // Check for TexturePack.pck (HD textures)
        let texture_pack = data_dir.join("TexturePack.pck");
        if texture_pack.exists() {
            let dlc_output = output.join("dlc").join(&dlc_name);
            std::fs::create_dir_all(&dlc_output)?;
            println!("    Found TexturePack.pck, extracting to {:?}", dlc_output);
            // Copy for now, full extraction would need PCK parser
            std::fs::copy(&texture_pack, dlc_output.join("TexturePack.pck"))?;
        }

        // Check for x16Data.pck / x32Data.pck
        for pck_name in ["x16Data.pck", "x32Data.pck"] {
            let pck_file = data_dir.join(pck_name);
            if pck_file.exists() {
                let dlc_output = output.join("dlc").join(&dlc_name);
                std::fs::create_dir_all(&dlc_output)?;
                println!("    Found {}, extracting to {:?}", pck_name, dlc_output);
                std::fs::copy(&pck_file, dlc_output.join(pck_name))?;
            }
        }
    }

    Ok(())
}

// ========================================================================
// Texture Conversion
// ========================================================================

fn convert_texture(
    asset_manager: &mut AssetManager,
    id: &Identifier,
    path: &Path,
    source_root: &Path,
    _format: TextureFormat,
    _mipmaps: bool,
) -> Result<()> {
    let img = image::open(path)
        .with_context(|| format!("Failed to load image: {:?}", path))?
        .to_rgba8();

    let texture = Texture::from_image(img);

    // Compute relative path from source root for VFS lookup
    let relative_path = path
        .strip_prefix(source_root)
        .unwrap_or(path)
        .to_str()
        .unwrap();

    asset_manager.load_into_atlas(*id, relative_path)?;

    Ok(())
}

// ========================================================================
// Language File Conversion
// ========================================================================

fn convert_language_file(
    asset_manager: &mut AssetManager,
    lang_code: &str,
    path: &Path,
) -> Result<()> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read language file: {:?}", path))?;

    let mut translations = HashMap::new();
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            translations.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    // Write as Java JSON format to output/lang/
    let output_dir = Path::new("game-assets/lang");
    std::fs::create_dir_all(output_dir)?;

    let output_path = output_dir.join(format!("{}.json", lang_code));
    let json = serde_json::to_string_pretty(&translations)?;
    std::fs::write(&output_path, json)?;

    println!(
        "  Converted {} entries to {:?}",
        translations.len(),
        output_path
    );
    Ok(())
}

// ========================================================================
// Atlas Building
// ========================================================================

fn build_atlas(asset_manager: &mut AssetManager) -> Result<()> {
    let atlas_texture = asset_manager.atlas_texture();
    println!(
        "  Atlas size: {}x{}",
        atlas_texture.width, atlas_texture.height
    );
    println!("  Atlas regions: {}", asset_manager.atlas.regions.len());
    Ok(())
}

// ========================================================================
// Texture Atlas Saving
// ========================================================================

fn save_texture_atlas(asset_manager: &AssetManager, output: &Path) -> Result<()> {
    let atlas_texture = asset_manager.atlas_texture();

    // Create the atlas image from raw RGBA data
    let img = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(
        atlas_texture.width,
        atlas_texture.height,
        atlas_texture.data.clone(),
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer from atlas data"))?;

    // Save atlas image
    let atlas_path = output.join("textures/block/atlas.png");
    std::fs::create_dir_all(atlas_path.parent().unwrap())?;
    img.save(&atlas_path)?;
    println!("  Saved atlas to {:?}", atlas_path);

    Ok(())
}

/// Copy individual texture files to output directory for direct loading
fn copy_texture_files(
    vfs: &AssetVfs,
    source: &Path,
    output: &Path,
    texture_files: &[(Identifier, PathBuf)],
    verbose: bool,
) -> Result<()> {
    let mut copied = 0;
    for (id, path) in texture_files {
        // Determine output path based on identifier
        let rel_path = if let Some(id_str) = id.to_string().strip_prefix("minecraft:") {
            // Convert minecraft:block/name -> textures/block/name.png
            // or minecraft:item/name -> textures/item/name.png
            let parts: Vec<&str> = id_str.split('/').collect();
            if parts.len() >= 2 {
                format!("textures/{}/{}.png", parts[0], parts[1..].join("/"))
            } else {
                format!("textures/misc/{}.png", id_str)
            }
        } else {
            format!("textures/misc/{}.png", id)
        };

        let output_path = output.join(&rel_path);
        std::fs::create_dir_all(output_path.parent().unwrap())?;

        // Copy the file
        std::fs::copy(path, &output_path)?;
        copied += 1;

        if verbose {
            println!("  Copied {} -> {}", path.display(), rel_path);
        }
    }

    println!("  Copied {} texture files", copied);
    Ok(())
}

fn write_pack_mcmeta(output: &Path, pack_format: i32, description: &str) -> Result<()> {
    #[derive(Serialize)]
    struct PackMeta {
        pack: PackInfo,
    }

    #[derive(Serialize)]
    struct PackInfo {
        pack_format: i32,
        description: String,
    }

    let meta = PackMeta {
        pack: PackInfo {
            pack_format,
            description: description.to_string(),
        },
    };

    let json = serde_json::to_string_pretty(&meta)?;
    std::fs::write(output.join("pack.mcmeta"), json)?;
    Ok(())
}

fn write_pack_toml(output: &Path) -> Result<()> {
    let toml = r#"# MCLRE Resource Pack Manifest
# This file is generated by assetc

name = "MCLRE Vanilla"
version = "1.0.0"
description = "Converted LCE assets for MCLRE"
pack_format = 15
"#;
    std::fs::write(output.join("pack.toml"), toml)?;
    Ok(())
}

fn save_manifest(output: &Path, asset_manager: &AssetManager) -> Result<()> {
    #[derive(Serialize)]
    struct Manifest {
        textures: HashMap<String, ManifestRegion>,
        generated_at: String,
    }

    #[derive(Serialize)]
    struct ManifestRegion {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        u_min: f32,
        v_min: f32,
        u_max: f32,
        v_max: f32,
    }

    let mut textures = HashMap::new();
    for (id, region) in &asset_manager.atlas.regions {
        textures.insert(
            id.to_string(),
            ManifestRegion {
                x: region.x,
                y: region.y,
                width: region.width,
                height: region.height,
                u_min: region.u_min,
                v_min: region.v_min,
                u_max: region.u_max,
                v_max: region.v_max,
            },
        );
    }

    let manifest = Manifest {
        textures,
        generated_at: chrono::Utc::now().to_rfc3339(),
    };

    let json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(output.join("manifest.json"), json)?;
    Ok(())
}

// ========================================================================
// Pack Command
// ========================================================================

fn pack_command(
    input: PathBuf,
    output: PathBuf,
    name: String,
    description: String,
    pack_format: i32,
) -> Result<()> {
    println!("Creating resource pack from {:?} to {:?}", input, output);
    println!("Pack: {} (format {})", name, pack_format);

    copy_dir(&input, &output)?;
    write_pack_mcmeta(&output, pack_format, &description)?;

    println!("Pack created at {:?}", output);
    Ok(())
}

fn atlas_command(input: PathBuf, output: PathBuf, size: u32, padding: u32) -> Result<()> {
    println!("Building texture atlas from {:?} to {:?}", input, output);
    println!("Size: {}x{}, padding: {}", size, size, padding);

    let mut atlas = TextureAtlas::new(size, size);
    let mut vfs = AssetVfs::new();
    vfs.add_root(input.clone());

    let files = vfs.list_dir("")?;
    for file in files {
        if file.to_lowercase().ends_with(".png") {
            let path = input.join(&file);
            let img = image::open(&path)?.to_rgba8();
            let texture = Texture::from_image(img);

            let name = Path::new(&file)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            let id = Identifier::parse(&format!("minecraft:atlas/{}", name))?;
            atlas.add_region(id, texture)?;
        }
    }

    let atlas_texture = atlas.texture();
    let img = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(
        atlas_texture.width,
        atlas_texture.height,
        atlas_texture.data.clone(),
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?;

    img.save(&output)?;

    let mut uvs = HashMap::new();
    for (id, region) in &atlas.regions {
        uvs.insert(
            id.to_string(),
            [region.u_min, region.v_min, region.u_max, region.v_max],
        );
    }

    let uv_path = output.with_extension("uv.json");
    std::fs::write(&uv_path, serde_json::to_string_pretty(&uvs)?)?;

    println!(
        "Atlas saved to {:?} ({} regions)",
        output,
        atlas.regions.len()
    );
    Ok(())
}

fn validate_command(path: PathBuf) -> Result<()> {
    println!("Validating asset directory: {:?}", path);

    let mut vfs = AssetVfs::new();
    vfs.add_root(path.clone());

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    let required_dirs = [
        "textures/block",
        "textures/item",
        "models/block",
        "models/item",
        "lang",
    ];

    for dir in required_dirs {
        if !vfs.exists(dir) {
            warnings.push(format!("Missing directory: {}", dir));
        }
    }

    if !vfs.exists("pack.mcmeta") {
        errors.push("Missing pack.mcmeta".to_string());
    }

    if let Ok(files) = vfs.list_dir("lang") {
        if files.is_empty() {
            warnings.push("No language files found".to_string());
        }
    }

    if errors.is_empty() && warnings.is_empty() {
        println!("âœ“ Validation passed");
    } else {
        for w in warnings {
            println!("âš  Warning: {}", w);
        }
        for e in errors {
            println!("âœ— Error: {}", e);
        }
    }

    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let rel_path = entry.path().strip_prefix(src)?;
        let dst_path = dst.join(rel_path);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(dst_path)?;
        } else {
            std::fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

trait TextureAtlasExt {
    fn add_region(&mut self, id: Identifier, texture: Texture) -> Result<()>;
}

impl TextureAtlasExt for TextureAtlas {
    fn add_region(&mut self, id: Identifier, texture: Texture) -> Result<()> {
        let region = self.add_texture(id, &texture)?;
        println!(
            "  Added to atlas: {} at ({}, {}) {}x{}",
            id, region.x, region.y, region.width, region.height
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_texture_format_parsing() {
        assert!(matches!("png".parse(), Ok(TextureFormat::Png)));
        assert!(matches!("ktx2".parse(), Ok(TextureFormat::Ktx2)));
        assert!("jpg".parse::<TextureFormat>().is_err());
    }
}
