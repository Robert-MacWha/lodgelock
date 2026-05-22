mod page;
mod render;

use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use host::{
    host::{Host, UserRequest},
    host_state::PluginSource,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tlock_hdk::tlock_api::{
    entities::{EntityId, PageId},
    page::PageEvent,
};

use page::PageItem;
use render::{page_ids_in_root, root_items, selectable_root_indices};

const PLUGINS_DIR: &str = "./tui-plugins";

pub enum Screen {
    Root,
    Events,
    Requests,
    FulfillRequest { request: UserRequest },
    BrowsePlugins { files: Vec<PathBuf> },
    LoadingPlugin { path: PathBuf, name: String },
    Page(PageId),
}

pub struct App {
    pub host: Arc<Host>,
    pub screen: Screen,
    pub cursor: usize,
    pub page_items: Vec<PageItem>,
    // form_id → { field_id → value }
    pub form_values: HashMap<String, HashMap<String, String>>,
    // dropdown cycling index (item index in page_items → selected option index)
    pub dropdown_indices: HashMap<usize, usize>,
    pub input_mode: bool,
    pub status: Option<String>,
    pub should_quit: bool,
    pub pending_page_refresh: Arc<Mutex<Option<PageId>>>,
}

impl App {
    fn new(host: Arc<Host>) -> Self {
        Self {
            host,
            screen: Screen::Root,
            cursor: 0,
            page_items: Vec::new(),
            form_values: HashMap::new(),
            dropdown_indices: HashMap::new(),
            input_mode: false,
            status: None,
            should_quit: false,
            pending_page_refresh: Arc::new(Mutex::new(None)),
        }
    }

    fn cursor_down(&mut self) {
        match &self.screen {
            Screen::FulfillRequest { request } => {
                let count = candidate_entities(&self.host, request).len() + 1;
                if count > 0 { self.cursor = (self.cursor + 1) % count; }
            }
            Screen::BrowsePlugins { files } => {
                let count = files.len();
                if count > 0 {
                    self.cursor = (self.cursor + 1) % count;
                }
            }
            Screen::Root => {
                let indices = selectable_root_indices(self);
                if let Some(pos) = indices.iter().position(|&i| i >= self.cursor) {
                    let next = if pos + 1 < indices.len() {
                        indices[pos + 1]
                    } else {
                        indices[0]
                    };
                    self.cursor = next;
                }
            }
            Screen::Page(_) => {
                let len = self.page_items.len();
                if len == 0 {
                    return;
                }
                let mut next = (self.cursor + 1) % len;
                while !self.page_items[next].is_interactive() && next != self.cursor {
                    next = (next + 1) % len;
                }
                self.cursor = next;
            }
            Screen::Requests => {
                let count = self.host.get_user_requests().len();
                if count > 0 {
                    self.cursor = (self.cursor + 1) % count;
                }
            }
            _ => {}
        }
    }

    fn cursor_up(&mut self) {
        match &self.screen {
            Screen::FulfillRequest { request } => {
                let count = candidate_entities(&self.host, request).len() + 1;
                if count > 0 {
                    self.cursor = if self.cursor == 0 { count - 1 } else { self.cursor - 1 };
                }
            }
            Screen::BrowsePlugins { files } => {
                let count = files.len();
                if count > 0 {
                    self.cursor = if self.cursor == 0 {
                        count - 1
                    } else {
                        self.cursor - 1
                    };
                }
            }
            Screen::Root => {
                let indices = selectable_root_indices(self);
                if let Some(pos) = indices.iter().rposition(|&i| i <= self.cursor) {
                    let prev = if pos > 0 {
                        indices[pos - 1]
                    } else {
                        *indices.last().unwrap_or(&0)
                    };
                    self.cursor = prev;
                }
            }
            Screen::Page(_) => {
                let len = self.page_items.len();
                if len == 0 {
                    return;
                }
                let mut prev = if self.cursor == 0 {
                    len - 1
                } else {
                    self.cursor - 1
                };
                while !self.page_items[prev].is_interactive() && prev != self.cursor {
                    prev = if prev == 0 { len - 1 } else { prev - 1 };
                }
                self.cursor = prev;
            }
            Screen::Requests => {
                let count = self.host.get_user_requests().len();
                if count > 0 {
                    self.cursor = if self.cursor == 0 {
                        count - 1
                    } else {
                        self.cursor - 1
                    };
                }
            }
            _ => {}
        }
    }

    fn go_back(&mut self) {
        self.screen = Screen::Root;
        self.cursor = 0;
        self.page_items.clear();
        self.form_values.clear();
        self.dropdown_indices.clear();
        self.input_mode = false;
        self.status = None;
    }

    fn refresh_page_items(&mut self, page_id: PageId) {
        self.page_items = self
            .host
            .get_interface(page_id)
            .map(|c| page::flatten(&c))
            .unwrap_or_default();

        // Sync dropdown_indices with current form_values
        for (i, item) in self.page_items.iter().enumerate() {
            if let PageItem::Dropdown {
                options,
                id,
                form_id,
                ..
            } = item
            {
                let selected_val = self
                    .form_values
                    .get(form_id.as_str())
                    .and_then(|m| m.get(id.as_str()))
                    .cloned();
                let idx = selected_val
                    .and_then(|v| options.iter().position(|o| o == &v))
                    .unwrap_or(0);
                self.dropdown_indices.insert(i, idx);
            }
        }

        // Move cursor to the first interactive item
        if let Some(first) = self.page_items.iter().position(|i| i.is_interactive()) {
            self.cursor = first;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let log_file = std::fs::File::create("tlock-tui.log")?;
    tracing_subscriber::fmt()
        .with_writer(log_file)
        .with_ansi(false)
        .init();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let host = Arc::new(Host::new());
    let mut app = App::new(host);

    let result = run(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    result
}

async fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| render::render(f, app))?;

        for req in app.host.get_user_requests() {
            let candidates = candidate_entities(&app.host, &req);
            if candidates.len() == 1 {
                resolve_request(&app.host, &req, &candidates[0]);
            }
        }

        let pending = app.pending_page_refresh.lock().unwrap().take();
        if let Some(page_id) = pending {
            if let Screen::Page(current_id) = app.screen {
                if current_id == page_id {
                    app.refresh_page_items(page_id);
                }
            }
        }

        // After rendering the loading screen, do the actual work
        if let Screen::LoadingPlugin { .. } = &app.screen {
            do_load_plugin(app).await;
            continue;
        }

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                handle_key(app, key.code, key.modifiers).await;
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

async fn do_load_plugin(app: &mut App) {
    let Screen::LoadingPlugin { path, name } = &app.screen else {
        return;
    };
    let (path, name) = (path.clone(), name.clone());
    let host = app.host.clone();

    // Return to root immediately so the user can fulfill any requests the
    // plugin's Init makes (vault selection, eth provider, etc.).
    app.go_back();

    tokio::spawn(async move {
        match std::fs::read(&path) {
            Err(e) => tracing::error!("read {path:?}: {e}"),
            Ok(bytes) => {
                if let Err(e) = host.new_plugin(PluginSource::Embedded(bytes), &name).await {
                    tracing::error!("load '{name}': {e}");
                }
            }
        }
    });
}

async fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Ctrl+C always quits
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        app.should_quit = true;
        return;
    }

    match &app.screen {
        Screen::Root => handle_root(app, code).await,
        Screen::Events => handle_back_screen(app, code),
        Screen::Requests => handle_requests(app, code),
        Screen::FulfillRequest { .. } => handle_fulfill_request(app, code),
        Screen::BrowsePlugins { .. } => handle_browse_plugins(app, code).await,
        Screen::LoadingPlugin { .. } => {} // handled in the run loop after rendering
        Screen::Page(_) => {
            if app.input_mode {
                handle_text_input(app, code).await;
            } else {
                handle_page(app, code).await;
            }
        }
    }
}

async fn handle_root(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('j') | KeyCode::Down => app.cursor_down(),
        KeyCode::Char('k') | KeyCode::Up => app.cursor_up(),
        KeyCode::Enter => {
            let items = root_items(app);
            let Some(label) = items.get(app.cursor) else {
                return;
            };

            if label == "Load Plugin" {
                let files = scan_plugins_dir(Path::new(PLUGINS_DIR));
                app.screen = Screen::BrowsePlugins { files };
                app.cursor = 0;
                return;
            }
            if label.starts_with("Events") {
                app.screen = Screen::Events;
                return;
            }
            if label.starts_with("Requests") {
                app.screen = Screen::Requests;
                app.cursor = 0;
                return;
            }

            // Otherwise it's a page row — compute which page_id it maps to
            let page_ids = page_ids_in_root(app);
            // The pages start after the 3 fixed items + 1 separator = index 4+
            let page_start = if page_ids.is_empty() { usize::MAX } else { 4 };
            if app.cursor >= page_start {
                let idx = app.cursor - page_start;
                if let Some(&page_id) = page_ids.get(idx) {
                    let page_id_clone = page_id;
                    app.screen = Screen::Page(page_id);
                    app.cursor = 0;
                    let host = app.host.clone();
                    let refresh = app.pending_page_refresh.clone();
                    tokio::spawn(async move {
                        if let Err(e) = host.page_on_load(page_id_clone).await {
                            tracing::error!("page_on_load: {e}");
                        }
                        *refresh.lock().unwrap() = Some(page_id_clone);
                    });
                }
            }
        }
        _ => {}
    }
}

