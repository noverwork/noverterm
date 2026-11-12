//! A terminal tab: an alacritty grid fed by a backend session and painted
//! straight onto a GPUI canvas.

mod images;
pub mod keys;
mod paint;

use std::collections::HashMap;
use std::ops::Range;
use std::sync::{mpsc, Arc};
use std::time::Duration;

use alacritty_terminal::event::{Event as TermEvent, EventListener, WindowSize};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Boundary, Column, Direction, Line, Point as GridPoint, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::search::{Match, RegexIter, RegexSearch};
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::{CursorShape, CursorStyle, Processor};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{h_flex, Disableable as _, Icon, IconName, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use crate::app::AppHandle;
use crate::runtime::local::LocalSessionManager;
use crate::runtime::ssh::SshSessionManager;
use crate::ui::bridge::backend;
use crate::ui::i18n::t;
use crate::ui::theme;

/// The terminal font ships with the app so CJK text lines up on every platform.
pub const FONT_FAMILY: &str = "Sarasa Term TC SemiBold";
const BLINK_INTERVAL: Duration = Duration::from_millis(600);
/// Space between the terminal area's edges and the grid, on every side.
const PADDING: Pixels = px(14.);
/// Marks the cells an inline image covers, so the image scrolls with them.
const IMAGE_URI: &str = "noverterm-image:";
/// Older images are forgotten past this many.
// ponytail: a fixed cap; track which ones are still in the scrollback if this bites.
const MAX_IMAGES: usize = 64;

/// What the previous front end (xterm.js' link addon) recognised as a URL.
const URL_PATTERN: &str = r#"(https?|HTTPS?)://[^\s"'!*(){}|\\^<>`]*[^\s"':,.!?{}|\\^~\[\]`()<>]"#;

// The window binds Tab to focus traversal and Copy to the platform copy key;
// inside a terminal both belong to the terminal, so the more specific
// "Terminal" context claims them back.
actions!(terminal, [SendTab, SendBackTab, Copy]);

pub fn bind_keys(cx: &mut App) {
    let copy = if cfg!(target_os = "macos") {
        "cmd-c"
    } else {
        "ctrl-c"
    };
    cx.bind_keys([
        KeyBinding::new("tab", SendTab, Some("Terminal")),
        KeyBinding::new("shift-tab", SendBackTab, Some("Terminal")),
        KeyBinding::new(copy, Copy, Some("Terminal")),
    ]);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    Ssh,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CursorKind {
    Block,
    Underline,
    Bar,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerminalConfig {
    pub font_size: f32,
    pub cursor_style: CursorKind,
    pub cursor_blink: bool,
    pub scrollback: usize,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            font_size: 14.,
            cursor_style: CursorKind::Block,
            cursor_blink: true,
            scrollback: 5000,
        }
    }
}

impl TerminalConfig {
    fn term_config(&self) -> Config {
        Config {
            scrolling_history: self.scrollback,
            default_cursor_style: CursorStyle {
                shape: match self.cursor_style {
                    CursorKind::Block => CursorShape::Block,
                    CursorKind::Underline => CursorShape::Underline,
                    CursorKind::Bar => CursorShape::Beam,
                },
                blinking: self.cursor_blink,
            },
            kitty_keyboard: true,
            ..Config::default()
        }
    }
}

pub enum TerminalEvent {
    /// The grid was fitted to a new size; every other tab should match it so
    /// switching tabs never reflows the screen in front of the user.
    Resized { cols: usize, rows: usize },
    /// Writing to the session failed.
    Error(String),
    /// Cmd+W while the terminal has focus.
    RequestClose,
    /// Cmd+K: open the AI assistant for this terminal.
    RequestAssistant,
}

/// An inline image the program drew, keyed by the id its marker cells carry.
struct PlacedImage {
    image: Arc<RenderImage>,
    /// One image pixel of transparent border on each side.
    margin: Size<Pixels>,
    size: Size<Pixels>,
}

#[derive(Clone)]
struct EventProxy(mpsc::Sender<TermEvent>);

impl EventListener for EventProxy {
    fn send_event(&self, event: TermEvent) {
        let _ = self.0.send(event);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct GridSize {
    cols: usize,
    rows: usize,
}

impl Dimensions for GridSize {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

pub struct TerminalView {
    session_id: String,
    kind: SessionKind,
    app: AppHandle,
    term: Term<EventProxy>,
    parser: Processor,
    events: mpsc::Receiver<TermEvent>,
    input: UnboundedSender<Vec<u8>>,
    focus: FocusHandle,
    config: TerminalConfig,
    grid: GridSize,
    /// Where and how big the grid was last painted, for mouse hit testing.
    bounds: Bounds<Pixels>,
    cell: Size<Pixels>,
    /// Text the input method is still composing.
    marked_text: Option<String>,
    selecting: bool,
    dragged: bool,
    /// Mouse button currently reported to the program as held.
    reported_button: Option<u8>,
    /// Cell of the last reported mouse motion, so moves within a cell stay quiet.
    reported_motion: Option<GridPoint>,
    /// Where the pointer is over the terminal, to keep the hovered link current.
    pointer: Option<(Point<Pixels>, Modifiers)>,
    /// The link under the pointer and the cells it covers.
    hovered_link: Option<(String, Match)>,
    /// Whether the program was last told the terminal has focus.
    focus_reported: bool,
    /// Ends a synchronized update the program never finished.
    sync_flush: Option<Task<()>>,
    scanner: images::Scanner,
    images: HashMap<u32, PlacedImage>,
    next_image: u32,
    scroll_remainder: f32,
    /// Modifiers held at the last change, to tell which one moved.
    last_modifiers: Modifiers,
    blink_on: bool,
    search_open: bool,
    /// Created the first time search opens; inputs need a window.
    search_input: Option<Entity<InputState>>,
    search_term: String,
    search_match: Option<Match>,
    search_missed: bool,
    /// Built once: the pointer re-checks for links on every output chunk.
    url_regex: std::cell::RefCell<RegexSearch>,
    _tasks: Vec<Task<()>>,
    _subscriptions: Vec<Subscription>,
    /// Focus and window activation watchers; they need a window to register.
    _window_subscriptions: Vec<Subscription>,
}

impl EventEmitter<TerminalEvent> for TerminalView {}

impl Focusable for TerminalView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TerminalView {
    pub fn new(
        session_id: String,
        kind: SessionKind,
        config: TerminalConfig,
        app: AppHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        let (event_sender, events) = mpsc::channel();
        let grid = GridSize { cols: 80, rows: 24 };
        let term = Term::new(config.term_config(), &grid, EventProxy(event_sender));

        // One writer per terminal keeps keystrokes in order.
        let (input, mut pending) = unbounded_channel::<Vec<u8>>();
        let writer = cx.spawn({
            let (app, id) = (app.clone(), session_id.clone());
            async move |this, cx| {
                while let Some(bytes) = pending.recv().await {
                    let (app, id) = (app.clone(), id.clone());
                    let written = backend(async move {
                        match kind {
                            SessionKind::Local => {
                                app.state::<LocalSessionManager>().write(&id, bytes).await
                            }
                            SessionKind::Ssh => {
                                app.state::<SshSessionManager>().write(&id, bytes).await
                            }
                        }
                    })
                    .await;
                    if let Err(error) = written {
                        let _ = this.update(cx, |_, cx| cx.emit(TerminalEvent::Error(error)));
                        break;
                    }
                }
            }
        });
        let blink = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(BLINK_INTERVAL).await;
            let alive = this.update(cx, |this, cx| {
                this.blink_on = !this.blink_on;
                if this.term.cursor_style().blinking {
                    cx.notify();
                }
            });
            if alive.is_err() {
                break;
            }
        });

        Self {
            session_id,
            kind,
            app,
            term,
            parser: Processor::new(),
            events,
            input,
            focus: cx.focus_handle(),
            config,
            grid,
            bounds: Bounds::default(),
            cell: size(px(8.), px(16.)),
            marked_text: None,
            selecting: false,
            dragged: false,
            reported_button: None,
            reported_motion: None,
            pointer: None,
            hovered_link: None,
            focus_reported: false,
            sync_flush: None,
            scanner: images::Scanner::default(),
            images: HashMap::new(),
            next_image: 0,
            scroll_remainder: 0.,
            last_modifiers: Modifiers::default(),
            blink_on: true,
            search_open: false,
            search_input: None,
            search_term: String::new(),
            search_match: None,
            search_missed: false,
            url_regex: std::cell::RefCell::new(
                RegexSearch::new(URL_PATTERN).expect("the URL pattern compiles"),
            ),
            _tasks: vec![writer, blink],
            _subscriptions: Vec::new(),
            _window_subscriptions: Vec::new(),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Feeds session output to the terminal.
    pub fn feed(&mut self, bytes: &[u8], cx: &mut Context<Self>) {
        for chunk in self.scanner.scan(bytes) {
            match chunk {
                images::Chunk::Text(text) => self.parser.advance(&mut self.term, &text),
                images::Chunk::Sixel(sequence) => {
                    if let Some(image) = images::decode_sixel(&sequence) {
                        let size = (image.width() as f32, image.height() as f32);
                        self.place_image(image, size);
                    }
                }
                images::Chunk::Iterm(payload) => {
                    if let Some(decoded) = images::decode_iterm(&payload) {
                        let cell = (f32::from(self.cell.width), f32::from(self.cell.height));
                        let whole = (
                            cell.0 * self.grid.cols as f32,
                            cell.1 * self.grid.rows as f32,
                        );
                        let size = decoded.display_size(cell, whole);
                        self.place_image(decoded.image, size);
                    }
                }
            }
        }
        self.after_parse(cx);
        self.schedule_sync_flush(cx);
    }

    /// Puts an image at the cursor and moves the cursor below it. Each row
    /// it covers gets a marker in its first cell, so the image follows the
    /// text when it scrolls and goes when that text is erased.
    // ponytail: decodes on the UI thread; move to the background if big images stall typing.
    fn place_image(&mut self, image: image::RgbaImage, (width, height): (f32, f32)) {
        let cell_height = f32::from(self.cell.height);
        let rows = (height / cell_height).ceil().max(1.) as usize;
        let (pixels_wide, pixels_high) = image.dimensions();
        // GPUI keeps images as BGRA.
        let mut pixels = image.into_raw();
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
        let Some(bgra) = image::RgbaImage::from_raw(pixels_wide, pixels_high, pixels) else {
            return;
        };
        // A transparent border keeps neighbours in GPUI's texture atlas from
        // bleeding into the edges when the image is scaled.
        let mut padded = image::RgbaImage::new(pixels_wide + 2, pixels_high + 2);
        image::imageops::replace(&mut padded, &bgra, 1, 1);
        let margin = size(
            px(width / pixels_wide as f32),
            px(height / pixels_high as f32),
        );
        let id = self.next_image;
        self.next_image += 1;
        self.images.insert(
            id,
            PlacedImage {
                image: Arc::new(RenderImage::new(vec![image::Frame::new(padded)])),
                margin,
                size: size(
                    px(width) + margin.width * 2.,
                    px(height) + margin.height * 2.,
                ),
            },
        );
        if self.images.len() > MAX_IMAGES {
            if let Some(oldest) = self.images.keys().min().copied() {
                self.images.remove(&oldest);
            }
        }
        let mut markers = String::new();
        for row in 0..rows {
            markers.push_str(&format!(
                "\x1b]8;;{IMAGE_URI}{id}:{row}\x1b\\ \x08\x1b]8;;\x1b\\\n"
            ));
        }
        self.parser.advance(&mut self.term, markers.as_bytes());
    }

    /// A synchronized update holds output back until the program ends it; one
    /// that never ends is flushed after its timeout, as alacritty does.
    fn schedule_sync_flush(&mut self, cx: &mut Context<Self>) {
        let Some(deadline) = self.parser.sync_timeout().sync_timeout() else {
            self.sync_flush = None;
            return;
        };
        self.sync_flush = Some(cx.spawn(async move |this, cx| {
            let wait = deadline.saturating_duration_since(std::time::Instant::now());
            cx.background_executor().timer(wait).await;
            // Newer output replaces this task, so the update it waited on is
            // still the one pending.
            let _ = this.update(cx, |this, cx| {
                if this.parser.sync_timeout().sync_timeout() == Some(deadline) {
                    this.parser.stop_sync(&mut this.term);
                    this.after_parse(cx);
                }
            });
        }));
    }

    /// Answers what the parser asked for and repaints.
    fn after_parse(&mut self, cx: &mut Context<Self>) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                // alacritty answers DA1 as a VT102; inline images make this
                // a VT220 with sixel (4), which is how programs detect it.
                TermEvent::PtyWrite(text) if text == "\x1b[?6c" => {
                    self.send(b"\x1b[?62;4;22c".to_vec())
                }
                TermEvent::PtyWrite(text) => self.send(text.into_bytes()),
                TermEvent::ClipboardStore(_, text) => {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
                TermEvent::ClipboardLoad(_, format) => {
                    let text = cx.read_from_clipboard().and_then(|item| item.text());
                    self.send(format(&text.unwrap_or_default()).into_bytes());
                }
                TermEvent::ColorRequest(index, format) => {
                    let color =
                        self.term.colors()[index].unwrap_or_else(|| paint::default_color(index));
                    self.send(format(color).into_bytes());
                }
                TermEvent::TextAreaSizeRequest(format) => {
                    let size = self.window_size();
                    self.send(format(size).into_bytes());
                }
                _ => {}
            }
        }
        if let Some((position, modifiers)) = self.pointer {
            self.update_hovered_link(position, modifiers);
        }
        cx.notify();
    }

    pub fn set_config(&mut self, config: TerminalConfig, cx: &mut Context<Self>) {
        if self.config != config {
            self.term.set_options(config.term_config());
            self.config = config;
            cx.notify();
        }
    }

    /// Matches the grid to the tab on screen without reporting it back.
    pub fn set_grid(&mut self, cols: usize, rows: usize, cx: &mut Context<Self>) {
        let grid = GridSize { cols, rows };
        if self.grid == grid {
            return;
        }
        self.grid = grid;
        self.term.resize(grid);
        let (app, id, kind) = (self.app.clone(), self.session_id.clone(), self.kind);
        let (cols, rows) = (cols as u32, rows as u32);
        cx.spawn(async move |this, cx| {
            let resized = backend(async move {
                match kind {
                    SessionKind::Local => {
                        app.state::<LocalSessionManager>()
                            .resize(&id, cols, rows)
                            .await
                    }
                    SessionKind::Ssh => {
                        app.state::<SshSessionManager>()
                            .resize(&id, cols, rows)
                            .await
                    }
                }
            })
            .await;
            if let Err(error) = resized {
                let _ = this.update(cx, |_, cx| cx.emit(TerminalEvent::Error(error)));
            }
        })
        .detach();
        cx.notify();
    }

    pub fn selection_text(&self) -> Option<String> {
        self.term
            .selection_to_string()
            .filter(|text| !text.is_empty())
    }

    /// The last `lines` rows of the buffer, ending at the cursor line.
    pub fn recent_text(&self, lines: usize) -> String {
        let grid = self.term.grid();
        let end = grid.cursor.point.line;
        let start = (end - (lines as i32 - 1)).max(grid.topmost_line());
        self.term
            .bounds_to_string(
                GridPoint::new(start, Column(0)),
                GridPoint::new(end, Column(grid.columns() - 1)),
            )
            .trim()
            .to_string()
    }

    /// Types `text` into the session as a paste.
    pub fn paste(&mut self, text: &str, cx: &mut Context<Self>) {
        self.term.scroll_display(Scroll::Bottom);
        self.send(keys::paste(text, *self.term.mode()));
        cx.notify();
    }

    /// Types `text` into the session as if the user had.
    pub fn send_text(&mut self, text: String) {
        self.send(text.into_bytes());
    }

    fn send(&mut self, bytes: Vec<u8>) {
        if !bytes.is_empty() {
            let _ = self.input.send(bytes);
        }
    }

    fn window_size(&self) -> WindowSize {
        WindowSize {
            num_lines: self.grid.rows as u16,
            num_cols: self.grid.cols as u16,
            cell_width: f32::from(self.cell.width) as u16,
            cell_height: f32::from(self.cell.height) as u16,
        }
    }

    fn font(&self) -> Font {
        font(FONT_FAMILY)
    }

    // --- Keyboard -----------------------------------------------------------

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        // The input method owns the keyboard while it composes.
        if self.marked_text.is_some() {
            return;
        }
        let keystroke = &event.keystroke;
        let modifiers = keystroke.modifiers;
        let key = keystroke.key.as_str();
        let command = modifiers.platform && !modifiers.control && !modifiers.alt;
        let control = modifiers.control && !modifiers.platform && !modifiers.alt;

        let handled = match key {
            "c" if !modifiers.shift && (command || control) => match self.selection_text() {
                Some(text) => {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                    true
                }
                None => false,
            },
            "v" if command || (control && modifiers.shift) => {
                self.paste_clipboard(cx);
                true
            }
            "insert"
                if modifiers.shift
                    && !(modifiers.control || modifiers.alt || modifiers.platform) =>
            {
                self.paste_clipboard(cx);
                true
            }
            "f" if !modifiers.shift && (command || control) => {
                self.open_search(window, cx);
                true
            }
            "g" if command => {
                if self.search_term.is_empty() {
                    self.open_search(window, cx);
                } else {
                    self.find(modifiers.shift, cx);
                }
                true
            }
            "w" if command && !modifiers.shift => {
                cx.emit(TerminalEvent::RequestClose);
                true
            }
            "a" if command && !modifiers.shift && cfg!(target_os = "macos") => {
                self.select_all(cx);
                true
            }
            // Shift+PageUp/PageDown page through the scrollback; full-screen
            // programs have none, so they still get the key.
            "pageup" | "pagedown"
                if modifiers.shift
                    && !(modifiers.control || modifiers.alt || modifiers.platform)
                    && !self.term.mode().contains(TermMode::ALT_SCREEN) =>
            {
                let scroll = if key == "pageup" {
                    Scroll::PageUp
                } else {
                    Scroll::PageDown
                };
                self.term.scroll_display(scroll);
                cx.notify();
                true
            }
            // Cmd+K on macOS; Ctrl+Shift+K elsewhere, since Ctrl+K is the shell's kill-line.
            "k" if (command && !modifiers.shift) || (control && modifiers.shift) => {
                cx.emit(TerminalEvent::RequestAssistant);
                true
            }
            _ => false,
        };
        if handled {
            cx.stop_propagation();
            return;
        }
        // Other Cmd shortcuts belong to the app, not the remote program.
        if cfg!(target_os = "macos") && modifiers.platform {
            return;
        }

        let key = keys::Key {
            key,
            text: keystroke.key_char.as_deref(),
            shift: modifiers.shift,
            alt: modifiers.alt,
            ctrl: modifiers.control,
            platform: modifiers.platform,
        };
        let kind = if event.is_held {
            keys::KeyEvent::Repeat
        } else {
            keys::KeyEvent::Press
        };
        let alt_is_meta = !cfg!(target_os = "macos");
        if let Some(bytes) = keys::encode(&key, kind, *self.term.mode(), alt_is_meta) {
            self.typed(bytes, cx);
            cx.stop_propagation();
        }
    }

    fn on_key_up(&mut self, event: &KeyUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if !self.term.mode().contains(TermMode::REPORT_EVENT_TYPES) || self.marked_text.is_some() {
            return;
        }
        let keystroke = &event.keystroke;
        let modifiers = keystroke.modifiers;
        let key = keys::Key {
            key: keystroke.key.as_str(),
            text: None,
            shift: modifiers.shift,
            alt: modifiers.alt,
            ctrl: modifiers.control,
            platform: modifiers.platform,
        };
        if let Some(bytes) = keys::encode(&key, keys::KeyEvent::Release, *self.term.mode(), true) {
            self.send(bytes);
            cx.stop_propagation();
        }
    }

    /// The platform copy key: copies the selection. Without one, Ctrl+C is
    /// still the shell's interrupt.
    fn copy(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = self.selection_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            return;
        }
        if cfg!(target_os = "macos") {
            return;
        }
        let key = keys::Key {
            key: "c",
            ctrl: true,
            ..keys::Key::default()
        };
        if let Some(bytes) = keys::encode(&key, keys::KeyEvent::Press, *self.term.mode(), true) {
            self.typed(bytes, cx);
        }
    }

    fn select_all(&mut self, cx: &mut Context<Self>) {
        let grid = self.term.grid();
        let start = GridPoint::new(grid.topmost_line(), Column(0));
        let end = GridPoint::new(grid.bottommost_line(), grid.last_column());
        let mut selection = Selection::new(SelectionType::Simple, start, Side::Left);
        selection.update(end, Side::Right);
        self.term.selection = Some(selection);
        cx.notify();
    }

    /// Modifier keys alone, for programs that asked for every key.
    fn on_modifiers_changed(
        &mut self,
        event: &ModifiersChangedEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some((position, _)) = self.pointer {
            self.pointer = Some((position, event.modifiers));
            if self.update_hovered_link(position, event.modifiers) {
                cx.notify();
            }
        }
        let mode = *self.term.mode();
        if !mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC) || self.marked_text.is_some() {
            self.last_modifiers = event.modifiers;
            return;
        }
        let (before, after) = (self.last_modifiers, event.modifiers);
        self.last_modifiers = after;
        let key = keys::Key {
            shift: after.shift,
            alt: after.alt,
            ctrl: after.control,
            platform: after.platform,
            ..keys::Key::default()
        };
        for (name, was, now) in [
            ("shift", before.shift, after.shift),
            ("control", before.control, after.control),
            ("alt", before.alt, after.alt),
            ("platform", before.platform, after.platform),
        ] {
            if was != now {
                if let Some(bytes) = keys::modifier(name, now, &key, mode) {
                    self.send(bytes);
                }
            }
        }
    }

    fn send_tab(&mut self, shift: bool, cx: &mut Context<Self>) {
        let key = keys::Key {
            key: "tab",
            shift,
            ..keys::Key::default()
        };
        if let Some(bytes) = keys::encode(&key, keys::KeyEvent::Press, *self.term.mode(), true) {
            self.typed(bytes, cx);
        }
    }

    /// Sends user input: clears the selection and follows the cursor.
    fn typed(&mut self, bytes: Vec<u8>, cx: &mut Context<Self>) {
        self.term.selection = None;
        self.term.scroll_display(Scroll::Bottom);
        self.blink_on = true;
        self.send(bytes);
        cx.notify();
    }

    fn paste_clipboard(&mut self, cx: &mut Context<Self>) {
        let Some(item) = cx.read_from_clipboard() else {
            return;
        };
        if let Some(text) = item.text().filter(|text| !text.is_empty()) {
            self.paste(&text, cx);
            return;
        }
        // Remote programs cannot read the local clipboard, so hand them a file path.
        if self.kind != SessionKind::Ssh {
            return;
        }
        let image = item.into_entries().find_map(|entry| match entry {
            ClipboardEntry::Image(image) => Some(image),
            _ => None,
        });
        let Some(image) = image else {
            return;
        };
        let extension = match image.format {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Gif => "gif",
            ImageFormat::Webp => "webp",
            ImageFormat::Bmp => "bmp",
            ImageFormat::Tiff => "tiff",
            _ => "png",
        }
        .to_string();
        let (app, id) = (self.app.clone(), self.session_id.clone());
        cx.spawn(async move |this, cx| {
            let uploaded = backend(async move {
                crate::sftp::sftp_paste_image(id, image.bytes, extension, app.state()).await
            })
            .await;
            match uploaded {
                Ok(path) => {
                    let _ = this.update(cx, |this, cx| this.paste(&path, cx));
                }
                Err(error) => tracing::error!(error, "Failed to paste image"),
            }
        })
        .detach();
    }

    // --- Mouse --------------------------------------------------------------

    /// The grid cell under a window position, and which half of it.
    fn cell_at(&self, position: Point<Pixels>) -> (GridPoint, Side) {
        let x = f32::from(position.x - self.bounds.origin.x - PADDING).max(0.);
        let y = f32::from(position.y - self.bounds.origin.y - PADDING).max(0.);
        let (cell_width, cell_height) = (f32::from(self.cell.width), f32::from(self.cell.height));
        let column = ((x / cell_width) as usize).min(self.grid.cols.saturating_sub(1));
        let row = ((y / cell_height) as usize).min(self.grid.rows.saturating_sub(1));
        let side = if x % cell_width > cell_width / 2. {
            Side::Right
        } else {
            Side::Left
        };
        let line = Line(row as i32 - self.term.grid().display_offset() as i32);
        (GridPoint::new(line, Column(column)), side)
    }

    /// Whether mouse events go to the program instead of selecting text.
    /// Shift (and Option on macOS) is the escape hatch back to selection.
    fn reports_mouse(&self, modifiers: Modifiers) -> bool {
        self.term.mode().intersects(TermMode::MOUSE_MODE) && !modifiers.shift && !modifiers.alt
    }

    fn report_mouse(
        &mut self,
        button: u8,
        pressed: bool,
        position: Point<Pixels>,
        modifiers: Modifiers,
    ) {
        let (point, _) = self.cell_at(position);
        let row = point.line.0 + self.term.grid().display_offset() as i32;
        let code = button
            + 4 * u8::from(modifiers.shift)
            + 8 * u8::from(modifiers.alt)
            + 16 * u8::from(modifiers.control);
        let (column, row) = (point.column.0 + 1, row + 1);
        let bytes = if self.term.mode().contains(TermMode::SGR_MOUSE) {
            let suffix = if pressed { 'M' } else { 'm' };
            format!("\x1b[<{code};{column};{row}{suffix}").into_bytes()
        } else {
            // The legacy encoding has one byte per coordinate.
            if column > 223 || row > 223 {
                return;
            }
            let code = if pressed { code } else { 3 };
            vec![
                0x1b,
                b'[',
                b'M',
                32 + code,
                32 + column as u8,
                32 + row as u8,
            ]
        };
        self.send(bytes);
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus.focus(window, cx);
        let button = match event.button {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
            _ => return,
        };
        if self.reports_mouse(event.modifiers) {
            self.reported_button = Some(button);
            self.reported_motion = Some(self.cell_at(event.position).0);
            self.report_mouse(button, true, event.position, event.modifiers);
            return;
        }
        let (point, side) = self.cell_at(event.position);
        if event.button == MouseButton::Right {
            // Right-click selects the word under it, as xterm.js does on
            // macOS, unless it lands inside the current selection.
            let inside = self
                .term
                .selection
                .as_ref()
                .and_then(|selection| selection.to_range(&self.term))
                .is_some_and(|range| range.contains(point));
            if !inside {
                self.term.selection = Some(Selection::new(SelectionType::Semantic, point, side));
                cx.notify();
            }
            return;
        }
        if event.button != MouseButton::Left {
            return;
        }
        // Shift+click extends the selection to the clicked cell.
        if event.modifiers.shift && event.click_count <= 1 {
            if let Some(selection) = self.term.selection.as_mut() {
                selection.update(point, side);
                self.selecting = true;
                self.dragged = true;
                cx.notify();
                return;
            }
        }
        let kind = match event.click_count {
            0 | 1 => SelectionType::Simple,
            2 => SelectionType::Semantic,
            _ => SelectionType::Lines,
        };
        self.term.selection = Some(Selection::new(kind, point, side));
        self.selecting = true;
        self.dragged = event.click_count > 1;
        cx.notify();
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.pointer = Some((event.position, event.modifiers));
        if let Some(button) = self.reported_button {
            let mode = *self.term.mode();
            if mode.intersects(TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION) {
                self.report_motion(button + 32, event.position, event.modifiers);
            }
            return;
        }
        if event.pressed_button.is_none()
            && self.term.mode().contains(TermMode::MOUSE_MOTION)
            && self.reports_mouse(event.modifiers)
        {
            // Any-event tracking: motion with no button held is button 3.
            self.report_motion(35, event.position, event.modifiers);
        }
        if self.selecting && event.pressed_button == Some(MouseButton::Left) {
            let (point, side) = self.cell_at(event.position);
            if let Some(selection) = self.term.selection.as_mut() {
                selection.update(point, side);
                self.dragged = true;
                cx.notify();
            }
            return;
        }
        if self.update_hovered_link(event.position, event.modifiers) {
            cx.notify();
        }
    }

    /// Reports motion once per cell crossed.
    fn report_motion(&mut self, button: u8, position: Point<Pixels>, modifiers: Modifiers) {
        let (point, _) = self.cell_at(position);
        if self.reported_motion == Some(point) {
            return;
        }
        self.reported_motion = Some(point);
        self.report_mouse(button, true, position, modifiers);
    }

    fn on_hover(&mut self, hovered: &bool, _: &mut Window, cx: &mut Context<Self>) {
        if !*hovered {
            self.pointer = None;
            if self.hovered_link.take().is_some() {
                cx.notify();
            }
        }
    }

    fn on_mouse_up(&mut self, event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(button) = self.reported_button.take() {
            self.report_mouse(button, false, event.position, event.modifiers);
            return;
        }
        if !self.selecting {
            return;
        }
        self.selecting = false;
        if !self.dragged {
            // A plain click: drop the empty selection, then follow a link or,
            // with Option/Alt, move the shell's cursor to the click.
            self.term.selection = None;
            let (point, _) = self.cell_at(event.position);
            let mouse_mode = self.term.mode().intersects(TermMode::MOUSE_MODE);
            if event.modifiers.alt && !mouse_mode {
                let moves = self.cursor_moves_to(point);
                self.typed(moves, cx);
            } else if let Some((url, _)) = self.link_at(point) {
                cx.open_url(&url);
            }
        }
        cx.notify();
    }

    /// Arrow keys that walk the cursor to `target`, as xterm.js' Alt+click
    /// does: along the wrapped line, and between rows only in full-screen
    /// programs, which own the whole screen.
    fn cursor_moves_to(&self, target: GridPoint) -> Vec<u8> {
        let cursor = self.term.grid().cursor.point;
        let mode = *self.term.mode();
        let arrow = |letter: char| {
            let prefix = if mode.contains(TermMode::APP_CURSOR) {
                "\x1bO"
            } else {
                "\x1b["
            };
            format!("{prefix}{letter}")
        };
        let columns = self.term.grid().columns() as i32;
        let same_line =
            self.term.line_search_left(cursor).line == self.term.line_search_left(target).line;
        let steps = if same_line {
            (target.line.0 - cursor.line.0) * columns
                + (target.column.0 as i32 - cursor.column.0 as i32)
        } else if mode.contains(TermMode::ALT_SCREEN) {
            let rows = target.line.0 - cursor.line.0;
            let vertical =
                arrow(if rows < 0 { 'A' } else { 'B' }).repeat(rows.unsigned_abs() as usize);
            let columns = target.column.0 as i32 - cursor.column.0 as i32;
            let horizontal =
                arrow(if columns < 0 { 'D' } else { 'C' }).repeat(columns.unsigned_abs() as usize);
            return format!("{vertical}{horizontal}").into_bytes();
        } else {
            0
        };
        arrow(if steps < 0 { 'D' } else { 'C' })
            .repeat(steps.unsigned_abs() as usize)
            .into_bytes()
    }

    fn on_scroll(&mut self, event: &ScrollEvent, _: &mut Window, cx: &mut Context<Self>) {
        let delta = f32::from(event.delta.pixel_delta(self.cell.height).y) + self.scroll_remainder;
        let lines = (delta / f32::from(self.cell.height)).trunc();
        self.scroll_remainder = delta - lines * f32::from(self.cell.height);
        let lines = lines as i32;
        if lines == 0 {
            return;
        }
        let mode = *self.term.mode();
        if self.reports_mouse(event.modifiers) {
            let button = if lines > 0 { 64 } else { 65 };
            for _ in 0..lines.unsigned_abs() {
                self.report_mouse(button, true, event.position, event.modifiers);
            }
        } else if mode.contains(TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL) {
            // Full-screen programs without mouse support scroll with the arrows.
            let arrow: &[u8] = match (lines > 0, mode.contains(TermMode::APP_CURSOR)) {
                (true, true) => b"\x1bOA",
                (true, false) => b"\x1b[A",
                (false, true) => b"\x1bOB",
                (false, false) => b"\x1b[B",
            };
            self.send(arrow.repeat(lines.unsigned_abs() as usize));
        } else {
            self.term.scroll_display(Scroll::Delta(lines));
            if let Some((position, modifiers)) = self.pointer {
                self.update_hovered_link(position, modifiers);
            }
            cx.notify();
        }
    }

    /// Recomputes the link under the pointer; true when it changed.
    fn update_hovered_link(&mut self, position: Point<Pixels>, modifiers: Modifiers) -> bool {
        let inside = self.bounds.contains(&position);
        let link = (inside && !self.reports_mouse(modifiers))
            .then(|| self.link_at(self.cell_at(position).0))
            .flatten();
        let changed = link.as_ref().map(|(_, range)| range)
            != self.hovered_link.as_ref().map(|(_, range)| range);
        self.hovered_link = link;
        changed
    }

    /// The link at `point` and the cells it covers: an OSC 8 hyperlink, or a
    /// URL in the text, followed across wrapped rows.
    fn link_at(&self, point: GridPoint) -> Option<(String, Match)> {
        let grid = self.term.grid();
        if point.line < grid.topmost_line() || point.line > grid.bottommost_line() {
            return None;
        }
        let line_start = self.term.line_search_left(point);
        let line_end = self.term.line_search_right(point);
        if let Some(link) = grid[point]
            .hyperlink()
            .filter(|link| !link.uri().starts_with(IMAGE_URI))
        {
            let same = |at: GridPoint| grid[at].hyperlink().as_ref() == Some(&link);
            let (mut start, mut end) = (point, point);
            while start > line_start {
                let previous = start.sub(&self.term, Boundary::None, 1);
                if !same(previous) {
                    break;
                }
                start = previous;
            }
            while end < line_end {
                let next = end.add(&self.term, Boundary::None, 1);
                if !same(next) {
                    break;
                }
                end = next;
            }
            return Some((link.uri().to_string(), start..=end));
        }
        let mut regex = self.url_regex.borrow_mut();
        let found = RegexIter::new(
            line_start,
            line_end,
            Direction::Right,
            &self.term,
            &mut regex,
        )
        .find(|found| found.contains(&point))?;
        let url = self
            .term
            .bounds_to_string(*found.start(), *found.end())
            .replace('\n', "");
        Some((url, found))
    }

    // --- Focus --------------------------------------------------------------

    /// Tells a program that asked (DECSET 1004) when the terminal gains or
    /// loses keyboard focus, including when the whole window does.
    fn sync_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focused = self.focus.is_focused(window) && window.is_window_active();
        if focused == self.focus_reported {
            return;
        }
        self.focus_reported = focused;
        if self.term.mode().contains(TermMode::FOCUS_IN_OUT) {
            self.send(if focused {
                b"\x1b[I".to_vec()
            } else {
                b"\x1b[O".to_vec()
            });
        }
        cx.notify();
    }

    fn watch_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self._window_subscriptions.is_empty() {
            return;
        }
        let focus = self.focus.clone();
        self._window_subscriptions = vec![
            cx.on_focus_in(&focus, window, Self::sync_focus),
            cx.on_focus_out(&focus, window, |this, _, window, cx| {
                this.sync_focus(window, cx)
            }),
            cx.observe_window_activation(window, Self::sync_focus),
        ];
        self.sync_focus(window, cx);
    }

    // --- Search -------------------------------------------------------------

    fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_open = true;
        if self.search_input.is_none() {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder(t("Search")));
            let changes = cx.subscribe(&input, |this, input, event: &InputEvent, cx| match event {
                InputEvent::Change => {
                    this.search_term = input.read(cx).value().to_string();
                    this.search_match = None;
                    this.find(false, cx);
                }
                InputEvent::PressEnter { shift, .. } => this.find(*shift, cx),
                InputEvent::Focus | InputEvent::Blur => {}
            });
            self._subscriptions.push(changes);
            self.search_input = Some(input);
        }
        if let Some(input) = &self.search_input {
            input.update(cx, |input, cx| input.focus(window, cx));
        }
        cx.notify();
    }

    fn close_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_open = false;
        self.search_match = None;
        self.search_missed = false;
        self.focus.focus(window, cx);
        cx.notify();
    }

    fn find(&mut self, backwards: bool, cx: &mut Context<Self>) {
        self.search_missed = false;
        if self.search_term.is_empty() {
            self.search_match = None;
            cx.notify();
            return;
        }
        let escaped: String = self
            .search_term
            .chars()
            .flat_map(|character| {
                let special = "\\.+*?()|[]{}^$#&-~".contains(character);
                special.then_some('\\').into_iter().chain([character])
            })
            .collect();
        let Ok(mut regex) = RegexSearch::new(&escaped) else {
            return;
        };
        let direction = if backwards {
            Direction::Left
        } else {
            Direction::Right
        };
        // Continue from the current match, or start at the top of the screen.
        let origin = match &self.search_match {
            Some(found) if backwards => {
                found
                    .start()
                    .sub(&self.term, alacritty_terminal::index::Boundary::None, 1)
            }
            Some(found) => {
                found
                    .end()
                    .add(&self.term, alacritty_terminal::index::Boundary::None, 1)
            }
            None => GridPoint::new(Line(-(self.term.grid().display_offset() as i32)), Column(0)),
        };
        match self
            .term
            .search_next(&mut regex, origin, direction, Side::Left, None)
        {
            Some(found) => {
                self.term.scroll_to_point(*found.start());
                self.search_match = Some(found);
            }
            None => {
                self.search_match = None;
                self.search_missed = true;
            }
        }
        cx.notify();
    }

    fn render_search(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let empty = self.search_term.is_empty();
        h_flex()
            .absolute()
            .top_2()
            .right_2()
            .h_8()
            .gap_1()
            .px_2()
            .rounded(px(2.))
            .border_1()
            .border_color(theme::hairline())
            .bg(theme::RAIL)
            .text_color(theme::STENCIL)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.close_search(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                Icon::new(IconName::Search)
                    .xsmall()
                    .text_color(theme::STENCIL_MUTE),
            )
            .children(self.search_input.as_ref().map(|input| {
                div()
                    .w(px(176.))
                    .child(Input::new(input).xsmall().appearance(false))
            }))
            .when(self.search_missed && !empty, |bar| {
                bar.child(
                    div()
                        .px_1()
                        .text_xs()
                        .text_color(rgb(0xe6c27a))
                        .child(t("No match")),
                )
            })
            .child(
                Button::new("search-previous")
                    .ghost()
                    .xsmall()
                    .icon(IconName::ChevronUp)
                    .tooltip(t("Previous result"))
                    .disabled(empty)
                    .on_click(cx.listener(|this, _, _, cx| this.find(true, cx))),
            )
            .child(
                Button::new("search-next")
                    .ghost()
                    .xsmall()
                    .icon(IconName::ChevronDown)
                    .tooltip(t("Next result"))
                    .disabled(empty)
                    .on_click(cx.listener(|this, _, _, cx| this.find(false, cx))),
            )
            .child(
                Button::new("search-close")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Close)
                    .tooltip(t("Close search"))
                    .on_click(cx.listener(|this, _, window, cx| this.close_search(window, cx))),
            )
    }
}

