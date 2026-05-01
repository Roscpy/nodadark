// nodadark-engine/src/main.rs
// Binaire standalone : démarre le proxy et log tout sans UI

use anyhow::Result;
use clap::Parser;
use nodadark_engine::{EngineEvent, ProxyConfig, ProxyEngine};
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser, Debug)]
#[command(name = "nodadark")]
#[command(about = "NodaDark — Proxy d'interception réseau haute performance")]
#[command(version = "0.1.0")]
struct Cli {
    #[arg(short, long, default_value_t = 8080, help = "Port d'écoute du proxy")]
    port: u16,

    #[arg(short, long, default_value = "127.0.0.1", help = "Adresse d'écoute")]
    bind: String,

    #[arg(long, default_value_t = 9090, help = "Port de l'API de contrôle")]
    api_port: u16,

    #[arg(
        long,
        default_value = "/tmp/nodadark.sock",
        help = "Chemin du socket Unix"
    )]
    socket: String,

    #[arg(long, help = "Désactive le mode Fail-Open (bloque les certs invalides)")]
    strict: bool,

    #[arg(short, long, help = "Active les logs verbeux")]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Logging
    let filter = if cli.verbose { "debug" } else { "info" };
    fmt()
        .with_env_filter(EnvFilter::new(filter))
        .with_target(false)
        .init();

    print_banner();

    let config = ProxyConfig {
        port: cli.port,
        bind: cli.bind.clone(),
        api_port: cli.api_port,
        socket_path: cli.socket.clone(),
        fail_open: !cli.strict,
        ..Default::default()
    };

    let (engine, mut rx) = ProxyEngine::new(config);

    // Log events to stdout (mode sans UI)
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => log_event(&event),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("⚠ {n} événements perdus (buffer plein)");
                }
                Err(_) => break,
            }
        }
    });

    tracing::info!("──────────────────────────────────────");
    tracing::info!(
        "📡 Proxy    : http://{}:{}",
        cli.bind,
        cli.port
    );
    tracing::info!("🔌 API      : tcp://127.0.0.1:{}", cli.api_port);
    tracing::info!("🔌 Socket   : {}", cli.socket);
    tracing::info!("──────────────────────────────────────");
    tracing::info!("Configurez votre appareil pour utiliser ce proxy.");
    tracing::info!("Installez le CA : {}/nodadark-ca.crt", dirs::config_dir().unwrap_or_default().join("nodadark/certs").display());
    tracing::info!("──────────────────────────────────────");

    engine.start().await
}

fn log_event(event: &EngineEvent) {
    match event {
        EngineEvent::Request { method, url, tls, .. } => {
            let scheme = if *tls { "https" } else { "http" };
            tracing::info!("→ [{method}] {scheme}://{url}");
        }
        EngineEvent::Response { id: _, status, duration_ms, size } => {
            let icon = match status {
                200..=299 => "✓",
                300..=399 => "↪",
                400..=499 => "✗",
                _ => "!",
            };
            tracing::info!("← {icon} {status} ({duration_ms}ms, {size}B)");
        }
        EngineEvent::Dropped { id } => {
            tracing::warn!("✂ Requête droppée : {id}");
        }
        EngineEvent::RequestError { id: _, error } => {
            tracing::error!("💀 Erreur : {error}");
        }
        EngineEvent::ProxyState { paused, port } => {
            if *paused {
                tracing::warn!("⏸  Proxy mis en pause (port {port})");
            } else {
                tracing::info!("▶  Proxy repris (port {port})");
            }
        }
        EngineEvent::RuleMatched { id: _, rule_name } => {
            tracing::info!("⚡ Règle déclenchée : {rule_name}");
        }
    }
}

fn print_banner() {
    println!(
        r#"
  ╔═╗╔╗╔╔═╗╔╦╗╔═╗╔╦╗╔═╗╦═╗╦╔═
  ║  ║║║║ ║ ║║╠═╣ ║║╠═╣╠╦╝╠╩╗
  ╚═╝╝╚╝╚═╝═╩╝╩ ╩═╩╝╩ ╩╩╚═╩ ╩
  Proxy d'Interception Réseau v0.1.0
  ⚠  À utiliser uniquement sur des réseaux autorisés.
"#
    );
}
