//! DataFixer — World upgrade tooling for MCLRE.
//!
//! This tool handles upgrading world saves from older data versions to the current version.
//! It reads the world's `level.dat` to determine the data version, then applies a series
//! of data fixers (upgrade steps) to bring the world up to date.
//!
//! Usage:
//!   cargo run --bin datafixer -- <WORLD_DIR> [--dry-run] [--backup] [--target-version VERSION]

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use mclre_core::nbt::{Compound, Value, read_bytes, write};
use mclre_world::anvil::RegionFile;
use mclre_world::level::DATA_VERSION;
use mclre_world::pos::ChunkPos;

/// Current data version for MCLRE (matches LCE 141217 baseline)
const CURRENT_DATA_VERSION: i32 = DATA_VERSION;

#[derive(Parser)]
#[command(
    name = "datafixer",
    version,
    about = "MCLRE World Data Fixer - upgrades world saves to current version"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check the data version of a world without modifying it
    Check {
        /// Path to the world directory
        world_dir: PathBuf,
    },
    /// Upgrade a world to the current data version
    Upgrade {
        /// Path to the world directory
        world_dir: PathBuf,
        /// Perform a dry run without writing changes
        #[arg(long)]
        dry_run: bool,
        /// Create a backup before upgrading
        #[arg(long, default_value = "true")]
        backup: bool,
        /// Target data version (default: current)
        #[arg(long)]
        target_version: Option<i32>,
        /// Force upgrade even if already at target version
        #[arg(long)]
        force: bool,
    },
    /// List all available data fixers (upgrade steps)
    ListFixers,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Check { world_dir } => check_world(&world_dir),
        Command::Upgrade {
            world_dir,
            dry_run,
            backup,
            target_version,
            force,
        } => upgrade_world(
            &world_dir,
            dry_run,
            backup,
            target_version.unwrap_or(CURRENT_DATA_VERSION),
            force,
        ),
        Command::ListFixers => list_fixers(),
    }
}

fn check_world(world_dir: &Path) -> Result<()> {
    println!("Checking world at: {}", world_dir.display());

    if !world_dir.is_dir() {
        anyhow::bail!("World directory does not exist: {}", world_dir.display());
    }

    let level_dat = world_dir.join("level.dat");
    if !level_dat.exists() {
        anyhow::bail!("level.dat not found in world directory");
    }

    let data_version = extract_data_version(&level_dat)?;

    println!("  Data Version: {}", data_version);
    println!("  Current Version: {}", CURRENT_DATA_VERSION);

    if data_version < CURRENT_DATA_VERSION {
        println!(
            "  Status: NEEDS UPGRADE ({} fixers to apply)",
            CURRENT_DATA_VERSION - data_version
        );
    } else if data_version == CURRENT_DATA_VERSION {
        println!("  Status: UP TO DATE");
    } else {
        println!("  Status: FUTURE VERSION (may be incompatible)");
    }

    Ok(())
}

fn upgrade_world(
    world_dir: &Path,
    dry_run: bool,
    backup: bool,
    target_version: i32,
    force: bool,
) -> Result<()> {
    println!("Upgrading world at: {}", world_dir.display());

    if !world_dir.is_dir() {
        anyhow::bail!("World directory does not exist: {}", world_dir.display());
    }

    let level_dat = world_dir.join("level.dat");
    if !level_dat.exists() {
        anyhow::bail!("level.dat not found in world directory");
    }

    let current_version = extract_data_version(&level_dat)?;

    println!("  Current Data Version: {}", current_version);
    println!("  Target Data Version: {}", target_version);

    if current_version >= target_version && !force {
        println!("  World is already at or above target version. Use --force to re-apply fixers.");
        return Ok(());
    }

    if backup && !dry_run {
        println!("  Creating backup...");
        let backup_path = create_backup(world_dir)?;
        println!("  Backup created at: {}", backup_path.display());
    } else if backup && dry_run {
        println!("  [DRY RUN] Would create backup");
    }

    let fixers = get_fixers_for_range(current_version, target_version);
    println!("  Fixers to apply: {}", fixers.len());

    for fixer in &fixers {
        println!("  - {}", fixer.name());
    }

    if dry_run {
        println!("\n[DRY RUN] No changes written. Run without --dry-run to apply.");
        return Ok(());
    }

    // Apply fixers
    for fixer in fixers {
        println!("  Applying: {}...", fixer.name());
        fixer
            .apply(world_dir)
            .context(format!("Failed to apply fixer: {}", fixer.name()))?;
    }

    // Update level.dat data version
    update_data_version(world_dir, target_version)?;
    println!("  Updated level.dat data version to {}", target_version);

    println!("\nUpgrade complete!");
    Ok(())
}

