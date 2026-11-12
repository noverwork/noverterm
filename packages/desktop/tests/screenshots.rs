//! Renders the real window offscreen and saves PNGs, so a change to the UI can
//! be looked at without a display. Needs a GPU: Metal on macOS, WGPU on Linux.
//!
//! `cargo test -p desktop --test screenshots` writes to `target/screenshots/`.

fn main() {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    shots::run();
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    println!("screenshots: skipped; no headless renderer on this platform");
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod shots {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    use desktop_lib::ui::{self, store::Route};
    use gpui_kit::{
        px, size, AnyWindowHandle, Bounds, HeadlessAppContext, WindowBounds, WindowOptions,
    };

    struct Stage {
        cx: HeadlessAppContext,
        window: AnyWindowHandle,
        out: PathBuf,
    }

    impl Stage {
        /// Lets backend work finish and its results reach the views.
        fn settle(&mut self, duration: Duration) {
            let steps = (duration.as_millis() / 50).max(1);
            for _ in 0..steps {
                std::thread::sleep(Duration::from_millis(50));
                self.cx.run_until_parked();
            }
        }

        fn shoot(&mut self, name: &str) {
            self.cx
                .update_window(self.window, |_, window, cx| {
                    window.refresh();
                    let _ = window.draw(cx);
                })
                .ok();
            self.cx.run_until_parked();
            let image = self
                .cx
                .capture_screenshot(self.window)
                .expect("offscreen rendering must be available");
            let path = self.out.join(format!("{name}.png"));
            image.save(&path).expect("save screenshot");
            println!("screenshot: {}", path.display());
        }

        fn close_dialogs(&mut self) {
            use gpui_kit::component::WindowExt as _;
            self.cx
                .update_window(self.window, |_, window, cx| window.close_all_dialogs(cx))
                .ok();
            self.settle(Duration::from_millis(400));
        }

        fn store<R>(
            &mut self,
            update: impl FnOnce(&mut ui::store::Store, &mut gpui_kit::Context<ui::store::Store>) -> R,
        ) -> R {
            self.cx.update(|cx| ui::store::store(cx).update(cx, update))
        }
    }

    /// A little of everything, so no page renders empty.
    fn seed(app: &desktop_lib::app::AppHandle) {
        use desktop_lib::store::{groups, hosts, keys, port_forwards, snippets};
        use shared::{PortForwardMappingInput, PortForwardWriteRequest, SnippetWriteRequest};

        desktop_lib::app::runtime().block_on(async {
            let group = groups::host_group_create("Production".to_string(), app.state())
                .await
                .expect("group");
            let host = |name: &str, address: &str, group_id: Option<String>| hosts::SaveConnectionInput {
                id: None,
                name: name.to_string(),
                host: address.to_string(),
                port: 22,
                username: "deploy".to_string(),
                group_id,
                jump_host_id: None,
                password: Some("secret".to_string()),
                private_key: None,
                passphrase: None,
                key_name: None,
                existing_key_id: None,
                ai_access: None,
            };
            let api = hosts::host_save(host("api-01", "10.0.0.11", Some(group.id.clone())), app.state())
                .await
                .expect("host");
            hosts::host_save(host("db-01", "10.0.0.21", Some(group.id)), app.state())
                .await
                .expect("host");
            hosts::host_save(host("staging", "staging.example.com", None), app.state())
                .await
                .expect("host");
            let snippet = |title: &str, body: &str, host_id: Option<String>, pin_icon: Option<&str>| {
                SnippetWriteRequest {
                    host_id,
                    title: title.to_string(),
                    body: body.to_string(),
                    pin_icon: pin_icon.map(str::to_string),
                }
            };
            for request in [
                snippet("Restart Nginx", "sudo systemctl restart nginx\nsudo systemctl status nginx", Some(api.id.clone()), None),
                snippet("k9s", "k9s", None, Some("k9s")),
                snippet("Tail logs", "journalctl -fu api", Some(api.id.clone()), Some("activity")),
            ] {
                snippets::snippet_create(request, app.state()).await.expect("snippet");
            }
            port_forwards::port_forward_preset_create(
                PortForwardWriteRequest {
                    name: "Postgres".to_string(),
                    host_id: api.id,
                    mappings: vec![PortForwardMappingInput {
                        bind_host: "127.0.0.1".to_string(),
                        bind_port: 5432,
                        target_host: "10.0.0.21".to_string(),
                        target_port: 5432,
                    }],
                },
                app.state(),
            )
            .await
            .expect("forward");
            keys::key_create(
                keys::KeyInput {
                    name: "Deploy key".to_string(),
                    kind: "inline".to_string(),
                    fingerprint: Some("SHA256:nThbg6kXUpJWGl7E1IGOCspRomTxdCARLviKw6E5SY8".to_string()),
                    private_key: "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXk\n-----END OPENSSH PRIVATE KEY-----".to_string(),
                    passphrase: Some("hunter2".to_string()),
                },
                app.state(),
            )
            .await
            .expect("key");
            app.state::<desktop_lib::trust::SshTrustStore>()
                .inner()
                .confirm(desktop_lib::trust::HostTrustConfirmation {
                    host: "10.0.0.11".to_string(),
                    port: 22,
                    algorithm: "ssh-ed25519".to_string(),
                    fingerprint: "SHA256:nThbg6kXUpJWGl7E1IGOCspRomTxdCARLviKw6E5SY8".to_string(),
                })
                .await
                .expect("known host");
            // The default port may belong to a running noverterm.
            let mcp = desktop_lib::mcp::McpSettings { port: 47399, allow_local_terminal: false };
            if let Err(error) = desktop_lib::mcp::mcp_configure(app.clone(), mcp).await {
                println!("screenshots: MCP server not started: {error}");
            }
        });
    }

    pub fn run() {
        let data = tempfile::tempdir().expect("scratch data dir");
        std::env::set_var("NOVERTERM_DATA_DIR", data.path());
        // A predictable prompt instead of the developer's own shell setup.
        std::env::set_var("SHELL", "/bin/sh");
        // Screenshots must not depend on what GitHub answers.
        std::env::set_var("NOVERTERM_SKIP_UPDATE_CHECK", "1");
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots");
        std::fs::create_dir_all(&out).expect("screenshot dir");

        let app = desktop_lib::bootstrap::backend().expect("backend starts");
        seed(&app);
        let mut cx = HeadlessAppContext::with_platform(
            gpui_kit::platform::current_platform(true).text_system(),
            Arc::new(ui::Assets),
            gpui_kit::platform::current_headless_renderer,
        );
        cx.allow_parking();
        let window = cx
            .update(|cx| {
                ui::init(app.clone(), cx);
                // Dialogs would otherwise be caught mid-entrance.
                cx.set_reduce_motion(true);
                ui::open_main_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(Bounds {
                            origin: Default::default(),
                            size: size(px(1200.), px(800.)),
                        })),
                        focus: false,
                        show: false,
                        ..Default::default()
                    },
                    cx,
                )
            })
            .expect("window opens");
        let mut stage = Stage { cx, window, out };
        stage.settle(Duration::from_millis(500));

        for (name, route) in [
            ("connections", Route::Connections),
            ("keys", Route::Keys),
            ("snippets", Route::Snippets),
            ("forwards", Route::Forwards),
            ("known-hosts", Route::KnownHosts),
            ("sftp", Route::Sftp),
            ("mcp", Route::Mcp),
            ("connection-form", Route::ConnectionForm(None)),
            ("key-form", Route::KeyForm(None)),
            ("forward-form", Route::ForwardForm(None)),
            (
                "snippet-form",
                Route::SnippetForm {
                    id: None,
                    shortcut: true,
                },
            ),
        ] {
            stage.store(|store, cx| store.navigate(route, cx));
            stage.settle(Duration::from_millis(300));
            stage.shoot(name);
        }

        let key = stage.store(|store, _| store.keys.first().map(|key| key.id.clone()));
        let forward =
            stage.store(|store, _| store.forwards.first().map(|forward| forward.id.clone()));
        for (name, route) in [
            ("key-edit", Route::KeyForm(key)),
            ("forward-edit", Route::ForwardForm(forward)),
        ] {
            stage.store(|store, cx| store.navigate(route, cx));
            stage.settle(Duration::from_millis(300));
            stage.shoot(name);
        }
        stage.store(|store, cx| {
            store.show_settings = true;
            cx.notify();
        });
        stage.settle(Duration::from_millis(500));
        stage.shoot("settings");
        stage.close_dialogs();
        stage.store(|store, cx| {
            store.show_settings = false;
            cx.notify();
        });

        let release = ui::views::updater::Release {
            tag_name: "v9.9.9".to_string(),
            html_url: "https://example.com".to_string(),
            body: Some("- Faster terminal rendering\n- Fixes for jump hosts".to_string()),
        };
        stage
            .cx
            .update_window(stage.window, |_, window, cx| {
                ui::views::updater::show(release, window, cx)
            })
            .ok();
        stage.settle(Duration::from_millis(500));
        stage.shoot("update");
        stage.close_dialogs();

        for (status, exit_status, command) in [
            ("finished", Some(0), "uptime"),
            ("denied", None, "rm -rf /tmp/cache"),
        ] {
            let activity = desktop_lib::mcp::McpActivity {
                host_name: "api-01".to_string(),
                command: command.to_string(),
                status: status.to_string(),
                exit_status,
            };
            app.emit("mcp_activity", activity).expect("emit");
        }
        stage.settle(Duration::from_millis(300));
        stage.shoot("mcp-activity");
        for command in ["sudo systemctl restart nginx", "df -h"] {
            let request = desktop_lib::mcp::McpApprovalRequest {
                id: command.to_string(),
                host_name: "api-01".to_string(),
                target: "deploy@10.0.0.11:22".to_string(),
                command: command.to_string(),
            };
            app.emit("mcp_approval_request", request).expect("emit");
        }
        stage.settle(Duration::from_millis(500));
        stage.shoot("mcp-approval");
        stage.close_dialogs();

        stage.store(|store, cx| store.connect_local(None, None, cx));
        stage.settle(Duration::from_millis(1500));
        let id = stage.store(|store, _| store.active_session.clone());
        if let Some(id) = id {
            let command = "printf 'plain \\033[31mred\\033[0m \\033[1;32mbold\\033[0m \\033[7minverse\\033[0m 中文寬字 ok\\n'\n";
            stage.store(|store, cx| store.write_session(&id, command.to_string(), cx));
        }
        stage.settle(Duration::from_millis(1000));
        // Tab must reach the shell rather than move focus: `cat -vt` shows it as ^I.
        if let Some(id) = stage.store(|store, _| store.active_session.clone()) {
            stage.store(|store, cx| store.write_session(&id, "cat -vt\n".to_string(), cx));
            stage.settle(Duration::from_millis(300));
            stage
                .cx
                .update_window(stage.window, |_, window, cx| {
                    window.dispatch_keystroke(
                        gpui_kit::Keystroke::parse("tab").expect("keystroke"),
                        cx,
                    );
                    window.dispatch_keystroke(
                        gpui_kit::Keystroke::parse("enter").expect("keystroke"),
                        cx,
                    );
                })
                .ok();
        }
        stage.settle(Duration::from_millis(1000));
        stage.shoot("terminal");

        if let Some(id) = stage.store(|store, _| store.active_session.clone()) {
            stage.store(|_, cx| cx.emit(ui::store::StoreEvent::Assistant(id)));
            stage.settle(Duration::from_millis(300));
            stage.shoot("assistant");
        }
    }
}