fn handle_back_screen(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => app.go_back(),
        _ => {}
    }
}

fn handle_requests(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => app.go_back(),
        KeyCode::Char('j') | KeyCode::Down => app.cursor_down(),
        KeyCode::Char('k') | KeyCode::Up => app.cursor_up(),
        KeyCode::Enter => {
            let requests = app.host.get_user_requests();
            if let Some(req) = requests.get(app.cursor).cloned() {
                app.screen = Screen::FulfillRequest { request: req };
                app.cursor = 0;
            }
        }
        _ => {}
    }
}

fn resolve_request(host: &Host, request: &UserRequest, entity: &EntityId) {
    match (request, entity) {
        (UserRequest::EthProviderSelection { id, .. }, EntityId::EthProvider(provider_id)) => {
            host.resolve_eth_provider_request(*id, *provider_id);
        }
        (UserRequest::VaultSelection { id, .. }, EntityId::Vault(vault_id)) => {
            host.resolve_vault_request(*id, *vault_id);
        }
        (UserRequest::CoordinatorSelection { id, .. }, EntityId::Coordinator(coord_id)) => {
            host.resolve_coordinator_request(*id, *coord_id);
        }
        _ => {}
    }
}

fn candidate_entities(host: &Host, request: &UserRequest) -> Vec<EntityId> {
    host.get_entities()
        .into_iter()
        .filter(|e| match request {
            UserRequest::EthProviderSelection { .. } => matches!(e, EntityId::EthProvider(_)),
            UserRequest::VaultSelection { .. } => matches!(e, EntityId::Vault(_)),
            UserRequest::CoordinatorSelection { .. } => matches!(e, EntityId::Coordinator(_)),
        })
        .collect()
}