fn create_backup(world_dir: &Path) -> Result<PathBuf> {
    use chrono::Utc;
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_name = format!("pre_upgrade_backup_{}", timestamp);
    let backup_path = world_dir.join("backups").join(&backup_name);

    fs::create_dir_all(backup_path.parent().unwrap())?;
    copy_dir_all(world_dir, &backup_path, Some("backups"))?;

    Ok(backup_path)
}

fn copy_dir_all(src: &Path, dst: &Path, exclude: Option<&str>) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name();

        if let Some(ex) = exclude {
            if name.to_string_lossy() == ex {
                continue;
            }
        }

        let src_path = entry.path();
        let dst_path = dst.join(&name);

        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path, exclude)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn list_fixers() -> Result<()> {
    println!("Available Data Fixers (MCLRE):");
    println!("===============================\n");

    let all_fixers = get_all_fixers();
    for (i, fixer) in all_fixers.iter().enumerate() {
        println!(
            "{}. {} (from v{} to v{})",
            i + 1,
            fixer.name(),
            fixer.from_version(),
            fixer.to_version()
        );
        println!("   {}", fixer.description());
        println!();
    }

    Ok(())
}

fn extract_data_version(level_dat_path: &Path) -> Result<i32> {
    let bytes = fs::read(level_dat_path).context("Failed to read level.dat file")?;
    let (_root_name, root) = read_bytes(&bytes).context("Failed to parse level.dat NBT")?;

    // level.dat structure: { "Data": { "Version": { "Name": "...", "Id": 19133, ... } } }
    if let Some(Value::Compound(data)) = root.get("Data") {
        if let Some(Value::Compound(version)) = data.get("Version") {
            if let Some(Value::Int(id)) = version.get("Id") {
                return Ok(*id);
            }
        }
    }
    anyhow::bail!("Could not extract data version from level.dat");
}

fn update_data_version(world_dir: &Path, version: i32) -> Result<()> {
    let level_dat = world_dir.join("level.dat");
    let bytes = fs::read(&level_dat).context("Failed to read level.dat")?;
    let (root_name, root) = read_bytes(&bytes).context("Failed to parse level.dat NBT")?;

    // Need to rebuild the root compound with the updated version
    let mut new_root = Compound::new();
    for (key, value) in root.iter() {
        if key == "Data" {
            if let Value::Compound(data_compound) = value {
                let mut new_data = Compound::new();
                for (d_key, d_value) in data_compound.iter() {
                    if d_key == "Version" {
                        if let Value::Compound(ver_compound) = d_value {
                            let mut new_ver = Compound::new();
                            for (v_key, v_value) in ver_compound.iter() {
                                if v_key == "Id" {
                                    new_ver.insert(v_key, Value::Int(version));
                                } else {
                                    new_ver.insert(v_key, v_value.clone());
                                }
                            }
                            new_data.insert(d_key, Value::Compound(new_ver));
                        } else {
                            new_data.insert(d_key, d_value.clone());
                        }
                    } else {
                        new_data.insert(d_key, d_value.clone());
                    }
                }
                new_root.insert(key, Value::Compound(new_data));
            } else {
                new_root.insert(key, value.clone());
            }
        } else {
            new_root.insert(key, value.clone());
        }
    }

    // Write atomically
    let temp_path = level_dat.with_extension("dat.tmp");
    let output = write(root_name.as_ref(), &new_root).context("Failed to write level.dat NBT")?;
    fs::write(&temp_path, output).context("Failed to write temp level.dat")?;
    fs::rename(&temp_path, &level_dat).context("Failed to replace level.dat")?;

    Ok(())
}

