//! Diagnostic inspection for real lead ad attribution metadata.
//!
//! Conforms strictly to ZapFast privacy boundaries:
//! Never displays or accesses conversation message text. Only inspects
//! captured ad attribution metadata (ad_id, ctwa_clid, source_type, timestamps)
//! to verify whether WhatsApp delivered the Click-to-WhatsApp fields.

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, vec2};

use super::crm_sidecar::format_currency;
use crate::app::App;
use crate::model::{ContactOrigin, CrmDeal};
use crate::theme::{self, Icon, Palette};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let search_id = egui::Id::new("lead_diagnostic_search");
    let filter_id = egui::Id::new("lead_diagnostic_filter");

    let mut search_str: String = ui.data_mut(|d| d.get_temp(search_id)).unwrap_or_default();
    let mut filter_mode: usize = ui.data_mut(|d| d.get_temp(filter_id)).unwrap_or(0); // 0: Todos, 1: Com ctwa_clid, 2: Sem ctwa_clid, 3: CAPI Enviado

    ui.vertical(|ui| {
        // Header
        ui.horizontal(|ui| {
            theme::icon(ui, Icon::Search, 20.0, palette.accent);
            theme::text(
                ui,
                "Diagnóstico de Atribuição de Leads (Metadados Reais)",
                theme::bold(17.0),
                palette.text,
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::icon_button(ui, Icon::X, 18.0, palette.secondary, palette.text, "Fechar")
                    .clicked()
                {
                    app.dialog = None;
                }
            });
        });

        ui.add_space(4.0);
        theme::text(
            ui,
            "Inspeciona quais campos de metadados de anúncio e clique (CTWA) foram recebidos do WhatsApp por lead. Respeita a privacidade: não acessa nem exibe o texto de conversas.",
            theme::regular(12.0),
            palette.dim,
        );

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Aggregate statistics
        let total_leads = app.contact_origins.len();
        let with_clid = app
            .contact_origins
            .values()
            .filter(|o| o.ctwa_clid.as_ref().is_some_and(|c| !c.trim().is_empty()))
            .count();
        let with_ad_id = app
            .contact_origins
            .values()
            .filter(|o| o.ad_id.as_ref().is_some_and(|a| !a.trim().is_empty()))
            .count();
        let capi_sent = app
            .meta_conversions
            .values()
            .filter(|c| c.status == "sent")
            .count();
        let capi_pending = app
            .meta_conversions
            .values()
            .filter(|c| c.status.starts_with("pending"))
            .count();

        // KPI chips row
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
            render_chip(ui, &palette, "Total de Leads Mapeados", &total_leads.to_string(), palette.text);
            render_chip(ui, &palette, "Com ctwa_clid (Clique direto)", &with_clid.to_string(), palette.accent);
            render_chip(ui, &palette, "Com ID de Anúncio", &with_ad_id.to_string(), palette.secondary);
            render_chip(ui, &palette, "Conversões Meta Enviadas", &capi_sent.to_string(), palette.accent);
            if capi_pending > 0 {
                render_chip(ui, &palette, "Conversões Pendentes", &capi_pending.to_string(), palette.warning);
            }
        });

        ui.add_space(10.0);

        // Filter & Search bar
        ui.horizontal(|ui| {
            theme::icon(ui, Icon::Search, 15.0, palette.dim);
            ui.add(
                egui::TextEdit::singleline(&mut search_str)
                    .hint_text("Buscar por contato, anúncio, campanha ou clid...")
                    .desired_width(260.0),
            );

            ui.add_space(8.0);
            if ui.selectable_label(filter_mode == 0, "Todos").clicked() {
                filter_mode = 0;
            }
            if ui.selectable_label(filter_mode == 1, "Com ctwa_clid").clicked() {
                filter_mode = 1;
            }
            if ui.selectable_label(filter_mode == 2, "Sem ctwa_clid").clicked() {
                filter_mode = 2;
            }
            if ui.selectable_label(filter_mode == 3, "Meta Enviado").clicked() {
                filter_mode = 3;
            }
        });

        ui.add_space(8.0);

        // Lead rows table
        let search_lower = search_str.trim().to_lowercase();
        let mut origins: Vec<_> = app.contact_origins.values().cloned().collect();
        origins.sort_by_key(|o| -o.first_contact_at);

        ScrollArea::vertical()
            .max_height(420.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;

                let mut displayed = 0;
                for orig in origins {
                    let has_clid = orig.ctwa_clid.as_ref().is_some_and(|c| !c.trim().is_empty());
                    let conv_status = app
                        .meta_conversions
                        .get(&orig.chat_id)
                        .map(|c| c.status.as_str())
                        .unwrap_or("none");

                    // Filter mode
                    match filter_mode {
                        1 if !has_clid => continue,
                        2 if has_clid => continue,
                        3 if conv_status != "sent" => continue,
                        _ => {}
                    }

                    // Search filter
                    let chat_title = app
                        .chat(&orig.chat_id)
                        .map(|c| app.chat_title(c))
                        .unwrap_or_else(|| orig.chat_id.clone());
                    let search_haystack = format!(
                        "{} {} {} {} {} {}",
                        chat_title,
                        orig.chat_id,
                        orig.ad_name,
                        orig.campaign_name,
                        orig.title,
                        orig.ctwa_clid.as_deref().unwrap_or_default()
                    )
                    .to_lowercase();

                    if !search_lower.is_empty() && !search_haystack.contains(&search_lower) {
                        continue;
                    }

                    let deal = app.crm_deals.get(&orig.chat_id);
                    displayed += 1;
                    render_diagnostic_row(ui, &palette, &orig, &chat_title, conv_status, deal);
                }

                if displayed == 0 {
                    ui.vertical_centered(|ui| {
                        ui.add_space(24.0);
                        ui.label(
                            RichText::new("Nenhum registro encontrado para os filtros selecionados.")
                                .size(13.0)
                                .color(palette.dim),
                        );
                        ui.add_space(24.0);
                    });
                }
            });
    });

    ui.data_mut(|d| {
        d.insert_temp(search_id, search_str);
        d.insert_temp(filter_id, filter_mode);
    });
}

