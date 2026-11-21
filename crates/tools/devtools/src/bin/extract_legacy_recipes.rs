//! Binary to extract recipes from the LCE reference source.

use anyhow::{Context, Result};
use clap::Parser;
use mclre_devtools::legacy_recipes::parse_all_recipe_files;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "extract-legacy-recipes", about = "Extract recipes from LCE reference")]
struct Args {
    /// Path to the reference LCE source directory (Minecraft.World)
    #[arg(long, default_value = "../../../reference/lce-141217/MinecraftConsoles-141217/Minecraft.World")]
    reference_dir: PathBuf,

    /// Output directory for generated TOML files
    #[arg(long, default_value = "../../../data/legacy")]
    output_dir: PathBuf,

    /// Only parse specific recipe file (for testing)
    #[arg(long)]
    file: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("Extracting recipes from: {:?}", args.reference_dir);
    println!("Output directory: {:?}", args.output_dir);

    // Read all recipe files
    let recipes_cpp = read_file(&args.reference_dir.join("Recipes.cpp"))?;
    let tool_recipes_cpp = read_file(&args.reference_dir.join("ToolRecipies.cpp"))?;
    let weapon_recipes_cpp = read_file(&args.reference_dir.join("WeaponRecipies.cpp"))?;
    let armor_recipes_cpp = read_file(&args.reference_dir.join("ArmorRecipes.cpp"))?;
    let food_recipes_cpp = read_file(&args.reference_dir.join("FoodRecipies.cpp"))?;
    let structure_recipes_cpp = read_file(&args.reference_dir.join("StructureRecipies.cpp"))?;
    let ore_recipes_cpp = read_file(&args.reference_dir.join("OreRecipies.cpp"))?;
    let cloth_dye_recipes_cpp = read_file(&args.reference_dir.join("ClothDyeRecipes.cpp"))?;

    // Parse all recipes
    let recipes = parse_all_recipe_files(
        &recipes_cpp,
        &tool_recipes_cpp,
        &weapon_recipes_cpp,
        &armor_recipes_cpp,
        &food_recipes_cpp,
        &structure_recipes_cpp,
        &ore_recipes_cpp,
        &cloth_dye_recipes_cpp,
    ).map_err(|e| anyhow::anyhow!(e)).context("Failed to parse recipes")?;

    println!("Parsed {} recipes", recipes.len());

    // Create output directory
    fs::create_dir_all(&args.output_dir).context("Failed to create output directory")?;

    // Write JSON for tooling/parity tests
    let json_path = args.output_dir.join("recipes.json");
    let json = serde_json::to_string_pretty(&recipes).context("Failed to serialize JSON")?;
    fs::write(&json_path, json).context("Failed to write recipes.json")?;
    println!("Written to: {:?}", json_path);

    Ok(())
}

fn read_file(path: &PathBuf) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("Failed to read {:?}", path))
}