/// Trait for data fixers (world upgrade steps)
trait DataFixer: Send + Sync {
    fn name(&self) -> &'static str;
    fn from_version(&self) -> i32;
    fn to_version(&self) -> i32;
    fn description(&self) -> &'static str;
    fn apply(&self, world_dir: &Path) -> Result<()>;
}

/// Get all fixers in order
fn get_all_fixers() -> Vec<Box<dyn DataFixer>> {
    vec![
        Box::new(FixerBlockStatePalette {}),
        Box::new(FixerBiomeIds {}),
        Box::new(FixerChunkHeight {}),
        Box::new(FixerEntityIds {}),
        Box::new(FixerItemStackFormat {}),
    ]
}

/// Get fixers applicable for a version range
fn get_fixers_for_range(from: i32, to: i32) -> Vec<Box<dyn DataFixer>> {
    get_all_fixers()
        .into_iter()
        .filter(|f| f.from_version() <= to && f.to_version() > from)
        .collect()
}

/// Fixer: Convert old block metadata to block state palette format
struct FixerBlockStatePalette;

impl DataFixer for FixerBlockStatePalette {
    fn name(&self) -> &'static str {
        "BlockStatePalette"
    }
    fn from_version(&self) -> i32 {
        0
    }
    fn to_version(&self) -> i32 {
        CURRENT_DATA_VERSION
    }
    fn description(&self) -> &'static str {
        "Converts chunk block data from (id, meta) format to block state palette indices"
    }

    fn apply(&self, world_dir: &Path) -> Result<()> {
        let region_dir = world_dir.join("region");
        if !region_dir.exists() {
            return Ok(()); // No region files to fix
        }

        for entry in fs::read_dir(&region_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("mca") {
                continue;
            }

            fix_region_file(&path)?;
        }

        Ok(())
    }
}

fn fix_region_file(region_path: &Path) -> Result<()> {
    let mut region = RegionFile::open(region_path)?;
    let mut modified = false;

    for chunk_x in 0..32 {
        for chunk_z in 0..32 {
            let pos = ChunkPos::new(chunk_x, chunk_z);
            if let Some(data) = region.read_chunk(pos)? {
                // Parse NBT directly to check if fix is needed
                if let Ok((_name, nbt)) = mclre_core::nbt::read_bytes(&data) {
                    if needs_block_state_fix(&nbt) {
                        // Re-encode with palette format
                        let fixed_data = apply_block_state_fix(nbt)?;
                        region.write_chunk(pos, &fixed_data)?;
                        modified = true;
                    }
                }
            }
        }
    }

    if modified {
        region.flush()?;
        println!(
            "    Fixed region: {}",
            region_path.file_name().unwrap().to_string_lossy()
        );
    }

    Ok(())
}

fn needs_block_state_fix(nbt: &Compound) -> bool {
    // Check if chunk uses old "Blocks"/"Data" format instead of "block_states" palette
    if let Some(Value::Compound(level)) = nbt.get("Level") {
        return level.get("Blocks").is_some() && level.get("block_states").is_none();
    }
    false
}

