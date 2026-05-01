// nodadark-tui/src/app.rs
// Boucle principale de l'application TUI

use crate::{
    events::handle_key,
    network::EngineClient,
    state::AppState,
    ui,
};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

pub async fn run(socket_path: String, api_port: u16) -> Result<()> {
    // Connexion au moteur
    let mut engine = connect_to_engine(&socket_path, api_port).await?;

    // Initialiser le terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let mut app = AppState::new();
    app.status_message = Some("✅ Connecté au moteur NodaDark".into());

    // Charger les requêtes existantes au démarrage
    engine.request_list(0, 200).await;

    let result = run_loop(&mut term, &mut app, &mut engine).await;

    // Restaurer le terminal
    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    term.show_cursor()?;

    result
}

async fn run_loop(
    term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut AppState,
    engine: &mut EngineClient,
) -> Result<()> {
    loop {
        // Rendu de l'interface
        term.draw(|f| ui::render(f, app))?;

        // Événements avec timeout court pour rester réactif
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // Quitter avec 'q' ou Ctrl+C
                if key.code == KeyCode::Char('q') && !app.is_editing() {
                    break;
                }
                if key.code == KeyCode::Char('c')
                    && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    break;
                }
                handle_key(key, app, engine).await?;
            }
        }

        // Vider les messages en attente du moteur
        engine.poll_messages(app).await;
    }

    Ok(())
}

async fn connect_to_engine(socket_path: &str, api_port: u16) -> Result<EngineClient> {
    // Essayer le socket Unix d'abord
    #[cfg(unix)]
    {
        if let Ok(client) = EngineClient::connect_unix(socket_path).await {
            return Ok(client);
        }
    }

    // Fallback TCP
    let addr = format!("127.0.0.1:{api_port}");
    match EngineClient::connect_tcp(&addr).await {
        Ok(client) => Ok(client),
        Err(_) => {
            // Mode démo sans moteur
            Ok(EngineClient::demo_mode())
        }
    }
}
