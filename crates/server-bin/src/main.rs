//! mclre-server: headless dedicated server executable (server-bin crate).
//!
//! Runs a headless Minecraft Legacy Console Edition server with LAN multiplayer support.
//! Supports UPnP port forwarding and mDNS service advertisement for LAN discovery.

use clap::Parser;
use mclre_server::{Server, ServerConfig};
use mclre_sim::GameMode;
use std::fs;
use std::path::PathBuf;
use tracing::{error, info, warn};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, FmtSubscriber, fmt::format::FmtSpan};

#[derive(Parser, Debug)]
#[command(name = "mclre-server", version, about = "MCLRE Dedicated Server", long_about = None)]
struct Args {
    /// World seed for generation
    #[arg(short, long, default_value = "0")]
    seed: i64,

    /// Maximum number of players
    #[arg(short = 'p', long, default_value = "20")]
    max_players: u32,

    /// Server message of the day
    #[arg(short = 'm', long, default_value = "MCLRE Server")]
    motd: String,

    /// Difficulty level
    #[arg(short, long, value_enum, default_value = "normal")]
    difficulty: DifficultyArg,

    /// Port for LAN multiplayer (0 = auto-assign)
    #[arg(short = 'P', long, default_value = "25565")]
    port: u16,

    /// Whether to advertise on LAN via mDNS
    #[arg(long, default_value = "true")]
    advertise_lan: bool,

    /// Disable mDNS advertisement
    #[arg(long, conflicts_with = "advertise_lan")]
    no_mdns: bool,

    /// Enable online mode (authenticate with auth servers)
    #[arg(long, default_value = "false")]
    online_mode: bool,

    /// View distance (chunks)
    #[arg(short, long, default_value = "10")]
    view_distance: u8,

    /// World name (used for save directory)
    #[arg(short, long, default_value = "world")]
    world_name: String,

    /// Log level
    #[arg(long, value_enum, default_value = "info")]
    log_level: LogLevelArg,

    /// Enable UPnP port forwarding
    #[arg(long, default_value = "true")]
    upnp: bool,

    /// Disable UPnP port forwarding
    #[arg(long, conflicts_with = "upnp")]
    no_upnp: bool,

    /// Tick rate (TPS)
    #[arg(long, default_value = "20")]
    tick_rate: u32,

    /// Run as daemon (headless server mode)
    #[arg(long, default_value = "false")]
    daemon: bool,

    /// Config file path (TOML)
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Generate default config file and exit
    #[arg(long)]
    generate_config: Option<PathBuf>,

    /// PID file path (for daemon mode)
    #[arg(long, default_value = "/tmp/mclre-server.pid")]
    pid_file: PathBuf,