impl EntityInputHandler for TerminalView {
    fn text_for_range(
        &mut self,
        _: Range<usize>,
        _: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        None
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: 0..0,
            reversed: false,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked_text
            .as_ref()
            .map(|text| 0..text.encode_utf16().count())
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_text = None;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.marked_text = None;
        let bytes = keys::text(text, *self.term.mode());
        self.typed(bytes, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        new_text: &str,
        _: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.marked_text = (!new_text.is_empty()).then(|| new_text.to_string());
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        // Where the input method should put its candidate window.
        Some(self.cursor_bounds())
    }

    fn character_index_for_point(
        &mut self,
        _: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.watch_focus(window, cx);
        let entity = cx.entity();
        let painter = entity.clone();
        div()
            .id("terminal")
            .relative()
            .size_full()
            .bg(theme::TERMINAL_INK)
            .track_focus(&self.focus)
            .key_context("Terminal")
            .on_action(cx.listener(|this, _: &SendTab, _, cx| this.send_tab(false, cx)))
            .on_action(cx.listener(|this, _: &SendBackTab, _, cx| this.send_tab(true, cx)))
            .on_action(cx.listener(|this, _: &Copy, _, cx| this.copy(cx)))
            .on_key_down(cx.listener(Self::on_key_down))
            .on_key_up(cx.listener(Self::on_key_up))
            .on_modifiers_changed(cx.listener(Self::on_modifiers_changed))
            .on_hover(cx.listener(Self::on_hover))
            .map(|terminal| {
                if self.term.mode().intersects(TermMode::MOUSE_MODE) {
                    terminal.cursor_default()
                } else if self.hovered_link.is_some() {
                    terminal.cursor_pointer()
                } else {
                    terminal.cursor_text()
                }
            })
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::on_mouse_up))
            .on_mouse_up(MouseButton::Right, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Middle, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Right, cx.listener(Self::on_mouse_up))
            .on_scroll(cx.listener(Self::on_scroll))
            .child(
                canvas(
                    move |bounds, window, cx| {
                        entity.update(cx, |this, cx| this.layout(bounds, window, cx))
                    },
                    move |bounds, frame, window, cx| {
                        let focus = painter.read(cx).focus.clone();
                        window.handle_input(
                            &focus,
                            ElementInputHandler::new(bounds, painter.clone()),
                            cx,
                        );
                        frame.paint(window, cx);
                    },
                )
                .size_full(),
            )
            .when(self.search_open, |terminal| {
                terminal.child(self.render_search(cx))
            })
    }
}

