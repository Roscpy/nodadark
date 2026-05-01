// nodadark-tui/src/ui.rs
// Rendu complet de l'interface terminal avec Ratatui

use crate::state::{ActivePanel, AppState, DetailTab};
use nodadark_engine::{InterceptedRequest, RequestState};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, Tabs, Wrap,
    },
    Frame,
};

// ── Palette de couleurs NodaDark ─────────────────────────────
const COLOR_BG:       Color = Color::Rgb(10,  12,  20);
const COLOR_PANEL:    Color = Color::Rgb(18,  22,  36);
const COLOR_BORDER:   Color = Color::Rgb(40,  80,  130);
const COLOR_ACCENT:   Color = Color::Cyan;
const COLOR_GREEN:    Color = Color::Rgb(80,  230, 120);
const COLOR_RED:      Color = Color::Rgb(220, 50,  50);
const COLOR_YELLOW:   Color = Color::Rgb(220, 180, 50);
const COLOR_CYAN:     Color = Color::Rgb(50,  200, 220);
const COLOR_GRAY:     Color = Color::Rgb(100, 110, 130);
const COLOR_WHITE:    Color = Color::Rgb(210, 215, 230);
const COLOR_SELECTED: Color = Color::Rgb(30,  60,  110);

pub fn render<B: Backend>(f: &mut Frame<B>, app: &AppState) {
    let area = f.size();

    // Fond global
    f.render_widget(
        Block::default().style(Style::default().bg(COLOR_BG)),
        area,
    );

    // Layout principal : header + body + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header / titre
            Constraint::Min(0),     // Contenu
            Constraint::Length(1),  // Barre d'état
        ])
        .split(area);

    render_header(f, chunks[0], app);
    render_body(f, chunks[1], app);
    render_status_bar(f, chunks[2], app);

    // Popups (au-dessus de tout)
    match &app.active_panel {
        ActivePanel::PopupAction       => render_action_popup(f, area, app),
        ActivePanel::PopupCookieEditor => render_cookie_editor(f, area, app),
        ActivePanel::PopupConfirmReplay => render_confirm_popup(f, area),
        _ => {}
    }
}

fn render_header<B: Backend>(f: &mut Frame<B>, area: Rect, app: &AppState) {
    let paused_style = if app.proxy_paused {
        Style::default().fg(COLOR_YELLOW).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(COLOR_GREEN).add_modifier(Modifier::BOLD)
    };

    let status_icon = if app.proxy_paused { "⏸  PAUSE" } else { "▶  LIVE" };
    let tls_icon    = "🔒";
    let conn_icon   = if app.engine_connected { "●" } else { "○" };

    let title = Line::from(vec![
        Span::styled(" ╔═╗ NodaDark ", Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("v0.1  ", Style::default().fg(COLOR_GRAY)),
        Span::styled(status_icon, paused_style),
        Span::styled(format!("  {tls_icon} MITM  "), Style::default().fg(COLOR_CYAN)),
        Span::styled(format!("{conn_icon} Proxy :{}  ", app.proxy_port),
            Style::default().fg(if app.engine_connected { COLOR_GREEN } else { COLOR_RED })),
        Span::styled(
            format!(" {} requêtes  ", app.requests.len()),
            Style::default().fg(COLOR_GRAY),
        ),
        Span::styled(
            "  [q]Quit [p]Pause [/]Filtre [j/k]Nav [Enter]Détail [a]Actions [r]Replay",
            Style::default().fg(COLOR_GRAY),
        ),
    ]);

    let header = Paragraph::new(title)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_BORDER))
                .style(Style::default().bg(COLOR_PANEL)),
        )
        .alignment(Alignment::Left);

    f.render_widget(header, area);
}

fn render_body<B: Backend>(f: &mut Frame<B>, area: Rect, app: &AppState) {
    if app.selected_request.is_some() && app.active_panel != ActivePanel::RequestList {
        // Vue split : liste (40%) + détail (60%)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
            .split(area);

        render_request_list(f, chunks[0], app);
        render_request_detail(f, chunks[1], app);
    } else {
        // Vue liste seule
        render_request_list(f, area, app);
    }
}