    /// Log directory (for daemon mode)
    #[arg(long)]
    log_dir: Option<PathBuf>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum DifficultyArg {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl From<DifficultyArg> for GameMode {
    fn from(d: DifficultyArg) -> Self {
        match d {
            DifficultyArg::Peaceful => GameMode::Survival,
            DifficultyArg::Easy => GameMode::Survival,
            DifficultyArg::Normal => GameMode::Survival,
            DifficultyArg::Hard => GameMode::Survival,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum LogLevelArg {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevelArg> for &'static str {
    fn from(l: LogLevelArg) -> Self {
        match l {
            LogLevelArg::Trace => "trace",
            LogLevelArg::Debug => "debug",
            LogLevelArg::Info => "info",
            LogLevelArg::Warn => "warn",
            LogLevelArg::Error => "error",
        }
    }
}

/// Server configuration struct that can be serialized to/from TOML.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ServerConfigFile {
    pub seed: i64,
    pub max_players: u32,
    pub motd: String,
    pub difficulty: String,
    pub port: u16,
    pub advertise_lan: bool,
    pub online_mode: bool,
    pub view_distance: u8,
    pub world_name: String,
    pub log_level: String,
    pub upnp: bool,
    pub tick_rate: u32,
    pub daemon: bool,
    pub log_dir: Option<String>,
    pub pid_file: String,
}

impl Default for ServerConfigFile {
    fn default() -> Self {
        Self {
            seed: 0,
            max_players: 20,
            motd: "MCLRE Server".to_string(),
            difficulty: "survival".to_string(),
            port: 25565,
            advertise_lan: true,
            online_mode: false,
            view_distance: 10,
            world_name: "world".to_string(),
            log_level: "info".to_string(),
            upnp: true,
            tick_rate: 20,
            daemon: false,
            log_dir: None,
            pid_file: "/tmp/mclre-server.pid".to_string(),
        }
    }
}

async fn run_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new(config);

    // Set up signal handling for graceful shutdown
    let shutdown_signal = async {
        tokio::signal::ctrl_c().await.ok();
    };

    tokio::select! {
        result = server.run_lan() => {
            match result {
                Ok(_) => info!("Server stopped gracefully"),
                Err(e) => error!("Server error: {}", e),
            }
        }
        _ = shutdown_signal => {
            info!("Shutdown signal received, stopping server...");
        }
    }

    Ok(())
}

fn write_pid_file(pid_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let pid = std::process::id().to_string();
    fs::write(pid_file, pid)?;
    Ok(())
}

fn remove_pid_file(pid_file: &PathBuf) {
    let _ = fs::remove_file(pid_file);
}

fn generate_config_file(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfigFile::default();
    let toml = toml::to_string_pretty(&config)?;
    fs::write(path, toml)?;
    info!("Generated default config file at: {}", path.display());
    Ok(())
}

fn load_config_file(path: &PathBuf) -> Result<ServerConfigFile, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: ServerConfigFile = toml::from_str(&content)?;
    Ok(config)
}

fn daemonize(pid_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Write PID file
    write_pid_file(pid_file)?;

    // On Unix, we could fork here. On Windows, we just continue.
    // For a true daemon on Unix, we'd use double-fork.
    // For now, we just write the PID file and continue.
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Handle config file generation
    if let Some(path) = &args.generate_config {
        return generate_config_file(path);
    }

    // Load config file if provided
    let file_config = if let Some(path) = &args.config {
        Some(load_config_file(path)?)
    } else {
        None
    };

    // Merge config: CLI args override file config
    let seed = file_config.as_ref().map(|c| c.seed).unwrap_or(args.seed);
    let max_players = file_config
        .as_ref()
        .map(|c| c.max_players)
        .unwrap_or(args.max_players);
    let motd = file_config
        .as_ref()
        .map(|c| c.motd.clone())
        .unwrap_or(args.motd);
    let difficulty = file_config
        .as_ref()
        .map(|c| c.difficulty.clone())
        .unwrap_or_else(|| format!("{:?}", args.difficulty).to_lowercase());
    let port = file_config.as_ref().map(|c| c.port).unwrap_or(args.port);
    let advertise_lan = file_config
        .as_ref()
        .map(|c| c.advertise_lan)
        .unwrap_or(args.advertise_lan);
    let online_mode = file_config
        .as_ref()
        .map(|c| c.online_mode)
        .unwrap_or(args.online_mode);
    let view_distance = file_config
        .as_ref()
        .map(|c| c.view_distance)
        .unwrap_or(args.view_distance);
    let world_name = file_config
        .as_ref()
        .map(|c| c.world_name.clone())
        .unwrap_or(args.world_name);
    let log_level = file_config
        .as_ref()
        .map(|c| c.log_level.clone())
        .unwrap_or_else(|| {
            let level_str: &str = args.log_level.into();
            level_str.to_string()
        });
    let upnp = file_config.as_ref().map(|c| c.upnp).unwrap_or(args.upnp);
    let tick_rate = file_config
        .as_ref()
        .map(|c| c.tick_rate)
        .unwrap_or(args.tick_rate);
    let daemon = file_config
        .as_ref()
        .map(|c| c.daemon)
        .unwrap_or(args.daemon);
    let log_dir = file_config
        .as_ref()
        .and_then(|c| c.log_dir.clone())
        .or_else(|| {
            args.log_dir
                .clone()
                .map(|p| p.to_string_lossy().to_string())
        });
    let pid_file = file_config
        .as_ref()
        .map(|c| PathBuf::from(&c.pid_file))
        .unwrap_or(args.pid_file.clone());

    // Initialize tracing
    let log_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new::<&str>(log_level.as_str()));

    // Set up logging to file if daemon mode or log_dir specified
    let file_appender = if daemon || log_dir.is_some() {
        let log_dir = log_dir.unwrap_or_else(|| "/var/log/mclre".to_string());
        fs::create_dir_all(&log_dir)?;
        let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "mclre-server.log");
        Some(file_appender)
    } else {
        None
    };

    if let Some(file_appender) = file_appender {
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        FmtSubscriber::builder()
            .with_env_filter(log_filter)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_writer(non_blocking)
            .init();
    } else {
        FmtSubscriber::builder()
            .with_env_filter(log_filter)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_span_events(FmtSpan::CLOSE)
            .init();
    };

    info!("Starting MCLRE Dedicated Server");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    if daemon {
        info!("Running in daemon mode");
        daemonize(&pid_file)?;
    }

    // Build server configuration
    let difficulty = match difficulty.as_str() {
        "peaceful" => DifficultyArg::Peaceful,
        "easy" => DifficultyArg::Easy,
        "normal" => DifficultyArg::Normal,
        "hard" => DifficultyArg::Hard,
        _ => DifficultyArg::Normal,
    };

    let config = ServerConfig {
        world_seed: seed,
        max_players: max_players,
        motd,
        difficulty: difficulty.into(),
        online_mode,
        view_distance,
        lan_port: port,
        advertise_lan: advertise_lan && !false, // no_mdns not in file config
    };

    // Handle UPnP setup if enabled
    let _upnp_manager: Option<mclre_net::UpnpPortManager> = if upnp {
        info!("UPnP enabled, attempting to discover gateway...");
        match mclre_net::setup_default_port_mapping(config.lan_port) {
            Ok(manager) => {
                if manager.is_available() {
                    info!("UPnP gateway found: {:?}", manager.external_ip());
                    info!(
                        "External address for sharing: {:?}",
                        mclre_net::get_public_address(&manager, config.lan_port)
                    );
                } else {
                    warn!("UPnP gateway not found, port mapping unavailable (local only)");
                }
                Some(manager)
            }
            Err(e) => {
                warn!("UPnP setup failed: {}", e);
                None
            }
        }
    } else {
        info!("UPnP disabled");
        None
    };

    info!("Server configuration:");
    info!("  World seed: {}", config.world_seed);
    info!("  Max players: {}", config.max_players);
    info!("  MOTD: {}", config.motd);
    info!("  Difficulty: {:?}", config.difficulty);
    info!("  Online mode: {}", config.online_mode);
    info!("  View distance: {} chunks", config.view_distance);
    info!("  LAN port: {}", config.lan_port);
    info!("  Advertise via mDNS: {}", config.advertise_lan);

    // Run server
    let result = run_server(config).await;

    // Clean up PID file on exit
    if daemon {
        remove_pid_file(&pid_file);
    }

    result
}
