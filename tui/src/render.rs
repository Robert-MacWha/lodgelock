use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::{candidate_entities, page::PageItem, App, Screen};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Split into main area + status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    match &app.screen {
        Screen::Root => render_root(frame, app, chunks[0]),
        Screen::Events => render_events(frame, app, chunks[0]),
        Screen::Requests => render_requests(frame, app, chunks[0]),
        Screen::FulfillRequest { request } => render_fulfill_request(frame, app, request, chunks[0]),
        Screen::BrowsePlugins { .. } => render_browse_plugins(frame, app, chunks[0]),
        Screen::LoadingPlugin { name, .. } => render_loading(frame, name, chunks[0]),
        Screen::Page(_) => render_page(frame, app, chunks[0]),
    }

    render_status(frame, app, chunks[1]);
}

fn render_root(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items = root_items(app);
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let style = if i == app.cursor {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if text.starts_with("───") {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            ListItem::new(text.as_str()).style(style)
        })
        .collect();

    let block = Block::default().borders(Borders::ALL).title(" tlock ");
    frame.render_widget(List::new(list_items).block(block), area);
}

fn render_page(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .page_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == app.cursor && item.is_interactive();
            let is_editing = is_selected && app.input_mode;

            match item {
                PageItem::Label(text) => {
                    ListItem::new(text.as_str()).style(Style::default().fg(Color::DarkGray))
                }
                PageItem::Button { text, .. } => {
                    let style = if is_selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!("[ {text} ]")).style(style)
                }
                PageItem::Input { label, id, form_id } => {
                    let value = app
                        .form_values
                        .get(form_id.as_str())
                        .and_then(|m| m.get(id.as_str()))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    let display = if is_editing {
                        format!("{label}: {value}▌")
                    } else if is_selected {
                        format!("{label}: [{value}]")
                    } else {
                        format!("{label}: {value}")
                    };
                    let style = if is_selected {
                        Style::default().fg(Color::Black).bg(Color::White)
                    } else {
                        Style::default()
                    };
                    ListItem::new(display).style(style)
                }
                PageItem::Dropdown {
                    label,
                    options,
                    id,
                    form_id,
                } => {
                    let selected_val = app
                        .form_values
                        .get(form_id.as_str())
                        .and_then(|m| m.get(id.as_str()))
                        .and_then(|v| options.iter().find(|o| *o == v))
                        .map(|s| s.as_str())
                        .or_else(|| options.first().map(|s| s.as_str()))
                        .unwrap_or("—");
                    let display = format!("{label}: < {selected_val} >");
                    let style = if is_selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(display).style(style)
                }
                PageItem::Submit { text, .. } => {
                    let style = if is_selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Green)
                    };
                    ListItem::new(format!("[ {text} ]")).style(style)
                }
            }
        })
        .collect();

    let title = match &app.screen {
        Screen::Page(id) => format!(" Page: {id} "),
        _ => " Page ".to_string(),
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    frame.render_widget(List::new(items).block(block), area);
}

