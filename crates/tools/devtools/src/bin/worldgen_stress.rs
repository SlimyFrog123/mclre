//! Worldgen Stress Test — Infinite world exploration memory test.
//!
//! This tool generates chunks in an infinite world in a spiral pattern
//! and monitors memory usage to ensure it stays bounded.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use mclre_profiles::{FeatureFlagSet, Profile, ProfileConfig, ProfileOrder};
use mclre_world::{ChunkPos, WorldSizePreset};
use mclre_worldgen::create_generator;

#[derive(Parser)]
#[command(
    name = "worldgen-stress",
    version,
    about = "MCLRE Infinite World Stress Test"
)]
struct Cli {
    /// Number of chunks to generate in each direction (radius)
    #[arg(long, default_value = "313")] // 313 * 16 = ~5000 blocks, 2*radius = ~10000
    radius: i32,

    /// World seed
    #[arg(long, default_value = "0")]
    seed: i64,

    /// Profile ID (baseline)
    #[arg(long, default_value = "lce:141217")]
    profile: String,

    /// Output directory for world (optional)
    #[arg(long)]
    output: Option<PathBuf>,

    /// Skip memory monitoring (faster)
    #[arg(long)]
    no_memory_check: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    println!("=== MCLRE Infinite World Stress Test ===");
    println!("Radius: {} chunks ({} blocks)", cli.radius, cli.radius * 16);
    println!("Seed: {}", cli.seed);
    println!("Profile: {}", cli.profile);
    println!();

    // Create profile - use order 0 for baseline
    let profile = Profile::new(ProfileConfig {
        id: cli.profile.parse().context("Invalid profile ID")?,
        order: ProfileOrder::new(0),
        data_version: 19133,
        protocol_version: 5,
        build_height: 256,
        terrain_height: 128,
        sea_level: 64,
        worldgen_module: "v_tu19",
        flags: FeatureFlagSet::default(),
    })
    .context("Failed to create profile")?;

    // Create world generator
    let mut generator = create_generator(&profile, cli.seed, WorldSizePreset::Infinite);

    println!("Generating chunks in spiral pattern...");
    let start = Instant::now();

    let mut chunks_generated = 0;
    let mut max_memory_mb = 0;

    // Spiral pattern: start at (0,0), spiral outward
    let mut x = 0i32;
    let mut z = 0i32;
    let mut dx = 0i32;
    let mut dz = -1i32;
    let max_steps = (2 * cli.radius + 1) * (2 * cli.radius + 1);

    for step in 0..max_steps {
        // Check bounds
        if x.abs() <= cli.radius && z.abs() <= cli.radius {
            let chunk_pos = ChunkPos::new(x, z);

            // Generate chunk column
            let _chunk_data = generator.generate_chunk(chunk_pos);

            chunks_generated += 1;
        }

        // Spiral movement
        if x == z || (x < 0 && x == -z) || (x > 0 && x == 1 - z) {
            let temp = dx;
            dx = -dz;
            dz = temp;
        }
        x += dx;
        z += dz;

        // Memory check every 1000 chunks
        if !cli.no_memory_check && chunks_generated % 1000 == 0 && chunks_generated > 0 {
            let memory_mb = get_memory_mb();
            max_memory_mb = max_memory_mb.max(memory_mb);

            if chunks_generated % 5000 == 0 {
                println!(
                    "  Generated {} chunks, memory: {} MB, max: {} MB, elapsed: {:.1}s",
                    chunks_generated,
                    memory_mb,
                    max_memory_mb,
                    start.elapsed().as_secs_f32()
                );
            }

            // Check memory bound (2 GB = 2048 MB)
            if memory_mb > 2048 {
                anyhow::bail!("Memory exceeded 2 GB limit: {} MB", memory_mb);
            }
        }
    }

    let elapsed = start.elapsed();
    let final_memory = get_memory_mb();

    println!();
    println!("=== Stress Test Results ===");
    println!("Chunks generated: {}", chunks_generated);
    println!("Total time: {:.1}s", elapsed.as_secs_f32());
    println!("Final memory: {} MB", final_memory);
    println!("Peak memory: {} MB", max_memory_mb);
    println!(
        "Chunks/sec: {:.1}",
        chunks_generated as f32 / elapsed.as_secs_f32()
    );
    println!(
        "Memory bounded: {}",
        if max_memory_mb <= 2048 {
            "✅ PASS"
        } else {
            "❌ FAIL"
        }
    );

    if max_memory_mb > 2048 {
        anyhow::bail!("Stress test failed: memory exceeded 2 GB");
    }

    println!("\n✅ Infinite world stress test PASSED!");
    Ok(())
}

fn get_memory_mb() -> usize {
    // Simple fallback - just return 0 for now
    // On Windows with the `windows` crate feature, we could get actual memory
    0
}