fn render_chip(ui: &mut egui::Ui, palette: &Palette, label: &str, value: &str, val_color: Color32) {
    Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.outline))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).size(11.0).color(palette.dim));
                ui.label(RichText::new(value).size(12.0).strong().color(val_color));
            });
        });
}

fn render_diagnostic_row(
    ui: &mut egui::Ui,
    palette: &Palette,
    orig: &ContactOrigin,
    chat_title: &str,
    conv_status: &str,
    deal: Option<&CrmDeal>,
) {
    let date_str = crate::util::local_datetime(orig.first_contact_at)
        .map(|(d, h, m)| format!("{:02}/{:02} {:02}:{:02}", d.day(), d.month(), h, m))
        .unwrap_or_else(|| "-".to_string());

    Frame::new()
        .fill(palette.panel)
        .stroke(Stroke::new(1.0, palette.outline))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // Top row: contact name, first contact date, and CAPI badge
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::User, 14.0, palette.accent);
                    ui.label(RichText::new(chat_title).size(13.5).strong().color(palette.text));
                    ui.label(RichText::new(format!("({})", orig.chat_id)).size(11.0).color(palette.dim));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(date_str).size(11.0).color(palette.dim));

                        // CAPI status badge
                        match conv_status {
                            "sent" => {
                                Frame::new()
                                    .fill(palette.accent.gamma_multiply(0.15))
                                    .corner_radius(CornerRadius::same(4))
                                    .inner_margin(Margin::symmetric(6, 2))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new("✓ Meta CAPI Enviado").size(10.5).color(palette.accent));
                                    });
                            }
                            "pending_confirmation" => {
                                Frame::new()
                                    .fill(palette.warning.gamma_multiply(0.15))
                                    .corner_radius(CornerRadius::same(4))
                                    .inner_margin(Margin::symmetric(6, 2))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new("⏳ Aguardando Confirmação").size(10.5).color(palette.warning));
                                    });
                            }
                            "pending_no_click" => {
                                Frame::new()
                                    .fill(palette.dim.gamma_multiply(0.15))
                                    .corner_radius(CornerRadius::same(4))
                                    .inner_margin(Margin::symmetric(6, 2))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new("Pendente (Sem clique rastreado)").size(10.5).color(palette.dim));
                                    });
                            }
                            "error" => {
                                Frame::new()
                                    .fill(palette.danger.gamma_multiply(0.15))
                                    .corner_radius(CornerRadius::same(4))
                                    .inner_margin(Margin::symmetric(6, 2))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new("✕ Falha CAPI").size(10.5).color(palette.danger));
                                    });
                            }
                            _ => {}
                        }
                    });
                });

                ui.add_space(4.0);

                // Origin / Ad / ctwa_clid metadata details
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 4.0);

                    // Origin / Campaign
                    let camp_label = orig.display_campaign();
                    Frame::new()
                        .fill(palette.surface)
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(6, 2))
                        .show(ui, |ui| {
                            ui.label(RichText::new(format!("Campanha: {}", camp_label)).size(11.0).color(palette.secondary));
                        });

                    // Ad ID
                    if let Some(ad_id) = &orig.ad_id {
                        Frame::new()
                            .fill(palette.surface)
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("ad_id: {}", ad_id)).size(11.0).color(palette.accent));
                            });
                    }

                    // Headline / Title
                    if !orig.title.is_empty() {
                        Frame::new()
                            .fill(palette.surface)
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("Criativo: {}", orig.title)).size(11.0).color(palette.text));
                            });
                    }

                    // ctwa_clid badge
                    if let Some(clid) = &orig.ctwa_clid {
                        let short_clid = if clid.len() > 16 {
                            format!("{}…{}", &clid[..8], &clid[clid.len() - 6..])
                        } else {
                            clid.clone()
                        };
                        Frame::new()
                            .fill(palette.accent.gamma_multiply(0.12))
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("ctwa_clid: {}", short_clid)).size(11.0).strong().color(palette.accent));
                            });
                    } else {
                        Frame::new()
                            .fill(palette.dim.gamma_multiply(0.12))
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(RichText::new("ctwa_clid: Ausente").size(11.0).color(palette.dim));
                            });
                    }

                    // Capture source type
                    let source_label = match orig.source_type.as_str() {
                        "ctwa_context" => "Contexto CTWA (Nativo)",
                        "ctwa_url" => "Parâmetros de URL",
                        "text_rule" => "Regra de Texto (Plano B)",
                        _ => "Não identificado / Orgânico",
                    };
                    Frame::new()
                        .fill(palette.surface)
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(6, 2))
                        .show(ui, |ui| {
                            ui.label(RichText::new(source_label).size(10.5).color(palette.dim));
                        });

                    // Deal value if present
                    if let Some(d) = deal {
                        if d.value_cents > 0 {
                            Frame::new()
                                .fill(palette.accent.gamma_multiply(0.12))
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(format_currency(d.value_cents)).size(11.0).strong().color(palette.accent));
                                });
                        }
                    }
                });
            });
        });
}
