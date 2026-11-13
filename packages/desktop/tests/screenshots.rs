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
                    pin_color: None,
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

    /// Opens the main window offscreen at `width` x `height` points.
    fn open_stage(
        app: &desktop_lib::app::AppHandle,
        out: PathBuf,
        width: f32,
        height: f32,
    ) -> Stage {
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
                            size: size(px(width), px(height)),
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
        stage
    }

    pub fn run() {
        if std::env::var_os("NOVERTERM_SITE_SHOTS").is_some() {
            return site::run();
        }
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
        let mut stage = open_stage(&app, out, 1200., 800.);

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
            version: "9.9.9".to_string(),
            notes: Some("- Faster terminal rendering\n- Fixes for jump hosts".to_string()),
            platforms: Default::default(),
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

    /// The landing page's screenshots (docs/assets/shots), in English with demo
    /// data. Run by scripts/site-shots.sh, which starts the demo SSH hosts.
    mod site {
        use std::path::PathBuf;
        use std::time::Duration;

        use desktop_lib::ui::store::Route;

        use super::{open_stage, Stage};

        const PASSWORD: &str = "demo";

        struct Seeded {
            web_01: String,
            web_02: String,
            api: String,
            db_01: String,
        }

        fn seed(app: &desktop_lib::app::AppHandle) -> Seeded {
            use desktop_lib::store::{
                groups, hosts, keys, links, port_forwards, settings, snippets,
            };
            use shared::{
                LinkWriteRequest, PortForwardMappingInput, PortForwardWriteRequest, Setting,
                SnippetWriteRequest,
            };

            desktop_lib::app::runtime().block_on(async {
                let setting = |key: &str, value: &str| Setting {
                    key: key.to_string(),
                    value: value.to_string(),
                };
                settings::set_setting(setting("noverterm.locale", "en"), app.state())
                    .await
                    .expect("locale");
                settings::set_setting(setting("link_relay", "link.example.com"), app.state())
                    .await
                    .expect("relay");

                let group = |name: &'static str| async move {
                    groups::host_group_create(name.to_string(), app.state())
                        .await
                        .expect("group")
                        .id
                };
                let (prd, stg, uat) = (group("PRD").await, group("STG").await, group("UAT").await);
                let host = |name: &str, user: &str, address: &str, port: i32, group: &str| {
                    hosts::SaveConnectionInput {
                        id: None,
                        name: name.to_string(),
                        host: address.to_string(),
                        port,
                        username: user.to_string(),
                        group_id: Some(group.to_string()),
                        jump_host_id: None,
                        password: Some(PASSWORD.to_string()),
                        private_key: None,
                        passphrase: None,
                        key_name: None,
                        existing_key_id: None,
                        ai_access: None,
                    }
                };
                let mut ids = Vec::new();
                for input in [
                    // Nothing listens on 2209: that tab is the held one.
                    host("db-01", "postgres", "127.0.0.1", 2209, &prd),
                    host("web-01", "deploy", "127.0.0.1", 2201, &prd),
                    host("web-02", "deploy", "127.0.0.1", 2202, &prd),
                    host("api", "deploy", "127.0.0.1", 2203, &stg),
                    host("worker", "deploy", "10.0.2.14", 22, &stg),
                    host("web", "qa", "10.0.3.7", 22, &uat),
                ] {
                    ids.push(hosts::host_save(input, app.state()).await.expect("host").id);
                }

                // Host keys of the demo containers, from scripts/site-shots.sh.
                let trust = app.state::<desktop_lib::trust::SshTrustStore>().inner().clone();
                for entry in std::env::var("SITE_HOST_KEYS").unwrap_or_default().split(',') {
                    let Some((port, fingerprint)) = entry.split_once('=') else { continue };
                    trust
                        .confirm(desktop_lib::trust::HostTrustConfirmation {
                            host: "127.0.0.1".to_string(),
                            port: port.parse().expect("port"),
                            algorithm: "ssh-ed25519".to_string(),
                            fingerprint: fingerprint.to_string(),
                        })
                        .await
                        .expect("trust");
                }

                let snippet = |title: &str, body: &str, host: Option<&String>, pin: Option<&str>| {
                    SnippetWriteRequest {
                        host_id: host.cloned(),
                        title: title.to_string(),
                        body: body.to_string(),
                        pin_icon: pin.map(str::to_string),
                        pin_color: None,
                    }
                };
                for request in [
                    snippet("Disk usage", "df -h", None, Some("hard-drive")),
                    snippet("Tail nginx errors", "sudo tail -f /var/log/nginx/error.log", Some(&ids[1]), Some("activity")),
                    snippet("Who is on", "who", None, None),
                    snippet("Postgres activity", "psql -c 'select pid, state, query from pg_stat_activity'", Some(&ids[0]), Some("database")),
                    snippet("Restart app", "sudo systemctl restart app", Some(&ids[3]), None),
                ] {
                    snippets::snippet_create(request, app.state()).await.expect("snippet");
                }

                let mapping = |port: i32, target: &str, target_port: i32| PortForwardMappingInput {
                    bind_host: "127.0.0.1".to_string(),
                    bind_port: port,
                    target_host: target.to_string(),
                    target_port,
                };
                port_forwards::port_forward_preset_create(
                    PortForwardWriteRequest {
                        name: "Grafana + Postgres".to_string(),
                        host_id: ids[1].clone(),
                        mappings: vec![
                            mapping(3000, "127.0.0.1", 3000),
                            mapping(9090, "127.0.0.1", 9090),
                            mapping(5432, "10.0.1.9", 5432),
                        ],
                    },
                    app.state(),
                )
                .await
                .expect("forward");

                for (name, fingerprint) in [
                    ("id_ed25519", "SHA256:4fN0q8m2Yc1VbXr7tLwZp3HsKjE9uDgA6oQyTnRkQ7v"),
                    ("deploy_rsa", "SHA256:nThbg6kXUpJWGl7E1IGOCspRomTxdCARLviKw6E5SY8"),
                    ("bastion_ed25519", "SHA256:Xq3mVt9LpZr2Ke8Nw5HbYc7Ud1Sf4Ga6Jo0Ti3Rx5Mv"),
                ] {
                    keys::key_create(
                        keys::KeyInput {
                            name: name.to_string(),
                            kind: "inline".to_string(),
                            fingerprint: Some(fingerprint.to_string()),
                            private_key: "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXk\n-----END OPENSSH PRIVATE KEY-----".to_string(),
                            passphrase: None,
                        },
                        app.state(),
                    )
                    .await
                    .expect("key");
                }

                for (subdomain, port) in [("shop", 3000), ("webhooks", 8787), ("docs", 4321)] {
                    links::link_create(
                        LinkWriteRequest {
                            subdomain: subdomain.to_string(),
                            target_host: "127.0.0.1".to_string(),
                            target_port: port,
                        },
                        app.state(),
                    )
                    .await
                    .expect("link");
                }

                let mcp = desktop_lib::mcp::McpSettings { port: 47322, allow_local_terminal: false };
                desktop_lib::mcp::mcp_configure(app.clone(), mcp).await.expect("MCP server");

                Seeded {
                    db_01: ids[0].clone(),
                    web_01: ids[1].clone(),
                    web_02: ids[2].clone(),
                    api: ids[3].clone(),
                }
            })
        }

        /// A home folder worth showing in the SFTP page's local pane.
        fn demo_home() -> tempfile::TempDir {
            let home = tempfile::tempdir().expect("demo home");
            for dir in ["Projects", "Downloads", "backups"] {
                std::fs::create_dir(home.path().join(dir)).expect("dir");
            }
            for (file, size) in [
                ("release-2026.11.04.tar.gz", 48 << 20),
                ("schema.sql", 182 << 10),
                ("nginx.conf", 3 << 10),
                ("notes.md", 2 << 10),
            ] {
                std::fs::File::create(home.path().join(file))
                    .and_then(|file| file.set_len(size))
                    .expect("file");
            }
            home
        }

        pub fn run() {
            let data = tempfile::tempdir().expect("scratch data dir");
            let home = demo_home();
            std::env::set_var("NOVERTERM_DATA_DIR", data.path());
            std::env::set_var("HOME", home.path());
            std::env::set_var("SHELL", "/bin/sh");
            std::env::set_var("PS1", "deploy@laptop ~ $ ");
            std::env::set_var("NOVERTERM_SKIP_UPDATE_CHECK", "1");
            let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/site-shots");
            std::fs::create_dir_all(&out).expect("site shot dir");

            let app = desktop_lib::bootstrap::backend().expect("backend starts");
            let seeded = seed(&app);
            let mut stage = open_stage(&app, out, 1440., 810.);

            for (name, route) in [
                ("connections", Route::Connections),
                ("forwards", Route::Forwards),
                ("snippets", Route::Snippets),
                ("keys", Route::Keys),
                ("mcp", Route::Mcp),
            ] {
                stage.store(|store, cx| store.navigate(route, cx));
                stage.settle(Duration::from_millis(400));
                stage.shoot(name);
            }

            // A Link that is live, as the relay would report it. The page has to
            // be open first: its runtime starts listening there.
            stage.store(|store, cx| store.navigate(Route::Links, cx));
            stage.settle(Duration::from_millis(400));
            let shop = stage.store(|store, _| {
                store
                    .links
                    .iter()
                    .find(|link| link.subdomain == "shop")
                    .map(|link| link.id.clone())
            });
            if let Some(id) = shop {
                let status = desktop_lib::runtime::link::LinkStatus {
                    id,
                    url: "https://shop.link.example.com".to_string(),
                    state: desktop_lib::runtime::link::LinkState::Online,
                    error: None,
                };
                app.emit("link_status", status).expect("emit");
            }
            stage.settle(Duration::from_millis(400));
            stage.shoot("links");

            // Tabs in hero order; web-01 runs htop and ends up in front.
            stage.store(|store, cx| store.connect_host(&seeded.db_01, None, cx));
            stage.store(|store, cx| {
                store.connect_host(&seeded.web_01, Some("htop".to_string()), cx)
            });
            stage.store(|store, cx| store.connect_host(&seeded.web_02, None, cx));
            stage.store(|store, cx| store.connect_host(&seeded.api, None, cx));
            stage.store(|store, cx| store.connect_local(None, None, cx));
            stage.settle(Duration::from_millis(4000));
            focus(&mut stage, 0);
            stage.settle(Duration::from_millis(400));
            stage.shoot("held");
            focus(&mut stage, 1);
            stage.settle(Duration::from_millis(1500));
            stage.shoot("hero");

            stage.store(|store, cx| store.navigate(Route::Sftp, cx));
            stage.settle(Duration::from_millis(2500));
            stage.shoot("sftp");
        }

        /// Brings the `index`th tab to the front.
        fn focus(stage: &mut Stage, index: usize) {
            let id =
                stage.store(|store, _| store.sessions.get(index).map(|session| session.id.clone()));
            stage.store(|store, cx| {
                store.set_active(id, cx);
                store.navigate(Route::Terminal, cx);
            });
        }
    }
}
