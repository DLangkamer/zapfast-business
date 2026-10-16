//! UI dialog for Plan B text-based attribution rules.

use egui::{Layout, RichText, Stroke, TextEdit};

use crate::app::App;
use crate::model::{Action, TextAttributionRule};

#[derive(Clone, Debug)]
struct FormState {
    pattern: String,
    match_type_idx: usize,
    ad_id: String,
    ad_name: String,
    campaign_name: String,
}

impl Default for FormState {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            match_type_idx: 1, // Contains by default
            ad_id: String::new(),
            ad_name: String::new(),
            campaign_name: String::new(),
        }
    }
}

pub fn manager(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let form_id = egui::Id::new("text_attribution_rule_form");
    let mut form: FormState = ui
        .ctx()
        .data_mut(|d| d.get_temp(form_id))
        .unwrap_or_default();

    ui.vertical(|ui| {
        // Title & Header
        ui.horizontal(|ui| {
            ui.heading(
                RichText::new("Regras de Atribuição por Texto (Plano B)")
                    .strong()
                    .color(palette.text),
            );
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new("✕").size(14.0).color(palette.dim))
                    .clicked()
                {
                    app.actions.push(Action::CloseDialog);
                }
            });
        });

        ui.label(
            RichText::new(
                "Associa leads automaticamente a campanhas e anúncios quando o lead envia a \
                 mensagem padrão de saudação configurada no Meta Ads.",
            )
            .size(12.0)
            .color(palette.dim),
        );

        ui.add_space(10.0);

        // Form to add a new rule
        egui::Frame::new()
            .fill(palette.surface)
            .stroke(Stroke::new(1.0, palette.surface_hover))
            .corner_radius(8.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Nova Regra de Correspondência")
                        .strong()
                        .size(13.0)
                        .color(palette.text),
                );
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Texto ou Palavra-chave:").size(12.0).color(palette.text));
                    ui.add_sized(
                        [ui.available_width(), 24.0],
                        TextEdit::singleline(&mut form.pattern)
                            .hint_text("ex: Quero saber mais sobre o Curso de Rust"),
                    );
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Tipo de Correspondência:").size(12.0).color(palette.text));
                    let match_labels = ["Exato", "Contém", "Começa com"];
                    for (i, label) in match_labels.iter().enumerate() {
                        let selected = form.match_type_idx == i;
                        if ui.selectable_label(selected, *label).clicked() {
                            form.match_type_idx = i;
                        }
                    }
                });

                ui.add_space(4.0);

                ui.columns(3, |cols| {
                    cols[0].label(RichText::new("Nome do Anúncio:").size(11.0).color(palette.dim));
                    cols[0].add(
                        TextEdit::singleline(&mut form.ad_name)
                            .hint_text("ex: Criativo Vídeo 01"),
                    );

                    cols[1].label(RichText::new("Nome da Campanha:").size(11.0).color(palette.dim));
                    cols[1].add(
                        TextEdit::singleline(&mut form.campaign_name)
                            .hint_text("ex: Lançamento Outubro"),
                    );

                    cols[2].label(RichText::new("ID do Anúncio (Opcional):").size(11.0).color(palette.dim));
                    cols[2].add(
                        TextEdit::singleline(&mut form.ad_id)
                            .hint_text("ex: 12020812345"),
                    );
                });

                ui.add_space(8.0);

                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_save = !form.pattern.trim().is_empty()
                        && (!form.ad_name.trim().is_empty() || !form.campaign_name.trim().is_empty());

                    let save_btn = ui.add_enabled(
                        can_save,
                        egui::Button::new(
                            RichText::new("+ Adicionar Regra")
                                .strong()
                                .color(if can_save { palette.on_accent } else { palette.dim }),
                        )
                        .fill(if can_save { palette.accent } else { palette.surface_hover }),
                    );

                    if save_btn.clicked() {
                        let match_type = match form.match_type_idx {
                            0 => "exact".to_string(),
                            2 => "starts_with".to_string(),
                            _ => "contains".to_string(),
                        };

                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);

                        let rule = TextAttributionRule {
                            id: 0,
                            ad_id: if form.ad_id.trim().is_empty() {
                                None
                            } else {
                                Some(form.ad_id.trim().to_string())
                            },
                            ad_name: form.ad_name.trim().to_string(),
                            campaign_name: form.campaign_name.trim().to_string(),
                            adset_name: String::new(),
                            pattern: form.pattern.trim().to_string(),
                            match_type,
                            case_sensitive: false,
                            active: true,
                            created_at: now,
                        };

                        app.actions.push(Action::SaveTextAttributionRule(rule));
                        form = FormState::default();
                    }
                });
            });

        ui.add_space(10.0);

        // List existing rules
        ui.label(
            RichText::new(format!(
                "Regras Ativas ({})",
                app.text_attribution_rules.len()
            ))
            .strong()
            .size(13.0)
            .color(palette.text),
        );
        ui.add_space(4.0);

        if app.text_attribution_rules.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(16.0);
                ui.label(
                    RichText::new("Nenhuma regra de texto cadastrada.")
                        .italics()
                        .color(palette.dim),
                );
                ui.label(
                    RichText::new(
                        "Cadastre os textos padrão de seus anúncios do Meta para capturar leads mesmo se a API não entregar os metadados.",
                    )
                    .size(11.0)
                    .color(palette.dim),
                );
                ui.add_space(16.0);
            });
        } else {
            egui::ScrollArea::vertical()
                .max_height(240.0)
                .show(ui, |ui| {
                    let mut delete_id = None;

                    for rule in &app.text_attribution_rules {
                        egui::Frame::new()
                            .fill(palette.surface)
                            .stroke(Stroke::new(1.0, palette.surface_hover))
                            .corner_radius(6.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!("\"{}\"", rule.pattern))
                                                    .strong()
                                                    .color(palette.text),
                                            );
                                            let type_badge = match rule.match_type.as_str() {
                                                "exact" => "Exato",
                                                "starts_with" => "Começa com",
                                                _ => "Contém",
                                            };
                                            ui.label(
                                                RichText::new(format!("[{type_badge}]"))
                                                    .size(11.0)
                                                    .color(palette.accent),
                                            );
                                        });

                                        ui.horizontal(|ui| {
                                            if !rule.campaign_name.is_empty() {
                                                ui.label(
                                                    RichText::new(format!("Campanha: {}", rule.campaign_name))
                                                        .size(11.0)
                                                        .color(palette.dim),
                                                );
                                            }
                                            if !rule.ad_name.is_empty() {
                                                ui.label(
                                                    RichText::new(format!("• Anúncio: {}", rule.ad_name))
                                                        .size(11.0)
                                                        .color(palette.dim),
                                                );
                                            }
                                            if let Some(ad_id) = &rule.ad_id {
                                                ui.label(
                                                    RichText::new(format!("• ID: {ad_id}"))
                                                        .size(11.0)
                                                        .color(palette.dim),
                                                );
                                            }
                                        });
                                    });

                                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui
                                            .button(RichText::new("Excluir").size(11.0).color(palette.danger))
                                            .clicked()
                                        {
                                            delete_id = Some(rule.id);
                                        }
                                    });
                                });
                            });
                        ui.add_space(4.0);
                    }

                    if let Some(id) = delete_id {
                        app.actions.push(Action::DeleteTextAttributionRule(id));
                    }
                });
        }
    });

    ui.ctx().data_mut(|d| d.insert_temp(form_id, form));
}
