use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use std::io::Stdout;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::provider::{BoxedProvider, Package, SearchResult};
use crate::ui;

pub enum SearchState {
    Idle,
    Searching,
    Done,
}

pub enum AppAction {
    Quit,
    Install,
}

pub struct App {
    pub query: String,
    pub packages: Vec<Package>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub provider: Arc<BoxedProvider>,
    pub search_state: SearchState,
    pub status_message: String,
    last_input: Instant,
    query_changed: bool,
    generation: u64,
    result_rx: mpsc::Receiver<(u64, SearchResult)>,
    result_tx: mpsc::Sender<(u64, SearchResult)>,
}

impl App {
    pub fn new(provider: BoxedProvider) -> Self {
        let (tx, rx) = mpsc::channel();
        App {
            query: String::new(),
            packages: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            provider: Arc::new(provider),
            search_state: SearchState::Idle,
            status_message: "Start typing to search.".to_string(),
            last_input: Instant::now(),
            query_changed: false,
            generation: 0,
            result_rx: rx,
            result_tx: tx,
        }
    }

    /// Main event loop. Returns when the user quits or selects a package to install.
    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> std::io::Result<AppAction> {
        loop {
            terminal.draw(|f| ui::render(f, self))?;

            if event::poll(Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    if let Some(action) = self.handle_key(key.code, key.modifiers) {
                        return Ok(action);
                    }
                }
            }

            self.tick();
        }
    }

    fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> Option<AppAction> {
        let ctrl = mods.contains(KeyModifiers::CONTROL);
        let no_meta = !mods.contains(KeyModifiers::CONTROL) && !mods.contains(KeyModifiers::ALT);

        match code {
            KeyCode::Char('c') | KeyCode::Char('q') | KeyCode::Char('x') if ctrl => {
                return Some(AppAction::Quit);
            }

            KeyCode::Esc => {
                self.query.clear();
                self.packages.clear();
                self.selected = 0;
                self.scroll_offset = 0;
                self.status_message = "Start typing to search.".to_string();
                self.search_state = SearchState::Idle;
                self.query_changed = false;
                self.generation += 1; // invalidate any in-flight search
            }

            KeyCode::Enter => {
                if !self.packages.is_empty() && self.selected < self.packages.len() {
                    return Some(AppAction::Install);
                }
            }

            KeyCode::Up => self.navigate(-1),
            KeyCode::Down => self.navigate(1),

            KeyCode::PageUp => {
                let page = self.get_visible_count().max(1) as i32;
                self.navigate(-page);
            }
            KeyCode::PageDown => {
                let page = self.get_visible_count().max(1) as i32;
                self.navigate(page);
            }

            KeyCode::Home => {
                self.selected = 0;
                self.scroll_offset = 0;
            }

            KeyCode::End => {
                if !self.packages.is_empty() {
                    self.selected = self.packages.len() - 1;
                    let visible = self.get_visible_count().max(1);
                    self.scroll_offset = self.packages.len().saturating_sub(visible);
                    self.adjust_scroll();
                }
            }

            KeyCode::Backspace => {
                if !self.query.is_empty() {
                    self.query.pop();
                    self.query_changed = true;
                    self.last_input = Instant::now();
                    if !self.query.is_empty() {
                        self.search_state = SearchState::Searching;
                        self.status_message = "Searching...".to_string();
                    }
                }
            }

            KeyCode::Char(c) if no_meta => {
                self.query.push(c);
                self.query_changed = true;
                self.last_input = Instant::now();
                self.search_state = SearchState::Searching;
                self.status_message = "Searching...".to_string();
            }

            _ => {}
        }

        None
    }

    fn navigate(&mut self, delta: i32) {
        if self.packages.is_empty() {
            return;
        }
        let n = self.packages.len() as i32;
        let new_sel = (self.selected as i32 + delta).clamp(0, n - 1) as usize;
        self.selected = new_sel;
        self.adjust_scroll();
    }

    fn adjust_scroll(&mut self) {
        let visible = self.get_visible_count().max(1);
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible {
            self.scroll_offset = self.selected - visible + 1;
        }
    }

    /// Returns the number of packages visible based on current terminal height.
    pub fn get_visible_count(&self) -> usize {
        let (_, height) = crossterm::terminal::size().unwrap_or((80, 24));
        // Layout: results area = height - 1 (status) - 3 (search block with borders)
        let results_height = (height as usize).saturating_sub(4);
        results_height / 2 // 2 lines per package
    }

    fn tick(&mut self) {
        // Drain the results channel, accept only the latest generation
        while let Ok((search_gen, result)) = self.result_rx.try_recv() {
            if search_gen == self.generation {
                self.packages = result.packages;
                self.selected = 0;
                self.scroll_offset = 0;
                if let Some(err) = result.error {
                    self.status_message = format!("Error: {err}");
                } else if self.packages.is_empty() {
                    self.status_message = "No results found.".to_string();
                } else {
                    let n = self.packages.len();
                    self.status_message = format!(
                        "Found {n} result{}.",
                        if n == 1 { "" } else { "s" }
                    );
                }
                self.search_state = SearchState::Done;
            }
        }

        // Fire a search after 400 ms debounce
        if self.query_changed && self.last_input.elapsed() >= Duration::from_millis(400) {
            let query = self.query.clone();
            if query.is_empty() {
                self.packages.clear();
                self.selected = 0;
                self.scroll_offset = 0;
                self.status_message = "Start typing to search.".to_string();
                self.search_state = SearchState::Idle;
                self.generation += 1;
            } else {
                self.generation += 1;
                self.search_state = SearchState::Searching;
                self.status_message = "Searching...".to_string();
                self.spawn_search(query);
            }
            self.query_changed = false;
        }
    }

    fn spawn_search(&self, query: String) {
        let search_gen = self.generation;
        let tx = self.result_tx.clone();
        let provider = Arc::clone(&self.provider);
        thread::spawn(move || {
            let result = provider.search(&query);
            tx.send((search_gen, result)).ok();
        });
    }
}