fn render_request_list<B: Backend>(f: &mut Frame<B>, area: Rect, app: &AppState) {
    let filtered = app.filtered_requests();
    let title = if app.filter_text.is_empty() {
        format!(" Requêtes ({}) ", filtered.len())
    } else {
        format!(" Requêtes ({}) — Filtre: \"{}\" ", filtered.len(), app.filter_text)
    };

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, req)| {
            let is_selected = i == app.list_offset;
            let method_color = match req.method.as_str() {
                "GET"    => COLOR_GREEN,
                "POST"   => COLOR_CYAN,
                "PUT"    => COLOR_YELLOW,
                "DELETE" => COLOR_RED,
                _        => COLOR_WHITE,
            };

            let status_color = match req.response_status {
                Some(s) if s >= 500 => COLOR_RED,
                Some(s) if s >= 400 => Color::Rgb(220, 100, 50),
                Some(s) if s >= 300 => COLOR_YELLOW,
                Some(s) if s >= 200 => COLOR_GREEN,
                None                => COLOR_CYAN, // En attente
                _                   => COLOR_GRAY,
            };

            let status_str = req.response_status
                .map(|s| s.to_string())
                .unwrap_or_else(|| "···".into());

            let duration = req.duration_ms
                .map(|d| format!("{d}ms"))
                .unwrap_or_else(|| "···".into());

            let host_short = req.host.chars().take(28).collect::<String>();
            let path_short = req.path.chars().take(22).collect::<String>();
            let tls_icon   = if req.tls { "🔒" } else { "  " };

            let dropped = req.state == RequestState::Dropped;
            let base_style = if is_selected {
                Style::default().bg(COLOR_SELECTED)
            } else if dropped {
                Style::default().fg(COLOR_GRAY).add_modifier(Modifier::DIM)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(format!(" {tls_icon}"), Style::default().fg(COLOR_GRAY)),
                Span::styled(
                    format!("[{:<6}]", req.method),
                    Style::default().fg(method_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {status_str} "),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    format!("{host_short}{path_short} "),
                    Style::default().fg(if is_selected { COLOR_WHITE } else { Color::Rgb(160,170,190) }),
                ),
                Span::styled(
                    format!("{duration} "),
                    Style::default().fg(COLOR_GRAY),
                ),
            ]);

            let item = ListItem::new(line).style(base_style);
            item
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.list_offset));

    let is_active = matches!(app.active_panel, ActivePanel::RequestList | ActivePanel::Search);
    let border_style = Style::default().fg(if is_active { COLOR_ACCENT } else { COLOR_BORDER });

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(title, Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Style::default().bg(COLOR_PANEL)),
        )
        .highlight_style(Style::default().bg(COLOR_SELECTED).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut list_state);

    // Barre de recherche si active
    if app.active_panel == ActivePanel::Search {
        let search_area = Rect {
            x: area.x + 1,
            y: area.y + area.height - 2,
            width: area.width - 2,
            height: 1,
        };
        let search_text = format!("/ {}_", app.search_input);
        let search_widget = Paragraph::new(search_text)
            .style(Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD));
        f.render_widget(search_widget, search_area);
    }
}

fn render_request_detail<B: Backend>(f: &mut Frame<B>, area: Rect, app: &AppState) {
    let req = match &app.selected_request {
        Some(r) => r,
        None    => return,
    };

    // Layout : info bar + onglets + contenu
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Résumé de la requête
            Constraint::Length(2),  // Onglets
            Constraint::Min(0),     // Contenu de l'onglet
        ])
        .split(area);

    render_detail_info(f, chunks[0], req);
    render_detail_tabs(f, chunks[1], app);
    render_detail_content(f, chunks[2], app, req);
}

