//! mclre: client executable. Window loop, glue; the only crate wiring client + integrated server.

use mclre_client::{Client, ClientConfig};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting MCLRE client...");

    let config = ClientConfig {
        title: "Minecraft: Legacy Rust Edition".to_string(),
        width: 1280,
        height: 720,
        target_frame_time: Duration::from_millis(50), // 20 TPS
        ..Default::default()
    };

    let client = Client::new(config)?;
    println!("Client created successfully, starting game loop...");

    client.run()?;

    Ok(())
}
