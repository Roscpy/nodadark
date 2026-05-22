// nodadark-tui/src/main.rs
// NodaDark v0.1.5 — embedded mode stable

mod app;
mod events;
mod network;
mod state;
mod ui;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "nodadark-tui")]
#[command(about = "NodaDark — Interface Terminal")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[arg(short, long, default_value = "/tmp/nodadark.sock")]
    socket: String,

    #[arg(short, long, default_value_t = 9090)]
    port: u16,

    // Mode embedded — lance le moteur intégré sur ce port proxy
    #[arg(long)]
    embedded: Option<u16>,

}

#[tokio::main]
async fn main() -> Result<()> {
    let log_path = std::env::temp_dir().join("nodadark-tui.log");
    let file = std::fs::File::create(&log_path)?;
    tracing_subscriber::fmt()
        .with_writer(file)
        .with_ansi(false)
        .init();

    let cli = Cli::parse();

    if let Some(proxy_port) = cli.embedded {
        let config = nodadark_engine::ProxyConfig {
            port: proxy_port,
            // API sur port fixe 9090 en mode embedded
            api_port: 9090,
            ..Default::default()
        };

        tokio::spawn(async move {
            let (engine, _) = nodadark_engine::ProxyEngine::new(config);
            if let Err(e) = engine.start().await {
                tracing::error!("Moteur integre: {e}");
            }
        });

        // Attendre que le moteur soit prêt — 1.5s sur Android
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

        // En mode embedded → forcer connexion TCP, pas Unix socket
        return app::run("/dev/null".into(), 9090).await;
    }

    app::run(cli.socket, cli.port).await

}
