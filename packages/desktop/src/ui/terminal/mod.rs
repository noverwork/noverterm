//! A terminal tab: an alacritty grid fed by a backend session and painted
//! straight onto a GPUI canvas.

pub mod keys;
mod paint;

use std::ops::Range;
use std::sync::mpsc;
use std::time::Duration;

use alacritty_terminal::event::{Event as TermEvent, EventListener, WindowSize};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Direction, Line, Point as GridPoint, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::search::{Match, RegexSearch};
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
/// Space between the window edge and the first column.
const PADDING: Pixels = px(4.);

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
    scroll_remainder: f32,
    blink_on: bool,
    search_open: bool,
    /// Created the first time search opens; inputs need a window.
    search_input: Option<Entity<InputState>>,
    search_term: String,
    search_match: Option<Match>,
    search_missed: bool,
    _tasks: Vec<Task<()>>,
    _subscriptions: Vec<Subscription>,
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
            scroll_remainder: 0.,
            blink_on: true,
            search_open: false,
            search_input: None,
            search_term: String::new(),
            search_match: None,
            search_missed: false,
            _tasks: vec![writer, blink],
            _subscriptions: Vec::new(),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Feeds session output to the terminal.
    pub fn feed(&mut self, bytes: &[u8], cx: &mut Context<Self>) {
        self.parser.advance(&mut self.term, bytes);
        while let Ok(event) = self.events.try_recv() {
            match event {
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
        let y = f32::from(position.y - self.bounds.origin.y).max(0.);
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
            self.report_mouse(button, true, event.position, event.modifiers);
            return;
        }
        if event.button != MouseButton::Left {
            return;
        }
        let (point, side) = self.cell_at(event.position);
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
        if let Some(button) = self.reported_button {
            let mode = *self.term.mode();
            if mode.intersects(TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION) {
                self.report_mouse(button + 32, true, event.position, event.modifiers);
            }
            return;
        }
        if !self.selecting || event.pressed_button != Some(MouseButton::Left) {
            return;
        }
        let (point, side) = self.cell_at(event.position);
        if let Some(selection) = self.term.selection.as_mut() {
            selection.update(point, side);
            self.dragged = true;
            cx.notify();
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
            // A plain click: drop the empty selection and follow a link.
            self.term.selection = None;
            let (point, _) = self.cell_at(event.position);
            if let Some(url) = self.url_at(point) {
                cx.open_url(&url);
            }
        }
        cx.notify();
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
            let arrow: &[u8] = if lines > 0 { b"\x1bOA" } else { b"\x1bOB" };
            self.send(arrow.repeat(lines.unsigned_abs() as usize));
        } else {
            self.term.scroll_display(Scroll::Delta(lines));
            cx.notify();
        }
    }

    /// The link under `point`: an OSC 8 hyperlink, or an http(s) URL in the text.
    fn url_at(&self, point: GridPoint) -> Option<String> {
        let grid = self.term.grid();
        if let Some(link) = grid[point].hyperlink() {
            return Some(link.uri().to_string());
        }
        let is_boundary = |character: char| {
            character.is_whitespace() || matches!(character, '"' | '\'' | '<' | '>' | '`' | '\0')
        };
        let character_at = |column: usize| grid[GridPoint::new(point.line, Column(column))].c;
        if is_boundary(character_at(point.column.0)) {
            return None;
        }
        let mut start = point.column.0;
        while start > 0 && !is_boundary(character_at(start - 1)) {
            start -= 1;
        }
        let mut end = point.column.0;
        while end + 1 < grid.columns() && !is_boundary(character_at(end + 1)) {
            end += 1;
        }
        let token: String = (start..=end)
            .map(character_at)
            .filter(|character| *character != ' ')
            .collect();
        let start = token.find("https://").or_else(|| token.find("http://"))?;
        let url = token[start..].trim_end_matches(['.', ',', ';', ':', '!', '?', ')', ']', '}']);
        Some(url.to_string())
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
        self.typed(text.as_bytes().to_vec(), cx);
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let painter = entity.clone();
        div()
            .id("terminal")
            .relative()
            .size_full()
            .bg(theme::TERMINAL_INK)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_key_up(cx.listener(Self::on_key_up))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::on_mouse_up))
            .on_mouse_up(MouseButton::Right, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
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
