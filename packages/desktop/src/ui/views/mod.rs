//! One module per page, plus the layers that sit over every page.

pub mod assistant;
pub mod common;
pub mod connection_form;
pub mod connections;
pub mod forwards;
pub mod keys;
pub mod known_hosts;
pub mod links;
pub mod mcp;
pub mod settings;
pub mod sftp;
pub mod snippets;
pub mod updater;

use gpui_kit::*;

use crate::ui::store::Route;
use crate::ui::theme;

/// Builds the page for `route`. `Route::Terminal` is drawn by the shell itself.
pub fn build(route: &Route, window: &mut Window, cx: &mut App) -> AnyView {
    match route {
        Route::Connections => cx
            .new(|cx| connections::ConnectionsView::new(window, cx))
            .into(),
        Route::ConnectionForm(id) => {
            let id = id.clone();
            cx.new(|cx| connection_form::ConnectionForm::new(id, window, cx))
                .into()
        }
        Route::Keys => cx.new(|cx| keys::KeysView::new(window, cx)).into(),
        Route::KeyForm(id) => {
            let id = id.clone();
            cx.new(|cx| keys::KeyForm::new(id, window, cx)).into()
        }
        Route::KnownHosts => cx
            .new(|cx| known_hosts::KnownHostsView::new(window, cx))
            .into(),
        Route::Forwards => cx.new(|cx| forwards::ForwardsView::new(window, cx)).into(),
        Route::ForwardForm(id) => {
            let id = id.clone();
            cx.new(|cx| forwards::ForwardForm::new(id, window, cx))
                .into()
        }
        Route::Links => cx.new(|cx| links::LinksView::new(window, cx)).into(),
        Route::LinkForm(id) => {
            let id = id.clone();
            cx.new(|cx| links::LinkForm::new(id, window, cx)).into()
        }
        Route::Sftp => cx.new(|cx| sftp::SftpView::new(window, cx)).into(),
        Route::Snippets => cx.new(|cx| snippets::SnippetsView::new(window, cx)).into(),
        Route::SnippetForm { id, shortcut } => {
            let (id, shortcut) = (id.clone(), *shortcut);
            cx.new(|cx| snippets::SnippetForm::new(id, shortcut, window, cx))
                .into()
        }
        Route::Mcp => cx.new(|cx| mcp::McpView::new(window, cx)).into(),
        _ => cx.new(|_| Placeholder(format!("{route:?}").into())).into(),
    }
}

struct Placeholder(SharedString);

impl Render for Placeholder {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(theme::STENCIL_MUTE)
            .child(self.0.clone())
    }
}

/// Whether AI agents may run commands in local terminals without asking.
pub fn local_terminal_ai_allowed(cx: &App) -> bool {
    mcp::events(cx).is_some_and(|events| events.read(cx).local_terminal)
}

/// What lives for the whole window regardless of the page: the MCP status
/// line, the approval dialog, settings, the update prompt, and the terminal's
/// AI assistant.
pub struct Overlays {
    status: Entity<mcp::StatusLine>,
    assistant: Entity<assistant::Assistant>,
    _subscriptions: Vec<Subscription>,
}

impl Overlays {
    pub fn new<V: 'static>(window: &mut Window, cx: &mut Context<V>) -> Self {
        let events = mcp::McpEvents::install(window, cx);
        let store = crate::ui::store::store(cx);
        let mut settings_open = false;
        let subscriptions = vec![
            // Tabs show whether AI may use local terminals.
            cx.observe(&events, |_, _, cx| cx.notify()),
            cx.observe_in(&store, window, move |_, store, window, cx| {
                let show = store.read(cx).show_settings;
                if show && !settings_open {
                    settings::open(window, cx);
                }
                settings_open = show;
            }),
        ];
        // Tests and offline setups can skip the request to GitHub. Version
        // 0.0.0 is a local build (releases get theirs from the tag), and every
        // release would look newer to it.
        let local_build = env!("CARGO_PKG_VERSION") == "0.0.0";
        if !local_build && std::env::var_os("NOVERTERM_SKIP_UPDATE_CHECK").is_none() {
            updater::check(window, cx).detach();
        }
        Self {
            status: cx.new(|cx| mcp::StatusLine::new(events, cx)),
            assistant: cx.new(|cx| assistant::Assistant::new(window, cx)),
            _subscriptions: subscriptions,
        }
    }

    /// The MCP status in the footer.
    pub fn status_line(&self) -> AnyElement {
        self.status.clone().into_any_element()
    }

    /// Layers drawn over the window. Dialogs open through the window's own
    /// dialog layer, so only the assistant panel is drawn here.
    pub fn layers(&self, _window: &mut Window, _cx: &mut App) -> Vec<AnyElement> {
        vec![self.assistant.clone().into_any_element()]
    }
}