fn render_events(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let events = app.host.get_events();
    let text: Vec<Line> = events
        .iter()
        .map(|e| {
            let plugin = e.plugin.as_deref().unwrap_or("host");
            Line::from(vec![
                Span::styled(
                    format!("[{}] ", e.timestamp.format("%H:%M:%S")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("{plugin}: "), Style::default().fg(Color::Cyan)),
                Span::raw(e.message.clone()),
            ])
        })
        .collect();

    let block = Block::default().borders(Borders::ALL).title(" Events ");
    let para = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn render_requests(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let requests = app.host.get_user_requests();
    let items: Vec<ListItem> = requests
        .iter()
        .enumerate()
        .map(|(i, req)| {
            use host::host::UserRequest;
            let label = match req {
                UserRequest::EthProviderSelection { chain_id, .. } => {
                    format!("EthProvider request for chain {chain_id}")
                }
                UserRequest::VaultSelection { .. } => "Vault selection request".to_string(),
                UserRequest::CoordinatorSelection { .. } => {
                    "Coordinator selection request".to_string()
                }
            };
            let style = if i == app.cursor {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(label).style(style)
        })
        .collect();

    let block = Block::default().borders(Borders::ALL).title(" Requests  (Enter to fulfill) ");
    frame.render_widget(List::new(items).block(block), area);
}

fn render_fulfill_request(
    frame: &mut Frame,
    app: &App,
    request: &host::host::UserRequest,
    area: ratatui::layout::Rect,
) {
    use host::host::UserRequest;

    let plugin_name = app
        .host
        .get_plugin(&request.plugin_id())
        .map(|p| p.name().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let (title, kind) = match request {
        UserRequest::EthProviderSelection { chain_id, .. } => (
            " Fulfill: EthProvider Request ",
            format!("Plugin '{plugin_name}' needs an EthProvider for chain {chain_id}"),
        ),
        UserRequest::VaultSelection { .. } => (
            " Fulfill: Vault Request ",
            format!("Plugin '{plugin_name}' needs a Vault"),
        ),
        UserRequest::CoordinatorSelection { .. } => (
            " Fulfill: Coordinator Request ",
            format!("Plugin '{plugin_name}' needs a Coordinator"),
        ),
    };

    let candidates = candidate_entities(&app.host, request);

    let mut items: Vec<ListItem> = candidates
        .iter()
        .enumerate()
        .map(|(i, entity_id)| {
            let owner = app
                .host
                .get_entity_plugin(*entity_id)
                .map(|p| p.name().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let label = format!("{entity_id}  (plugin: {owner})");
            let style = if i == app.cursor {
                Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(label).style(style)
        })
        .collect();

    // Deny row
    let deny_idx = candidates.len();
    let deny_style = if app.cursor == deny_idx {
        Style::default().fg(Color::White).bg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };
    items.push(ListItem::new("[ Deny ]").style(deny_style));

    // Description label at top
    let mut all_items = vec![ListItem::new(kind).style(Style::default().fg(Color::DarkGray))];
    all_items.extend(items);

    let block = Block::default().borders(Borders::ALL).title(title);
    frame.render_widget(List::new(all_items).block(block), area);
}

fn render_browse_plugins(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let Screen::BrowsePlugins { files } = &app.screen else {
        return;
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Load Plugin  ({})", crate::PLUGINS_DIR));

    if files.is_empty() {
        let msg = format!("No .wasm files found in {}", crate::PLUGINS_DIR);
        frame.render_widget(
            Paragraph::new(msg)
                .block(block)
                .style(Style::default().fg(Color::DarkGray)),
            area,
        );
        return;
    }

    let items: Vec<ListItem> = files
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
            let style = if i == app.cursor {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(name).style(style)
        })
        .collect();

    frame.render_widget(List::new(items).block(block), area);
}

fn render_loading(frame: &mut Frame, name: &str, area: ratatui::layout::Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Loading ");
    frame.render_widget(
        Paragraph::new(format!("Loading '{name}'…"))
            .block(block)
            .style(Style::default()),
        area,
    );
}

fn render_status(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let help = if app.input_mode {
        "  Enter: confirm  Esc: cancel"
    } else {
        match &app.screen {
            Screen::Root => "  j/k: navigate  Enter: select  q: quit",
            Screen::Page(_) => "  j/k: navigate  Enter: activate  ←/→: cycle dropdown  Esc: back",
            Screen::Events | Screen::Requests => "  Esc/q: back",
            Screen::FulfillRequest { .. } => "  j/k: navigate  Enter: select  Esc: back",
            Screen::BrowsePlugins { .. } => "  j/k: navigate  Enter: load  Esc: cancel",
            Screen::LoadingPlugin { .. } => "",
        }
    };

    let text = if let Some(status) = &app.status {
        format!("{status}  {help}")
    } else {
        help.to_string()
    };

    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}

pub fn root_items(app: &App) -> Vec<String> {
    let event_count = app.host.get_events().len();
    let request_count = app.host.get_user_requests().len();

    let mut items = vec![
        "Load Plugin".to_string(),
        format!("Events ({event_count})"),
        format!("Requests ({request_count})"),
    ];

    let pages = app.host.get_entities();
    let page_ids: Vec<_> = pages
        .iter()
        .filter_map(|e| match e {
            tlock_hdk::tlock_api::entities::EntityId::Page(id) => Some(*id),
            _ => None,
        })
        .collect();

    if !page_ids.is_empty() {
        items.push("─── Pages ───────────────────────────────────".to_string());
        for page_id in page_ids {
            let plugin_name = app
                .host
                .get_entity_plugin(page_id)
                .map(|p| p.name().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            items.push(format!("{plugin_name}"));
        }
    }

    items
}

pub fn selectable_root_indices(app: &App) -> Vec<usize> {
    let items = root_items(app);
    items
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.starts_with("───"))
        .map(|(i, _)| i)
        .collect()
}

pub fn page_ids_in_root(app: &App) -> Vec<tlock_hdk::tlock_api::entities::PageId> {
    app.host
        .get_entities()
        .iter()
        .filter_map(|e| match e {
            tlock_hdk::tlock_api::entities::EntityId::Page(id) => Some(*id),
            _ => None,
        })
        .collect()
}
