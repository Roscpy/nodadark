// nodadark-tui/src/main.rs

mod app;
mod events;
mod network;
mod state;
mod ui;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "nodadark-tui")]
#[command(about = "NodaDark — Interface Terminal (style Hacker)")]
#[command(version = "0.1.0")]
struct Cli {
    /// Chemin du socket Unix du moteur
    #[arg(short, long, default_value = "/tmp/nodadark.sock")]
    socket: String,

    /// Port TCP de l'API (si socket Unix non disponible)
    #[arg(short, long, default_value_t = 9090)]
    port: u16,

    /// Démarrer le moteur intégré sur ce port proxy
    #[arg(long)]
    embedded: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Logger vers un fichier (pas vers stdout — le TUI l'utilise)
    let log_path = std::env::temp_dir().join("nodadark-tui.log");
    let file = std::fs::File::create(&log_path)?;
    tracing_subscriber::fmt()
        .with_writer(file)
        .with_ansi(false)
        .init();

    let cli = Cli::parse();

    // Optionnellement démarrer le moteur en arrière-plan
    if let Some(proxy_port) = cli.embedded {
        let config = nodadark_engine::ProxyConfig {
            port: proxy_port,
            ..Default::default()
        };
        tokio::spawn(async move {
            let (engine, _) = nodadark_engine::ProxyEngine::new(config);
            if let Err(e) = engine.start().await {
                tracing::error!("Moteur intégré: {e}");
            }
        });
        // Laisser le moteur démarrer
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    app::run(cli.socket, cli.port).await
}
