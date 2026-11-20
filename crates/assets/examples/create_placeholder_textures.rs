//! Creates simple placeholder textures for testing.

use image::{ImageBuffer, Rgba};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = Path::new("assets-placeholder/textures/block");
    std::fs::create_dir_all(output_dir)?;

    // Stone - gray with some noise
    create_stone_texture(output_dir.join("stone.png"))?;

    // Dirt - brown
    create_solid_texture(output_dir.join("dirt.png"), [101, 67, 33, 255])?;

    // Grass block top - green
    create_solid_texture(output_dir.join("grass_block_top.png"), [64, 160, 32, 255])?;

    // Bedrock - dark gray
    create_solid_texture(output_dir.join("bedrock.png"), [50, 50, 50, 255])?;

    println!("Created placeholder textures in {:?}", output_dir);
    Ok(())
}

fn create_solid_texture(
    path: std::path::PathBuf,
    color: [u8; 4],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(16, 16);
    for pixel in img.pixels_mut() {
        *pixel = Rgba(color);
    }
    img.save(path)?;
    Ok(())
}

fn create_stone_texture(path: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(16, 16);
    let base_color = [128, 128, 128, 255];

    // Add some noise pattern
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    for y in 0..16 {
        for x in 0..16 {
            let mut hasher = DefaultHasher::new();
            (x * 100 + y).hash(&mut hasher);
            let hash = hasher.finish();

            let variation = ((hash % 40) as i32) - 20; // -20 to +20
            let r = (base_color[0] as i32 + variation).clamp(0, 255) as u8;
            let g = (base_color[1] as i32 + variation).clamp(0, 255) as u8;
            let b = (base_color[2] as i32 + variation).clamp(0, 255) as u8;

            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
    img.save(path)?;
    Ok(())
}