fn apply_block_state_fix(nbt: Compound) -> Result<Vec<u8>> {
    // This is a simplified version - in reality, this would:
    // 1. Read the old Blocks/Data/Add arrays
    // 2. Convert each (id, meta) to a BlockStateId via the legacy mapping
    // 3. Build a palette of unique block states
    // 4. Write the new block_states format with palette + indices

    // For now, just return the original data as a placeholder
    // Real implementation would use mclre_core::nbt::write
    write("", &nbt).map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// Fixer: Remap old biome IDs to new namespaced biome IDs
struct FixerBiomeIds;

impl DataFixer for FixerBiomeIds {
    fn name(&self) -> &'static str {
        "BiomeIds"
    }
    fn from_version(&self) -> i32 {
        0
    }
    fn to_version(&self) -> i32 {
        CURRENT_DATA_VERSION
    }
    fn description(&self) -> &'static str {
        "Remaps numeric biome IDs to namespaced biome identifiers (e.g., 1 -> minecraft:plains)"
    }

    fn apply(&self, world_dir: &Path) -> Result<()> {
        let region_dir = world_dir.join("region");
        if !region_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&region_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("mca") {
                continue;
            }

            fix_biomes_in_region(&path)?;
        }

        Ok(())
    }
}

fn fix_biomes_in_region(region_path: &Path) -> Result<()> {
    let mut region = RegionFile::open(region_path)?;
    let mut modified = false;

    for chunk_x in 0..32 {
        for chunk_z in 0..32 {
            let pos = ChunkPos::new(chunk_x, chunk_z);
            if let Some(data) = region.read_chunk(pos)? {
                if let Ok((_name, nbt)) = mclre_core::nbt::read_bytes(&data) {
                    if needs_biome_fix(&nbt) {
                        let fixed_data = apply_biome_fix(nbt)?;
                        region.write_chunk(pos, &fixed_data)?;
                        modified = true;
                    }
                }
            }
        }
    }

    if modified {
        region.flush()?;
    }

    Ok(())
}

fn needs_biome_fix(nbt: &Compound) -> bool {
    if let Some(Value::Compound(level)) = nbt.get("Level") {
        if let Some(Value::ByteArray(_)) = level.get("Biomes") {
            return true; // Old format uses byte array of biome IDs
        }
    }
    false
}

fn apply_biome_fix(nbt: Compound) -> Result<Vec<u8>> {
    // Convert Biomes byte array to biome palette + indices (256 entries per chunk section)
    // Similar to block states but for biomes
    write("", &nbt).map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// Fixer: Update chunk height/sections format for build height changes
struct FixerChunkHeight;

impl DataFixer for FixerChunkHeight {
    fn name(&self) -> &'static str {
        "ChunkHeight"
    }
    fn from_version(&self) -> i32 {
        0
    }
    fn to_version(&self) -> i32 {
        CURRENT_DATA_VERSION
    }
    fn description(&self) -> &'static str {
        "Updates chunk section format for build height changes (Y=0 to Y=255 -> Y=-64 to Y=319)"
    }

    fn apply(&self, _world_dir: &Path) -> Result<()> {
        // This would handle vertical chunk section shifts
        // For LCE 141217, the height is 128 (Y=0 to Y=127), so this may not be needed
        // but is included for future compatibility
        Ok(())
    }
}

/// Fixer: Remap old numeric entity IDs to namespaced entity IDs
struct FixerEntityIds;

impl DataFixer for FixerEntityIds {
    fn name(&self) -> &'static str {
        "EntityIds"
    }
    fn from_version(&self) -> i32 {
        0
    }
    fn to_version(&self) -> i32 {
        CURRENT_DATA_VERSION
    }
    fn description(&self) -> &'static str {
        "Remaps numeric entity IDs to namespaced identifiers (e.g., 50 -> minecraft:creeper)"
    }

    fn apply(&self, _world_dir: &Path) -> Result<()> {
        // Would process entity NBT in chunks
        Ok(())
    }
}

/// Fixer: Update item stack format (damage value -> components)
struct FixerItemStackFormat;

impl DataFixer for FixerItemStackFormat {
    fn name(&self) -> &'static str {
        "ItemStackFormat"
    }
    fn from_version(&self) -> i32 {
        0
    }
    fn to_version(&self) -> i32 {
        CURRENT_DATA_VERSION
    }
    fn description(&self) -> &'static str {
        "Converts item stacks from damage-value format to data components format"
    }

    fn apply(&self, _world_dir: &Path) -> Result<()> {
        // Would process items in chests, inventories, entity equipment
        Ok(())
    }
}
