// nodadark-tui/src/events.rs
// NodaDark v0.1.5 — F1/F2/F3 filtres rapides + x HAR export

use crate::{
    network::EngineClient,
    state::{ActivePanel, AppState, DetailTab},
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub async fn handle_key(key: KeyEvent, app: &mut AppState, engine: &mut EngineClient) -> Result<()> {
    match &app.active_panel {
        ActivePanel::RequestList       => handle_list_keys(key, app, engine).await,
        ActivePanel::RequestDetail     => handle_detail_keys(key, app, engine).await,
        ActivePanel::Search            => handle_search_keys(key, app),
        ActivePanel::PopupAction       => handle_popup_action_keys(key, app, engine).await,
        ActivePanel::PopupCookieEditor => handle_cookie_editor_keys(key, app),
        ActivePanel::PopupConfirmReplay => handle_confirm_replay_keys(key, app, engine).await,
    }
    Ok(())
}

async fn handle_list_keys(key: KeyEvent, app: &mut AppState, engine: &mut EngineClient) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => app.select_down(),
        KeyCode::Char('k') | KeyCode::Up   => app.select_up(),

        KeyCode::Enter => app.open_detail(),

        KeyCode::Char('a') => app.open_action_popup(),

        KeyCode::Char('/') => {
            app.search_active = true;
            app.search_input.clear();
            app.active_panel = ActivePanel::Search;
        }

        // F1 — filtrer seulement les erreurs 4xx/5xx
        KeyCode::F(1) => {
            app.filter_text = "4".into();
            app.list_offset = 0;
            app.status_message = Some("F1 — Filtre: erreurs 4xx/5xx".into());
        }

        // F2 — filtrer seulement les POST
        KeyCode::F(2) => {
            app.filter_text = "POST".into();
            app.list_offset = 0;
            app.status_message = Some("F2 — Filtre: requêtes POST".into());
        }

        // F3 — filtrer seulement HTTPS
        KeyCode::F(3) => {
            app.filter_text = "https".into();
            app.list_offset = 0;
            app.status_message = Some("F3 — Filtre: requêtes HTTPS".into());
        }

        // Esc — effacer le filtre actif
        KeyCode::Esc => {
            if !app.filter_text.is_empty() {
                app.filter_text.clear();
                app.list_offset = 0;
                app.status_message = Some("Filtre effacé".into());
            }
        }

        // x — export HAR
        KeyCode::Char('x') => {
            engine.send_command(&serde_json::json!({
                "command": "export_har",
                "name": "nodadark_export"
            })).await;
            app.status_message = Some("📦 Export HAR lancé → ~/.local/share/nodadark/".into());
        }

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

        KeyCode::Char('d') => {
            if let Some(req) = app.get_selected() {
                let id = req.id.clone();
                engine.drop_request(&id).await;
                app.status_message = Some(format!("✂ Requête {id} droppée"));
            }
        }

        KeyCode::Char('r') => {
            if let Some(req) = app.get_selected() {
                let id = req.id.clone();
                engine.replay(&id).await;
                app.status_message = Some(format!("↪ Replay : {id}"));
            }
        }

        KeyCode::Char('i') => {
            if let Some(req) = app.get_selected() {
                let id = req.id.clone();
                engine.get_request(&id).await;
            }
        }

        KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
            engine.clear().await;
            app.requests.clear();
            app.list_offset = 0;
            app.status_message = Some("🗑  Historique effacé".into());
        }

        KeyCode::Char('G') => {
            let count = app.filtered_requests().len();
            if count > 0 { app.list_offset = count - 1; }
        }

        KeyCode::Char('g') => { app.list_offset = 0; }

        KeyCode::PageDown => { for _ in 0..10 { app.select_down(); } }
        KeyCode::PageUp   => { for _ in 0..10 { app.select_up(); }   }

        _ => {}
    }

}

async fn handle_detail_keys(key: KeyEvent, app: &mut AppState, engine: &mut EngineClient) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.active_panel = ActivePanel::RequestList;
        }

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

        // x — export HAR depuis le détail aussi
        KeyCode::Char('x') => {
            engine.send_command(&serde_json::json!({
                "command": "export_har",
                "name": "nodadark_export"
            })).await;
            app.status_message = Some("📦 Export HAR lancé".into());
        }

        KeyCode::Char('r') => {
            if let Some(req) = &app.selected_request {
                let id = req.id.clone();
                engine.replay(&id).await;
                app.status_message = Some(format!("↪ Replay : {id}"));
            }
        }
        KeyCode::Char('e') => { app.open_cookie_editor(); }
        KeyCode::Char('a') => { app.open_action_popup(); }

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
            app.filter_text = app.search_input.clone();
            app.list_offset = 0;
        }
        KeyCode::Char(c) => {
            app.search_input.push(c);
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
        0 => {
            if let Some(id) = req_id {
                engine.replay(&id).await;
                app.status_message = Some(format!("↪ Replay envoyé : {id}"));
            }
            app.active_panel = ActivePanel::RequestList;
        }
        1 => { app.open_cookie_editor(); }
        2 => { app.open_cookie_editor(); }
        3 => {
            if let Some(id) = req_id {
                engine.drop_request(&id).await;
                app.status_message = Some(format!("✂ Droppé : {id}"));
            }
            app.active_panel = ActivePanel::RequestList;
        }
        4 => {
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
