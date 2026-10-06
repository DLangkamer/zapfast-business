//! Connect to Remote ZapFast Server Dialog.
//!
//! Allows scanning the local network for ZapFast Docker servers, entering credentials,
//! and connecting the desktop app.

use std::io::Read;
use egui::{Layout, Vec2};
use crate::app::App;
use crate::model::Action;
use crate::theme::{self, Icon};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;

    ui.vertical(|ui| {
        // Header
        ui.horizontal(|ui| {
            theme::icon(ui, Icon::Globe, 22.0, palette.accent);
            ui.add_space(4.0);
            theme::text(
                ui,
                "Conectar ao Servidor ZapFast",
                theme::bold(18.0),
                palette.text,
            );
        });

        ui.add_space(6.0);
        theme::paragraph(
            ui,
            "Conecte este aplicativo a um servidor ZapFast Docker na rede local ou em nuvem para acessar suas contas centralizadas.",
            theme::regular(13.0),
            palette.secondary,
        );

        ui.add_space(14.0);

        // LAN Auto-Discovery Section
        ui.group(|ui| {
            ui.horizontal(|ui| {
                theme::icon(ui, Icon::Search, 16.0, palette.accent);
                theme::text(ui, "Descoberta na Rede Local (LAN)", theme::bold(13.5), palette.text);
            });
            ui.add_space(4.0);
            theme::paragraph(
                ui,
                "Localize automaticamente servidores ZapFast rodando no Docker na mesma rede Wi-Fi/cabeada:",
                theme::regular(12.0),
                palette.secondary,
            );
            ui.add_space(6.0);

            if theme::pill_button(ui, &palette, "🔍 Buscar Servidores na Rede Local", false).clicked() {
                app.server_status_msg = Some("Buscando servidores na rede local...".to_string());
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    let servers = runtime.block_on(crate::remote::discover_local_servers(
                        std::time::Duration::from_millis(1500),
                    ));
                    let _ = tx.send(servers);
                });

                if let Ok(servers) = rx.recv_timeout(std::time::Duration::from_millis(1600)) {
                    if servers.is_empty() {
                        app.server_status_msg = Some("Nenhum servidor encontrado na porta 47120. Verifique se o Docker está ativo.".to_string());
                    } else {
                        app.server_status_msg = Some(format!("{} servidor(es) encontrado(s)!", servers.len()));
                        app.server_discovered = servers;
                    }
                }
            }

            if !app.server_discovered.is_empty() {
                ui.add_space(6.0);
                theme::text(ui, "Servidores Encontrados:", theme::bold(12.5), palette.accent);
                for server in &app.server_discovered {
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::Check, 14.0, palette.accent);
                        theme::text(ui, &format!("{} ({})", server.name, server.address()), theme::regular(12.5), palette.text);
                        if theme::pill_button(ui, &palette, "Usar Este", true).clicked() {
                            app.server_url_input = server.address();
                        }
                    });
                }
            }
        });

        ui.add_space(14.0);

        // Connection credentials
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Endereço do Servidor:").font(theme::medium(13.0)).color(palette.text));
        });
        ui.add(
            egui::TextEdit::singleline(&mut app.server_url_input)
                .hint_text("127.0.0.1:8080 ou 192.168.1.100:8080")
                .desired_width(f32::INFINITY),
        );

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Usuário:").font(theme::medium(13.0)).color(palette.text));
        });
        ui.add(
            egui::TextEdit::singleline(&mut app.server_username_input)
                .hint_text("Ex: admin ou seu_login")
                .desired_width(f32::INFINITY),
        );

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Senha:").font(theme::medium(13.0)).color(palette.text));
        });
        ui.add(
            egui::TextEdit::singleline(&mut app.server_password_input)
                .password(true)
                .hint_text("••••••••")
                .desired_width(f32::INFINITY),
        );

        if let Some(msg) = &app.server_status_msg {
            ui.add_space(8.0);
            theme::paragraph(ui, msg, theme::regular(12.5), palette.accent);
        }

        ui.add_space(18.0);

        // Bottom Actions
        ui.horizontal(|ui| {
            if theme::pill_button(ui, &palette, "Cancelar", false).clicked() {
                app.actions.push(Action::CloseDialog);
            }

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::pill_button(ui, &palette, "Conectar", true).clicked() {
                    let addr = app.server_url_input.trim().to_string();
                    let user = app.server_username_input.trim().to_string();
                    let pass = app.server_password_input.trim().to_string();

                    if addr.is_empty() || user.is_empty() || pass.is_empty() {
                        app.server_status_msg = Some("Preencha todos os campos para conectar.".to_string());
                    } else {
                        app.server_status_msg = Some("Conectando ao servidor...".to_string());
                        let http_url = if addr.starts_with("http://") || addr.starts_with("https://") {
                            format!("{}/api/login", addr.trim_end_matches('/'))
                        } else {
                            format!("http://{}/api/login", addr.trim_end_matches('/'))
                        };

                        let body = serde_json::json!({
                            "username": user,
                            "password": pass,
                        }).to_string();

                        match ureq::post(&http_url)
                            .header("Content-Type", "application/json")
                            .send_bytes(body.as_bytes())
                        {
                            Ok(mut resp) => {
                                let mut text = String::new();
                                if let Ok(_) = resp.body_mut().read_to_string(&mut text) {
                                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                                        if let Some(tok) = val.get("token").and_then(|v| v.as_str()) {
                                            app.server_status_msg = Some("✔ Autenticado com sucesso no servidor!".to_string());
                                            app.toast(format!("Conectado ao servidor como {user}"));
                                            app.actions.push(Action::CloseDialog);
                                        } else {
                                            app.server_status_msg = Some("Erro: Resposta inesperada do servidor".to_string());
                                        }
                                    } else {
                                        app.server_status_msg = Some("Erro ao processar resposta do servidor".to_string());
                                    }
                                }
                            }
                            Err(e) => {
                                app.server_status_msg = Some(format!("Falha na conexão: {e}"));
                            }
                        }
                    }
                }
            });
        });
    })
}
