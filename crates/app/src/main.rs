//! mclre: client executable. Window loop, glue; the only crate wiring client + integrated server.

use clap::Parser;
use mclre_client::{Client, ClientConfig};
use mclre_core::Vec3;
use mclre_identity::PlayerIdentity;
use mclre_net::{InMemoryTransport, Transport};
use mclre_server::IntegratedServer;
use mclre_world::WorldSizePreset;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

mod asset_import;
mod crash_reporting;

/// Command line arguments for MCLRE
#[derive(Parser)]
#[command(name = "mclre", version, about = "Minecraft: Legacy Rust Edition")]
struct Args {
    /// Window width
    #[arg(long, default_value = "1280")]
    width: u32,

    /// Window height
    #[arg(long, default_value = "720")]
    height: u32,

    /// World size preset
    #[arg(long, default_value = "medium")]
    world_size: WorldSizePreset,

    /// Number of split-screen players
    #[arg(long, default_value = "1")]
    split_screen: usize,

    /// Disable VSync (default: enabled)
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    vsync: bool,

    /// Game assets directory
    #[arg(long, default_value = "game-assets")]
    assets_dir: std::path::PathBuf,

    /// LCE asset source directory (for first-run import)
    #[arg(long)]
    lce_source: Option<std::path::PathBuf>,

    /// Skip asset import check
    #[arg(long)]
    skip_asset_check: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Auto-import assets if missing (non-interactive)
    #[arg(long)]
    auto_import: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize crash reporting FIRST, before anything else
    crash_reporting::init_crash_reporting(None);

    let args = Args::parse();

    println!("Starting MCLRE client...");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Handle asset import before starting client
    if !args.skip_asset_check {
        let status = asset_import::check_asset_status(&args.assets_dir);

        match status {
            asset_import::AssetImportStatus::Complete => {
                println!("✅ Game assets found at: {}", args.assets_dir.display());
            }
            asset_import::AssetImportStatus::Missing
            | asset_import::AssetImportStatus::Incomplete => {
                println!(
                    "⚠️  Game assets missing or incomplete: {}",
                    args.assets_dir.display()
                );

                if args.auto_import {
                    println!("Running auto-import...");
                    asset_import::run_auto_import(&args.assets_dir, args.lce_source, args.verbose)?;
                } else {
                    println!("Run with --auto-import to automatically convert assets,");
                    println!("or provide --lce-source to specify the LCE asset directory.");
                    println!("Alternatively, run assetc manually:");
                    println!(
                        "  cargo run --bin assetc -- convert --source <LCE_SOURCE> --output {}",
                        args.assets_dir.display()
                    );
                    return Err("Game assets required".into());
                }
            }
            asset_import::AssetImportStatus::Importing => {
                println!("Assets are being imported, please wait...");
            }
        }
    }

    let config = ClientConfig {
        title: "MCLRE".to_string(),
        width: args.width,
        height: args.height,
        target_frame_time: Duration::from_millis(50),
        world_preset: args.world_size,
        split_screen_players: args.split_screen,
        spawn_pos: Vec3::new(0.0, 65.62, 0.0),
        enable_vsync: args.vsync,
    };

    let active_players = config.split_screen_players;
    let mut client = Client::new(config)?;
    println!("Client created successfully, ready for main menu...");

    // Create integrated server but DON'T start it yet - wait for "Play Game"
    let server_config = mclre_server::ServerConfig::default();
    let integrated_server = IntegratedServer::new(server_config);
    let server_arc = Arc::new(Mutex::new(integrated_server));

    // Create InMemoryTransport pairs for each player
    let mut client_transports = Vec::with_capacity(active_players);
    let mut entity_ids = Vec::with_capacity(active_players);

    for player_idx in 0..active_players {
        let (client_transport, server_transport) = InMemoryTransport::pair();

        let identity = PlayerIdentity::new(Uuid::now_v7(), format!("Player{}", player_idx + 1));

        // Add client to server (but don't start server yet)
        let entity_id = {
            let mut server = server_arc.lock();
            server.add_client(Box::new(server_transport), identity)?
        };

        client_transports.push(Arc::new(client_transport) as Arc<dyn Transport + Send + Sync>);
        entity_ids.push(entity_id);

        println!(
            "Player {} registered with integrated server (entity_id {})",
            player_idx, entity_id
        );
    }

    // Set up singleplayer transports on the client
    let network_transports = client.network_transports_mut();
    for (i, transport) in client_transports.into_iter().enumerate() {
        if i < network_transports.len() {
            network_transports[i] = Some(transport);
        }
    }
    println!("Singleplayer transports configured, starting main menu...");

    // Give client access to server for deferred startup
    client.set_server(server_arc.clone());

    // Wrap run in error handling to generate error reports
    if let Err(e) = client.run() {
        let report_path = crash_reporting::generate_error_report(&e, "Client run failed");
        eprintln!("Error report generated: {}", report_path.display());
        return Err(e.into());
    }

    Ok(())
}
