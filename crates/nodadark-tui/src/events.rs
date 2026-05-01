// nodadark-tui/src/events.rs
// Gestion des événements clavier (style Vim)

use crate::{
    network::EngineClient,
    state::{ActivePanel, AppState, DetailTab},
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub async fn handle_key(key: KeyEvent, app: &mut AppState, engine: &mut EngineClient) -> Result<()> {
    match &app.active_panel {
        ActivePanel::RequestList => handle_list_keys(key, app, engine).await,
        ActivePanel::RequestDetail => handle_detail_keys(key, app, engine).await,
        ActivePanel::Search => handle_search_keys(key, app),
        ActivePanel::PopupAction => handle_popup_action_keys(key, app, engine).await,
        ActivePanel::PopupCookieEditor => handle_cookie_editor_keys(key, app),
        ActivePanel::PopupConfirmReplay => handle_confirm_replay_keys(key, app, engine).await,
    }
    Ok(())
}

async fn handle_list_keys(key: KeyEvent, app: &mut AppState, engine: &mut EngineClient) {
    match key.code {
        // Navigation Vim
        KeyCode::Char('j') | KeyCode::Down => app.select_down(),
        KeyCode::Char('k') | KeyCode::Up   => app.select_up(),

        // Ouvrir le détail
        KeyCode::Enter => app.open_detail(),

        // Popup d'actions
        KeyCode::Char('a') => app.open_action_popup(),

        // Recherche / filtre
        KeyCode::Char('/') => {
            app.search_active = true;
            app.search_input.clear();
            app.active_panel = ActivePanel::Search;
        }

        // Pause / Reprise
        KeyCode::Char('p') => {
            if app.proxy_paused {
                engine.resume().await;
                app.proxy_paused = false;
                app.status_message = Some("▶  Proxy repris".into());
            } else {
                engine.pause().await;
                app.proxy_paused = true;
                app.status_message = Some("⏸  Proxy en pause".into());
            }
        }

        // Drop direct (dd style Vim)
        KeyCode::Char('d') => {
            if let Some(req) = app.get_selected() {
                let id = req.id.clone();
                engine.drop_request(&id).await;
                app.status_message = Some(format!("✂ Requête {id} droppée"));
            }
        }

        // Replay direct
        KeyCode::Char('r') => {
            if let Some(req) = app.get_selected() {
                let id = req.id.clone();
                engine.replay(&id).await;
                app.status_message = Some(format!("↪ Replay : {id}"));
            }
        }

        // Récupérer le détail complet depuis le moteur
        KeyCode::Char('i') => {
            if let Some(req) = app.get_selected() {
                let id = req.id.clone();
                engine.get_request(&id).await;
            }
        }

        // Effacer tout
        KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
            engine.clear().await;
            app.requests.clear();
            app.list_offset = 0;
            app.status_message = Some("🗑  Historique effacé".into());
        }

        // Aller au dernier item
        KeyCode::Char('G') => {
            let count = app.filtered_requests().len();
            if count > 0 {
                app.list_offset = count - 1;
            }
        }

        // Aller au premier item
        KeyCode::Char('g') => {
            app.list_offset = 0;
        }

        // Page bas / haut
        KeyCode::PageDown => {
            for _ in 0..10 { app.select_down(); }
        }
        KeyCode::PageUp => {
            for _ in 0..10 { app.select_up(); }
        }

        _ => {}
    }
}

async fn handle_detail_keys(key: KeyEvent, app: &mut AppState, engine: &mut EngineClient) {
    match key.code {
        // Retour à la liste
        KeyCode::Esc | KeyCode::Char('q') => {
            app.active_panel = ActivePanel::RequestList;
        }

        // Onglets
        KeyCode::Tab => {
            app.detail_tab = match app.detail_tab {
                DetailTab::Headers => DetailTab::Body,
                DetailTab::Body    => DetailTab::Hex,
                DetailTab::Hex     => DetailTab::Headers,
            };
            app.detail_scroll = 0;
        }

        KeyCode::Char('1') => { app.detail_tab = DetailTab::Headers; app.detail_scroll = 0; }
        KeyCode::Char('2') => { app.detail_tab = DetailTab::Body;    app.detail_scroll = 0; }
        KeyCode::Char('3') => { app.detail_tab = DetailTab::Hex;     app.detail_scroll = 0; }

        // Scroll dans le contenu
        KeyCode::Char('j') | KeyCode::Down  => {
            app.detail_scroll = app.detail_scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.detail_scroll = app.detail_scroll.saturating_sub(1);
        }
        KeyCode::PageDown | KeyCode::Char('f') => {
            app.detail_scroll = app.detail_scroll.saturating_add(10);
        }
        KeyCode::PageUp | KeyCode::Char('b') => {
            app.detail_scroll = app.detail_scroll.saturating_sub(10);
        }

        // Actions
        KeyCode::Char('r') => {
            if let Some(req) = &app.selected_request {
                let id = req.id.clone();
                engine.replay(&id).await;
                app.status_message = Some(format!("↪ Replay : {id}"));
            }
        }
        KeyCode::Char('e') => {
            app.open_cookie_editor();
        }
        KeyCode::Char('a') => {
            app.open_action_popup();
        }

        _ => {}
    }
}