fn render_detail_info<B: Backend>(f: &mut Frame<B>, area: Rect, req: &InterceptedRequest) {
    let status_color = match req.response_status {
        Some(s) if s >= 500 => COLOR_RED,
        Some(s) if s >= 400 => Color::Rgb(220,100,50),
        Some(s) if s >= 300 => COLOR_YELLOW,
        Some(_)             => COLOR_GREEN,
        None                => COLOR_CYAN,
    };

    let status_str = req.response_status.map(|s| s.to_string()).unwrap_or("···".into());
    let duration   = req.duration_ms.map(|d| format!("{d}ms")).unwrap_or("···".into());
    let tls_str    = if req.tls { "HTTPS 🔒" } else { "HTTP" };

    let info = Line::from(vec![
        Span::styled(format!(" {} ", req.method), Style::default().fg(COLOR_GREEN).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{tls_str} "), Style::default().fg(COLOR_CYAN)),
        Span::styled(req.url.clone(), Style::default().fg(COLOR_WHITE)),
        Span::styled(format!("  → {status_str} ", ), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        Span::styled(format!("({duration})"), Style::default().fg(COLOR_GRAY)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_ACCENT))
        .style(Style::default().bg(COLOR_PANEL));

    f.render_widget(Paragraph::new(info).block(block), area);
}

fn render_detail_tabs<B: Backend>(f: &mut Frame<B>, area: Rect, app: &AppState) {
    let tab_names = vec!["[1] Headers", "[2] Body", "[3] Hex"];
    let selected_idx = match app.detail_tab {
        DetailTab::Headers => 0,
        DetailTab::Body    => 1,
        DetailTab::Hex     => 2,
    };

    let tabs = Tabs::new(tab_names.iter().map(|t| Line::from(*t)).collect::<Vec<_>>())
        .select(selected_idx)
        .style(Style::default().fg(COLOR_GRAY))
        .highlight_style(Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD | Modifier::UNDERLINED))
        .divider(Span::raw("  │  "));

    f.render_widget(tabs, area);
}

fn render_detail_content<B: Backend>(f: &mut Frame<B>, area: Rect, app: &AppState, req: &InterceptedRequest) {
    match app.detail_tab {
        DetailTab::Headers => render_headers_tab(f, area, app, req),
        DetailTab::Body    => render_body_tab(f, area, app, req),
        DetailTab::Hex     => render_hex_tab(f, area, app, req),
    }
}

fn render_headers_tab<B: Backend>(f: &mut Frame<B>, area: Rect, app: &AppState, req: &InterceptedRequest) {
    let mut lines: Vec<Line> = vec![];

    lines.push(Line::from(Span::styled(
        " ──── REQUEST HEADERS ────",
        Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD),
    )));

    for (k, v) in &req.request_headers {
        let key_style = if k.to_lowercase() == "cookie" || k.to_lowercase() == "authorization" {
            Style::default().fg(COLOR_YELLOW).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(COLOR_CYAN)
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {k}: "), key_style),
            Span::styled(v.clone(), Style::default().fg(COLOR_WHITE)),
        ]));
    }

    if !req.response_headers.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " ──── RESPONSE HEADERS ────",
            Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD),
        )));
        for (k, v) in &req.response_headers {
            lines.push(Line::from(vec![
                Span::styled(format!(" {k}: "), Style::default().fg(COLOR_CYAN)),
                Span::styled(v.clone(), Style::default().fg(COLOR_WHITE)),
            ]));
        }
    }

    let scrolled: Vec<Line> = lines.into_iter().skip(app.detail_scroll).collect();

    let para = Paragraph::new(scrolled)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_BORDER))
                .style(Style::default().bg(COLOR_PANEL)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(para, area);
}

fn render_body_tab<B: Backend>(f: &mut Frame<B>, area: Rect, app: &AppState, req: &InterceptedRequest) {
    let body_bytes = req.response_body.as_ref()
        .or(req.request_body.as_ref());

    let content = match body_bytes {
        None => "  (Body vide)".to_string(),
        Some(b) => {
            let raw = String::from_utf8_lossy(b).to_string();
            // Essayer de formatter le JSON
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                serde_json::to_string_pretty(&v).unwrap_or(raw)
            } else {
                raw
            }
        }
    };

    let lines: Vec<Line> = content
        .lines()
        .skip(app.detail_scroll)
        .map(|l| {
            // Coloriser les clés JSON
            if l.trim_start().starts_with('"') {
                Line::from(Span::styled(l.to_string(), Style::default().fg(COLOR_CYAN)))
            } else if l.trim().starts_with('{') || l.trim().starts_with('[') {
                Line::from(Span::styled(l.to_string(), Style::default().fg(COLOR_ACCENT)))
            } else {
                Line::from(Span::styled(l.to_string(), Style::default().fg(COLOR_WHITE)))
            }
        })
        .collect();

    let size_info = body_bytes.map(|b| format!(" ({} octets)", b.len())).unwrap_or_default();
    let title = format!(" Body{size_info} ");

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(title, Style::default().fg(COLOR_ACCENT)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_BORDER))
                .style(Style::default().bg(COLOR_PANEL)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(para, area);
}

