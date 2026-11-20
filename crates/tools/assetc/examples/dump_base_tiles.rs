use image::{GenericImageView, ImageBuffer, Rgba};
use std::fs;

fn main() {
    let path =
        "reference/lce-141217/MinecraftConsoles-141217/Minecraft.Client/Common/res/terrain.png";

    let img = image::open(path).expect("Failed to open");
    let rgba: ImageBuffer<Rgba<u8>, Vec<u8>> = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    println!("Image: {}x{}", width, height);

    let tile_size = 16;
    let tiles_x = width / tile_size;
    let tiles_y = height / tile_size;

    println!("Tiles: {}x{} = {}", tiles_x, tiles_y, tiles_x * tiles_y);

    // Create output directory
    fs::create_dir_all("debug_base_tiles").ok();

    // Dump each tile as a separate image
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let x = tx * tile_size;
            let y = ty * tile_size;

            let mut tile: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(tile_size, tile_size);
            for py in 0..tile_size {
                for px in 0..tile_size {
                    if x + px < width && y + py < height {
                        let pixel = rgba.get_pixel(x + px, y + py);
                        tile.put_pixel(px, py, *pixel);
                    }
                }
            }

            let index = ty * tiles_x + tx;
            let filename = format!("debug_base_tiles/tile_{:04}_{}x{}.png", index, tx, ty);
            tile.save(&filename).ok();

            // Print first few pixels of each tile
            let p = tile.get_pixel(0, 0);
            let p2 = tile.get_pixel(8, 8);
            println!(
                "Tile {:04} ({},{}): corner={:?} center={:?}",
                index, tx, ty, p, p2
            );
        }
    }

    println!("Done! Check debug_base_tiles/ folder");
}