fn handle_fulfill_request(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::Requests;
            app.cursor = 0;
            return;
        }
        KeyCode::Char('j') | KeyCode::Down => { app.cursor_down(); return; }
        KeyCode::Char('k') | KeyCode::Up => { app.cursor_up(); return; }
        KeyCode::Enter => {}
        _ => return,
    }

    let Screen::FulfillRequest { request } = &app.screen else { return };
    let request = request.clone();
    let candidates = candidate_entities(&app.host, &request);
    let cursor = app.cursor;

    if cursor < candidates.len() {
        resolve_request(&app.host, &request, &candidates[cursor]);
    } else {
        // Deny row
        app.host.deny_user_request(request.id());
    }

    app.go_back();
}

fn scan_plugins_dir(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("wasm"))
        .collect();
    files.sort();
    files
}

async fn handle_browse_plugins(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.go_back();
            return;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.cursor_down();
            return;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.cursor_up();
            return;
        }
        KeyCode::Enter => {}
        _ => return,
    }

    let Screen::BrowsePlugins { files } = &app.screen else {
        return;
    };
    let Some(path) = files.get(app.cursor).cloned() else {
        return;
    };

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("plugin")
        .to_string();

    app.screen = Screen::LoadingPlugin { path, name };
}

async fn handle_page(app: &mut App, code: KeyCode) {
    let page_id = match &app.screen {
        Screen::Page(id) => *id,
        _ => return,
    };

    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.go_back();
            return;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.cursor_down();
            return;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.cursor_up();
            return;
        }
        KeyCode::Left | KeyCode::Right => {
            let cursor = app.cursor;
            if let Some(PageItem::Dropdown {
                options,
                id,
                form_id,
                ..
            }) = app.page_items.get(cursor).cloned()
            {
                if options.is_empty() {
                    return;
                }
                let idx = app.dropdown_indices.entry(cursor).or_insert(0);
                if code == KeyCode::Right {
                    *idx = (*idx + 1) % options.len();
                } else {
                    *idx = if *idx == 0 {
                        options.len() - 1
                    } else {
                        *idx - 1
                    };
                }
                let selected = options[*idx].clone();
                app.form_values
                    .entry(form_id)
                    .or_default()
                    .insert(id, selected);
            }
            return;
        }
        KeyCode::Enter => {}
        _ => return,
    }

    // Enter key
    let cursor = app.cursor;
    let item = match app.page_items.get(cursor).cloned() {
        Some(item) => item,
        None => return,
    };

    match item {
        PageItem::Button { id, .. } => {
            let event = PageEvent::ButtonClicked(id);
            let host = app.host.clone();
            let refresh = app.pending_page_refresh.clone();
            tokio::spawn(async move {
                if let Err(e) = host.page_on_update((page_id, event)).await {
                    tracing::error!("page_on_update: {e}");
                }
                *refresh.lock().unwrap() = Some(page_id);
            });
        }
        PageItem::Input { .. } => {
            app.input_mode = true;
        }
        PageItem::Dropdown {
            options,
            id,
            form_id,
            ..
        } => {
            if options.is_empty() {
                return;
            }
            let idx = app.dropdown_indices.entry(cursor).or_insert(0);
            *idx = (*idx + 1) % options.len();
            let selected = options[*idx].clone();
            app.form_values
                .entry(form_id)
                .or_default()
                .insert(id, selected);
        }
        PageItem::Submit { form_id, .. } => {
            let values = app.form_values.get(&form_id).cloned().unwrap_or_default();
            let event = PageEvent::FormSubmitted(form_id, values);
            let host = app.host.clone();
            let refresh = app.pending_page_refresh.clone();
            tokio::spawn(async move {
                if let Err(e) = host.page_on_update((page_id, event)).await {
                    tracing::error!("page_on_update: {e}");
                }
                *refresh.lock().unwrap() = Some(page_id);
            });
        }
        _ => {}
    }
}

async fn handle_text_input(app: &mut App, code: KeyCode) {
    let page_id = match &app.screen {
        Screen::Page(id) => *id,
        _ => return,
    };

    let cursor = app.cursor;
    let item = match app.page_items.get(cursor).cloned() {
        Some(item) => item,
        None => {
            app.input_mode = false;
            return;
        }
    };

    let PageItem::Input { id, form_id, .. } = item else {
        app.input_mode = false;
        return;
    };

    match code {
        KeyCode::Esc => {
            app.input_mode = false;
        }
        KeyCode::Enter => {
            app.input_mode = false;
            let _ = page_id; // value committed to form_values; no event fired until Submit
        }
        KeyCode::Backspace => {
            let val = app
                .form_values
                .entry(form_id)
                .or_default()
                .entry(id)
                .or_default();
            val.pop();
        }
        KeyCode::Char(c) => {
            let val = app
                .form_values
                .entry(form_id)
                .or_default()
                .entry(id)
                .or_default();
            val.push(c);
        }
        _ => {}
    }
}