fn render_hex_tab<B: Backend>(f: &mut Frame<B>, area: Rect, app: &AppState, req: &InterceptedRequest) {
    let body_bytes = req.response_body.as_ref()
        .or(req.request_body.as_ref());

    let lines: Vec<Line> = match body_bytes {
        None => vec![Line::from(Span::styled("  (Aucune donnée)", Style::default().fg(COLOR_GRAY)))],
        Some(bytes) => {
            bytes
                .chunks(16)
                .enumerate()
                .skip(app.detail_scroll)
                .take(area.height as usize)
                .map(|(i, chunk)| {
                    let offset = format!("{:08x}  ", i * 16);
                    let hex_part: String = chunk
                        .iter()
                        .map(|b| format!("{b:02x} "))
                        .collect::<String>();
                    let padding = " ".repeat((16 - chunk.len()) * 3);
                    let ascii_part: String = chunk
                        .iter()
                        .map(|&b| if b.is_ascii_graphic() { b as char } else { '.' })
                        .collect();

                    Line::from(vec![
                        Span::styled(offset, Style::default().fg(COLOR_GRAY)),
                        Span::styled(format!("{hex_part}{padding} "), Style::default().fg(COLOR_CYAN)),
                        Span::styled(format!("│ {ascii_part}"), Style::default().fg(COLOR_WHITE)),
                    ])
                })
                .collect()
        }
    };

    let size_info = body_bytes.map(|b| format!(" ({} octets)", b.len())).unwrap_or_default();

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(
                    format!(" Hex Viewer{size_info} "),
                    Style::default().fg(COLOR_ACCENT),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_BORDER))
                .style(Style::default().bg(COLOR_PANEL)),
        );

    f.render_widget(para, area);
}

fn render_status_bar<B: Backend>(f: &mut Frame<B>, area: Rect, app: &AppState) {
    let msg = app.status_message.as_deref().unwrap_or(
        "[j/k] Naviguer  [Enter] Détail  [a] Actions  [r] Replay  [p] Pause  [/] Filtre  [q] Quitter",
    );

    let status = Paragraph::new(msg)
        .style(Style::default().fg(COLOR_GRAY).bg(Color::Rgb(14, 16, 28)));

    f.render_widget(status, area);
}

fn render_action_popup<B: Backend>(f: &mut Frame<B>, area: Rect, app: &AppState) {
    let popup_width  = 50u16;
    let popup_height = (app.popup_items.len() as u16) + 4;
    let popup_area   = centered_rect(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = app.popup_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.popup_selected {
                Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD).bg(COLOR_SELECTED)
            } else {
                Style::default().fg(COLOR_WHITE)
            };
            ListItem::new(format!("  {item}  ")).style(style)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.popup_selected));

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(
                    " Actions ",
                    Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_ACCENT))
                .style(Style::default().bg(Color::Rgb(14, 18, 32))),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, popup_area, &mut list_state);
}

fn render_cookie_editor<B: Backend>(f: &mut Frame<B>, area: Rect, app: &AppState) {
    let popup_width  = 70u16;
    let popup_height = (app.cookie_rows.len().max(3) as u16) + 6;
    let popup_area   = centered_rect(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let rows: Vec<Row> = app.cookie_rows
        .iter()
        .enumerate()
        .map(|(i, (k, v))| {
            let is_sel = i == app.cookie_selected;
            let val = if let Some((_, ref buf)) = app.cookie_editing {
                if is_sel { format!("{buf}_") } else { v.clone() }
            } else {
                v.clone()
            };
            let style = if is_sel {
                Style::default().bg(COLOR_SELECTED).fg(COLOR_ACCENT)
            } else {
                Style::default().fg(COLOR_WHITE)
            };
            Row::new(vec![
                Cell::from(k.clone()).style(Style::default().fg(COLOR_CYAN)),
                Cell::from(val).style(style),
            ])
        })
        .collect();

    let table = Table::new(rows)
        .header(
            Row::new(vec!["Cookie Name", "Valeur"])
                .style(Style::default().fg(COLOR_GRAY).add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(Span::styled(
                    " 🍪 Cookie Editor — [Enter] Éditer  [Tab] Confirmer  [Esc] Fermer ",
                    Style::default().fg(COLOR_YELLOW).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_YELLOW))
                .style(Style::default().bg(Color::Rgb(18, 16, 10))),
        )
        .widths(&[Constraint::Percentage(35), Constraint::Percentage(65)]);

    f.render_widget(table, popup_area);
}

fn render_confirm_popup<B: Backend>(f: &mut Frame<B>, area: Rect) {
    let popup_area = centered_rect(40, 5, area);
    f.render_widget(Clear, popup_area);

    let text = Paragraph::new("\n  Confirmer le replay ? [y] Oui  [n] Non")
        .style(Style::default().fg(COLOR_WHITE))
        .block(
            Block::default()
                .title(Span::styled(" Replay ", Style::default().fg(COLOR_CYAN)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_CYAN))
                .style(Style::default().bg(COLOR_PANEL)),
        );

    f.render_widget(text, popup_area);
}

/// Calcule une zone centrée dans `r`
fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width:  width.min(r.width),
        height: height.min(r.height),
    }
}
