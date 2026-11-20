use image::GenericImageView;

fn main() {
    let paths = [
        "reference/lce-141217/MinecraftConsoles-141217/Minecraft.Client/Common/res/terrain.png",
        "reference/lce-141217/MinecraftConsoles-141217/Minecraft.Client/Common/res/TitleUpdate/res/terrain.png",
    ];

    for path in paths {
        match image::open(path) {
            Ok(img) => {
                println!("{}: {}x{}", path, img.width(), img.height());
                let rgba = img.to_rgba8();
                // Check first few pixels
                for y in 0..4.min(rgba.height()) {
                    for x in 0..4.min(rgba.width()) {
                        let p = rgba.get_pixel(x, y);
                        print!("[{:3},{:3},{:3},{:3}] ", p[0], p[1], p[2], p[3]);
                    }
                    println!();
                }
                println!("---");
            }
            Err(e) => {
                println!("{}: ERROR - {}", path, e);
            }
        }
    }
}
