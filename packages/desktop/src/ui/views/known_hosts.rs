//! Known hosts: the SSH host fingerprints the user has trusted.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{h_flex, v_flex, Sizable as _};
use gpui_kit::*;

use crate::trust::{KnownHostsResponse, TrustedSshHost};
use crate::ui::bridge::backend;
use crate::ui::i18n::t;
use crate::ui::icons::lucide;
use crate::ui::store::store;
use crate::ui::theme;
use crate::ui::views::common;

pub struct KnownHostsView {
    hosts: Vec<TrustedSshHost>,
    loading: bool,
    error: Option<SharedString>,
}

impl KnownHostsView {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let app = store(cx).read(cx).app();
        cx.spawn(async move |this, cx| {
            let loaded =
                backend(async move { crate::trust::known_hosts_get(app.state()).await }).await;
            let _ = this.update(cx, |this, cx| this.loaded(loaded, cx));
        })
        .detach();
        Self {
            hosts: Vec::new(),
            loading: true,
            error: None,
        }
    }

    fn loaded(&mut self, result: Result<KnownHostsResponse, String>, cx: &mut Context<Self>) {
        self.loading = false;
        match result {
            Ok(response) => self.hosts = response.hosts,
            Err(error) => self.error = Some(error.into()),
        }
        cx.notify();
    }

    fn request_remove(
        &mut self,
        host: &TrustedSshHost,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.error = None;
        cx.notify();
        let (view, name, port) = (cx.entity(), host.host.clone(), host.port);
        common::confirm_delete(
            t("Remove Known Host?"),
            t("This removes the saved SSH fingerprint. The next connection to this host will ask you to trust it again."),
            Some(format!("{}:{}", host.host, host.port).into()),
            t("Remove host"),
            window,
            cx,
            move |_, cx| view.update(cx, |this, cx| this.remove(name.clone(), port, cx)),
        );
    }

    fn remove(&mut self, host: String, port: u16, cx: &mut Context<Self>) {
        let app = store(cx).read(cx).app();
        cx.spawn(async move |this, cx| {
            let removed =
                backend(
                    async move { crate::trust::known_hosts_remove(host, port, app.state()).await },
                )
                .await;
            let _ = this.update(cx, |this, cx| this.loaded(removed, cx));
        })
        .detach();
    }

    fn render_card(&self, host: &TrustedSshHost, cx: &Context<Self>) -> impl IntoElement {
        let address = format!("{}:{}", host.host, host.port);
        let remove = host.clone();
        common::card(SharedString::from(format!("known-host-{address}")))
            .child(
                h_flex()
                    .items_start()
                    .gap_3()
                    .child(common::icon_box("server"))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(address.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme::STENCIL_MUTE)
                                    .child(host.algorithm.clone()),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .mt_4()
                    .gap_2()
                    .px_3()
                    .py_2p5()
                    .rounded(theme::RADIUS)
                    .border_1()
                    .border_color(theme::hairline())
                    .bg(theme::RAIL)
                    .child(
                        h_flex()
                            .gap_2()
                            .text_color(theme::STENCIL_MUTE)
                            .child(lucide("fingerprint-pattern").xsmall())
                            .child(common::section_title(t("Fingerprint"))),
                    )
                    .child(div().text_xs().child(host.fingerprint.clone())),
            )
            .child(
                div().mt_4().flex().child(
                    Button::new(SharedString::from(format!("remove-{address}")))
                        .ghost()
                        .xsmall()
                        .icon(lucide("trash"))
                        .label(t("Remove"))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.request_remove(&remove, window, cx);
                        })),
                ),
            )
    }
}

impl Render for KnownHostsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = if self.loading {
            common::empty_state(t("Loading...")).into_any_element()
        } else if self.hosts.is_empty() {
            common::empty_state(t(
                "No Known Hosts yet. Confirm a host fingerprint during SSH connection to add it here.",
            ))
            .into_any_element()
        } else {
            div()
                .grid()
                .grid_cols(3)
                .gap_3()
                .children(self.hosts.iter().map(|host| self.render_card(host, cx)))
                .into_any_element()
        };

        common::page(
            t("Known Hosts"),
            div(),
            common::page_body("known-hosts")
                .children(
                    self.error
                        .clone()
                        .map(|error| common::error_banner(error).mb_4()),
                )
                .child(body),
        )
    }
}