#[cfg(test)]
mod view_tests {
    use super::*;
    use tokio::sync::mpsc::UnboundedReceiver;

    struct Harness {
        cx: TestAppContext,
        window: AnyWindowHandle,
        view: Entity<TerminalView>,
        sent: UnboundedReceiver<Vec<u8>>,
    }

    impl Harness {
        fn new() -> Self {
            let mut cx = TestAppContext::single();
            cx.update(|cx| {
                gpui_kit::init(cx);
                crate::ui::shell::bind_keys(cx);
                bind_keys(cx);
            });
            let app = AppHandle::default();
            app.manage(LocalSessionManager::new());
            let (window, view) = cx
                .update(|cx| {
                    gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
                        cx.new(|cx| {
                            TerminalView::new(
                                "t".into(),
                                SessionKind::Local,
                                TerminalConfig::default(),
                                app,
                                cx,
                            )
                        })
                    })
                })
                .unwrap();
            let (input, sent) = unbounded_channel();
            cx.update(|cx| view.update(cx, |view, _| view.input = input));
            cx.update_window(window, |_, window, cx| {
                window.activate_window();
                view.read(cx).focus.clone().focus(window, cx)
            })
            .unwrap();
            cx.run_until_parked();
            let mut harness = Self {
                cx,
                window,
                view,
                sent,
            };
            harness.take();
            harness
        }

        fn take(&mut self) -> String {
            self.cx.run_until_parked();
            let mut bytes = Vec::new();
            while let Ok(chunk) = self.sent.try_recv() {
                bytes.extend(chunk);
            }
            String::from_utf8(bytes).unwrap()
        }

        fn keys(&mut self, keys: &str) -> String {
            self.cx.simulate_keystrokes(self.window, keys);
            self.take()
        }

        fn feed(&mut self, output: &str) {
            let view = self.view.clone();
            self.cx
                .update(|cx| view.update(cx, |view, cx| view.feed(output.as_bytes(), cx)));
            self.cx.run_until_parked();
        }

        fn read<R>(&mut self, read: impl FnOnce(&TerminalView) -> R) -> R {
            let view = self.view.clone();
            self.cx.update(|cx| read(view.read(cx)))
        }
    }

    #[::core::prelude::v1::test]
    fn keys_reach_the_session_through_the_window() {
        let mut terminal = Harness::new();
        assert_eq!(terminal.keys("alt-enter"), "\x1b\r");
        assert_eq!(terminal.keys("enter"), "\r");
        assert_eq!(terminal.keys("shift-enter"), "\x1b[13;2u");
        assert_eq!(terminal.keys("tab"), "\t");
        assert_eq!(terminal.keys("shift-tab"), "\x1b[Z");
        assert_eq!(terminal.keys("ctrl-c"), "\x03");
        assert_eq!(terminal.keys("escape"), "\x1b");
        assert_eq!(terminal.keys("alt-backspace"), "\x1b\x7f");
        assert_eq!(terminal.keys("alt-left"), "\x1b[1;3D");

        terminal.feed("\x1b[>5u");
        assert_eq!(terminal.keys("alt-enter"), "\x1b[13;3u");
        assert_eq!(terminal.keys("escape"), "\x1b[27u");
    }

    #[::core::prelude::v1::test]
    fn focus_changes_are_reported_when_asked() {
        let mut terminal = Harness::new();
        terminal.feed("\x1b[?1004h");
        let window = terminal.window;
        terminal
            .cx
            .update_window(window, |_, window, cx| window.blur(cx))
            .unwrap();
        assert_eq!(terminal.take(), "\x1b[O");
        let view = terminal.view.clone();
        terminal
            .cx
            .update_window(window, |_, window, cx| {
                view.read(cx).focus.clone().focus(window, cx)
            })
            .unwrap();
        assert_eq!(terminal.take(), "\x1b[I");
    }

    #[::core::prelude::v1::test]
    fn links_are_found_across_wrapped_rows_and_osc8() {
        let mut terminal = Harness::new();
        let cols = terminal.read(|view| view.grid.cols);
        let url = format!("https://example.com/{}", "a".repeat(cols));
        terminal.feed(&format!("see {url}. done\r\n"));
        let found = terminal.read(|view| view.link_at(GridPoint::new(Line(1), Column(3))));
        assert_eq!(found.map(|(link, _)| link), Some(url));
        let none = terminal.read(|view| view.link_at(GridPoint::new(Line(0), Column(1))));
        assert!(none.is_none());

        terminal.feed("\x1b]8;;https://osc8.example/\x1b\\label\x1b]8;;\x1b\\ tail\r\n");
        let line = terminal.read(|view| view.term.grid().cursor.point.line - 1);
        let found = terminal.read(|view| view.link_at(GridPoint::new(line, Column(2))));
        let (link, range) = found.unwrap();
        assert_eq!(link, "https://osc8.example/");
        assert_eq!(
            (range.start().column, range.end().column),
            (Column(0), Column(4))
        );
    }

    #[::core::prelude::v1::test]
    fn alt_click_walks_the_cursor_along_the_line() {
        let mut terminal = Harness::new();
        terminal.feed("$ echo hello");
        let moves = terminal.read(|view| view.cursor_moves_to(GridPoint::new(Line(0), Column(2))));
        assert_eq!(String::from_utf8(moves).unwrap(), "\x1b[D".repeat(10));
        let moves = terminal.read(|view| view.cursor_moves_to(GridPoint::new(Line(3), Column(0))));
        assert!(moves.is_empty());
    }

    #[::core::prelude::v1::test]
    fn unfinished_synchronized_updates_are_flushed() {
        let mut terminal = Harness::new();
        let shown = |view: &TerminalView| view.term.grid()[GridPoint::new(Line(0), Column(0))].c;
        let view = terminal.view.clone();
        let held = terminal.cx.update(|cx| {
            view.update(cx, |view, cx| {
                view.feed(b"\x1b[?2026hvisible", cx);
                shown(view)
            })
        });
        assert_eq!(held, ' ');
        terminal
            .cx
            .executor()
            .advance_clock(Duration::from_millis(500));
        terminal.cx.run_until_parked();
        assert_eq!(terminal.read(shown), 'v');
    }

    #[::core::prelude::v1::test]
    fn sixel_images_are_placed_and_advertised() {
        let mut terminal = Harness::new();
        terminal.feed("\x1b[c");
        assert_eq!(terminal.take(), "\x1b[?62;4;22c");
        // 40 pixels tall: three rows at the default 16px cell.
        let band = "#1;2;100;0;0#1!8~";
        let sixel = format!("ab\x1bPq{}\x1b\\cd", [band; 7].join("-"));
        terminal.feed(&sixel);
        let (count, cursor, marker) = terminal.read(|view| {
            let grid = view.term.grid();
            let marker = grid[GridPoint::new(Line(1), Column(2))]
                .hyperlink()
                .map(|link| link.uri().to_string());
            (view.images.len(), grid.cursor.point, marker)
        });
        assert_eq!(count, 1);
        assert_eq!(marker.as_deref(), Some("noverterm-image:0:1"));
        assert_eq!(cursor, GridPoint::new(Line(3), Column(4)));
        // Image markers are not links.
        let link = terminal.read(|view| view.link_at(GridPoint::new(Line(0), Column(2))));
        assert!(link.is_none());
    }

    #[::core::prelude::v1::test]
    fn shift_page_up_scrolls_back_and_symbols_keep_kitty_shift() {
        let mut terminal = Harness::new();
        terminal.feed(&"line\r\n".repeat(200));
        assert_eq!(terminal.keys("shift-pageup"), "");
        assert!(terminal.read(|view| view.term.grid().display_offset()) > 0);
        assert_eq!(terminal.keys("shift-pagedown"), "");
        assert_eq!(terminal.read(|view| view.term.grid().display_offset()), 0);
    }
}