fn handle_search_keys(key: KeyEvent, app: &mut AppState) {
    match key.code {
        KeyCode::Esc => {
            app.search_active = false;
            app.search_input.clear();
            app.filter_text.clear();
            app.active_panel = ActivePanel::RequestList;
        }
        KeyCode::Enter => {
            app.filter_text = app.search_input.clone();
            app.search_active = false;
            app.list_offset = 0;
            app.active_panel = ActivePanel::RequestList;
        }
        KeyCode::Backspace => {
            app.search_input.pop();
            // Filtrage live
            app.filter_text = app.search_input.clone();
            app.list_offset = 0;
        }
        KeyCode::Char(c) => {
            app.search_input.push(c);
            // Filtrage live
            app.filter_text = app.search_input.clone();
            app.list_offset = 0;
        }
        _ => {}
    }
}

async fn handle_popup_action_keys(key: KeyEvent, app: &mut AppState, engine: &mut EngineClient) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.active_panel = if app.selected_request.is_some() {
                ActivePanel::RequestDetail
            } else {
                ActivePanel::RequestList
            };
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.popup_selected + 1 < app.popup_items.len() {
                app.popup_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.popup_selected > 0 {
                app.popup_selected -= 1;
            }
        }
        KeyCode::Enter => {
            execute_popup_action(app.popup_selected, app, engine).await;
        }
        _ => {}
    }
}

async fn execute_popup_action(idx: usize, app: &mut AppState, engine: &mut EngineClient) {
    let req_id = app.selected_request.as_ref().map(|r| r.id.clone())
        .or_else(|| app.get_selected().map(|r| r.id.clone()));

    match idx {
        0 => { // Replay
            if let Some(id) = req_id {
                engine.replay(&id).await;
                app.status_message = Some(format!("↪ Replay envoyé : {id}"));
            }
            app.active_panel = ActivePanel::RequestList;
        }
        1 => { // Éditer et rejouer → ouvrir cookie editor pour l'instant
            app.open_cookie_editor();
        }
        2 => { // Éditer cookies
            app.open_cookie_editor();
        }
        3 => { // Drop
            if let Some(id) = req_id {
                engine.drop_request(&id).await;
                app.status_message = Some(format!("✂ Droppé : {id}"));
            }
            app.active_panel = ActivePanel::RequestList;
        }
        4 => { // Copier URL (dans le log pour l'instant)
            if let Some(req) = &app.selected_request {
                tracing::info!("URL copiée : {}", req.url);
                app.status_message = Some(format!("📋 URL : {}", req.url));
            }
            app.active_panel = ActivePanel::RequestList;
        }
        _ => {}
    }
}

fn handle_cookie_editor_keys(key: KeyEvent, app: &mut AppState) {
    match key.code {
        KeyCode::Esc => {
            app.cookie_editing = None;
            app.active_panel = ActivePanel::RequestDetail;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.cookie_selected + 1 < app.cookie_rows.len() {
                app.cookie_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.cookie_selected > 0 {
                app.cookie_selected -= 1;
            }
        }
        KeyCode::Enter => {
            // Commencer l'édition de la valeur du cookie sélectionné
            if let Some(row) = app.cookie_rows.get(app.cookie_selected) {
                app.cookie_editing = Some((1, row.1.clone()));
            }
        }
        KeyCode::Char(c) if app.cookie_editing.is_some() => {
            if let Some((_, ref mut buf)) = app.cookie_editing {
                buf.push(c);
            }
        }
        KeyCode::Backspace if app.cookie_editing.is_some() => {
            if let Some((_, ref mut buf)) = app.cookie_editing {
                buf.pop();
            }
        }
        KeyCode::Tab if app.cookie_editing.is_some() => {
            // Confirmer l'édition
            if let Some((col, ref buf)) = app.cookie_editing.clone() {
                if col == 1 {
                    if let Some(row) = app.cookie_rows.get_mut(app.cookie_selected) {
                        row.1 = buf.clone();
                    }
                }
                app.cookie_editing = None;
            }
        }
        _ => {}
    }
}

async fn handle_confirm_replay_keys(key: KeyEvent, app: &mut AppState, engine: &mut EngineClient) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Enter => {
            if let Some(req) = &app.selected_request {
                let id = req.id.clone();
                engine.replay(&id).await;
                app.status_message = Some(format!("↪ Replay confirmé : {id}"));
            }
            app.active_panel = ActivePanel::RequestList;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.active_panel = ActivePanel::RequestList;
        }
        _ => {}
    }
}
