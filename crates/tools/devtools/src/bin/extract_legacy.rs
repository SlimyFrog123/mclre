//! `extract-legacy`: parses the reference block table into `data/legacy/blocks.toml`.

use std::path::PathBuf;

use clap::Parser;
use mclre_devtools::legacy_blocks::{parse_tile_source, render_toml};

/// Extracts legacy block metadata from the reference `Tile.cpp`.
#[derive(Debug, Parser)]
#[command(name = "extract-legacy", about, version)]
struct Args {
    /// Path to the reference `Minecraft.World/Tile.cpp`.
    #[arg(
        long,
        default_value = "reference/lce-141217/MinecraftConsoles-141217/Minecraft.World/Tile.cpp"
    )]
    source: PathBuf,

    /// Output TOML path.
    #[arg(long, default_value = "data/legacy/blocks.toml")]
    out: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let source = std::fs::read_to_string(&args.source)
        .map_err(|error| anyhow::anyhow!("cannot read {}: {error}", args.source.display()))?;
    let blocks = parse_tile_source(&source).map_err(anyhow::Error::msg)?;
    if blocks.is_empty() {
        anyhow::bail!(
            "no block registrations found in {} (wrong file or reference layout changed)",
            args.source.display()
        );
    }

    // Label the source relative to the repository root when possible, so the
    // generated file does not embed an absolute path.
    let label = args
        .source
        .strip_prefix(repo_root())
        .unwrap_or(&args.source)
        .display()
        .to_string()
        .replace('\\', "/");

    let toml = render_toml(&blocks, &label);
    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&args.out, toml)?;
    println!(
        "wrote {} legacy blocks to {}",
        blocks.len(),
        args.out.display()
    );
    Ok(())
}

/// Best-effort repository root (`CARGO_MANIFEST_DIR` is `crates/tools/devtools`).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}
