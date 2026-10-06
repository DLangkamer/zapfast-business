//! Client sidecar panel (Mini-CRM): encrypted internal notes, pipeline stage selector,
//! deal value (R$), tags, and follow-up reminders attached to the active chat.

use egui::{vec2, Align, Color32, Layout, Margin, Rounding, Stroke, Vec2};

use crate::app::App;
use crate::model::{Action, Chat, CrmDeal, CrmFollowup};
use crate::theme::{self, Icon, Palette};

pub fn show(app: &mut App, ui: &mut egui::Ui, chat: &Chat) {
    let palette = app.palette;
    let deal = app
        .crm_deals
        .get(&chat.id)
        .cloned()
        .unwrap_or_else(|| CrmDeal {
            chat_id: chat.id.clone(),
            column_id: app
                .crm_columns
                .first()
                .map(|c| c.id.clone())
                .unwrap_or_else(|| "lead".to_owned()),
            value_cents: 0,
            notes: String::new(),
            tags: Vec::new(),
            updated_at: crate::util::now(),
        });

    egui::Frame::none()
        .fill(palette.panel)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(290.0);

            // Header: Title and Close button
            ui.horizontal(|ui| {
                theme::icon(ui, Icon::User, 18.0, palette.accent);
                theme::text(ui, "CRM do Contato", theme::semibold(15.0), palette.text);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if theme::icon_button(ui, Icon::X, 16.0, palette.secondary, palette.text, "Fechar painel").clicked() {
                        app.actions.push(Action::ToggleCrmSidecar);
                    }
                });
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 12.0;

                    // 1. Etapa do Funil (Pipeline Column)
                    render_stage_selector(app, ui, &palette, &deal);

                    // 2. Valor do Negócio (R$)
                    render_deal_value(app, ui, &palette, &deal);

                    // 3. Etiquetas / Tags
                    render_tags(app, ui, &palette, &deal);

                    // 4. Notas Internas
                    render_internal_notes(app, ui, &palette, &deal);

                    // 5. Lembretes de Follow-up
                    render_followups(app, ui, &palette, &chat.id);
                });
        });
}

fn render_stage_selector(app: &mut App, ui: &mut egui::Ui, palette: &Palette, deal: &CrmDeal) {
    ui.vertical(|ui| {
        theme::text(ui, "Etapa do Funil", theme::semibold(13.0), palette.text);
        ui.add_space(4.0);

        let columns = app.crm_columns.clone();
        for col in columns {
            let is_selected = deal.column_id == col.id;
            let (bg, border) = if is_selected {
                (palette.accent.gamma_multiply(0.2), palette.accent)
            } else {
                (palette.surface, palette.surface_hover)
            };

            let res = egui::Frame::none()
                .fill(bg)
                .stroke(Stroke::new(if is_selected { 1.5 } else { 1.0 }, border))
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let dot_color = parse_hex_color(&col.color).unwrap_or(palette.accent);
                        let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot_rect.center(), 4.0, dot_color);
                        ui.add_space(4.0);
                        theme::text(
                            ui,
                            &col.title,
                            if is_selected { theme::semibold(12.5) } else { theme::regular(12.5) },
                            if is_selected { palette.text } else { palette.secondary },
                        );
                        if is_selected {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                theme::icon(ui, Icon::Check, 14.0, palette.accent);
                            });
                        }
                    });
                })
                .response;

            if res.interact(egui::Sense::click()).clicked() && !is_selected {
                let mut updated = deal.clone();
                updated.column_id = col.id;
                updated.updated_at = crate::util::now();
                app.actions.push(Action::SaveCrmDeal(updated));
            }
        }
    });
}

fn render_deal_value(app: &mut App, ui: &mut egui::Ui, palette: &Palette, deal: &CrmDeal) {
    ui.vertical(|ui| {
        theme::text(ui, "Valor do Negócio", theme::semibold(13.0), palette.text);
        ui.add_space(4.0);

        let mut val_str = format!("{:.2}", deal.value_cents as f64 / 100.0);
        let id = ui.id().with("crm_deal_value");

        let response = ui.horizontal(|ui| {
            theme::text(ui, "R$", theme::semibold(13.0), palette.dim);
            ui.add(
                egui::TextEdit::singleline(&mut val_str)
                    .id(id)
                    .desired_width(120.0)
                    .hint_text("0,00"),
            )
        }).inner;

        if response.lost_focus() {
            let sanitized = val_str.replace(',', ".").replace(' ', "");
            if let Ok(num) = sanitized.parse::<f64>() {
                let cents = (num * 100.0).round() as i64;
                if cents != deal.value_cents {
                    let mut updated = deal.clone();
                    updated.value_cents = cents;
                    updated.updated_at = crate::util::now();
                    app.actions.push(Action::SaveCrmDeal(updated));
                }
            }
        }

        let formatted = format_currency(deal.value_cents);
        theme::text(ui, &format!("Atual: {formatted}"), theme::regular(11.5), palette.dim);
    });
}

fn render_tags(app: &mut App, ui: &mut egui::Ui, palette: &Palette, deal: &CrmDeal) {
    ui.vertical(|ui| {
        theme::text(ui, "Etiquetas do Contato", theme::semibold(13.0), palette.text);
        ui.add_space(4.0);

        // Render current tags
        if !deal.tags.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                for (idx, tag) in deal.tags.iter().enumerate() {
                    egui::Frame::none()
                        .fill(palette.surface_hover)
                        .rounding(Rounding::same(4.0))
                        .inner_margin(Margin::symmetric(6, 3))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                theme::text(ui, tag, theme::medium(11.5), palette.text);
                                let close = theme::icon_button(ui, Icon::X, 10.0, palette.dim, palette.danger, "Remover");
                                if close.clicked() {
                                    let mut updated = deal.clone();
                                    updated.tags.remove(idx);
                                    updated.updated_at = crate::util::now();
                                    app.actions.push(Action::SaveCrmDeal(updated));
                                }
                            });
                        });
                }
            });
            ui.add_space(4.0);
        }

        // Add tag row
        let tag_input_id = ui.id().with("crm_new_tag_input");
        let mut new_tag = ui.ctx().data(|d| d.get_temp::<String>(tag_input_id)).unwrap_or_default();

        ui.horizontal(|ui| {
            let res = ui.add(
                egui::TextEdit::singleline(&mut new_tag)
                    .hint_text("+ Nova tag...")
                    .desired_width(180.0),
            );

            let add_btn = ui.button("+ Adicionar");
            let enter_pressed = res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            if (add_btn.clicked() || enter_pressed) && !new_tag.trim().is_empty() {
                let trimmed = new_tag.trim().to_owned();
                if !deal.tags.contains(&trimmed) {
                    let mut updated = deal.clone();
                    updated.tags.push(trimmed);
                    updated.updated_at = crate::util::now();
                    app.actions.push(Action::SaveCrmDeal(updated));
                }
                new_tag.clear();
            }
            ui.ctx().data_mut(|d| d.insert_temp(tag_input_id, new_tag));
        });
    });
}

fn render_internal_notes(app: &mut App, ui: &mut egui::Ui, palette: &Palette, deal: &CrmDeal) {
    ui.vertical(|ui| {
        theme::text(ui, "Notas Internas (Criptografadas)", theme::semibold(13.0), palette.text);
        ui.add_space(4.0);

        let notes_id = ui.id().with("crm_internal_notes");
        let mut notes_draft = ui
            .ctx()
            .data(|d| d.get_temp::<String>(notes_id))
            .unwrap_or_else(|| deal.notes.clone());

        let res = ui.add(
            egui::TextEdit::multiline(&mut notes_draft)
                .desired_rows(4)
                .desired_width(ui.available_width())
                .hint_text("Adicione notas particulares sobre este cliente, reuniões, objeções..."),
        );

        if res.lost_focus() && notes_draft != deal.notes {
            let mut updated = deal.clone();
            updated.notes = notes_draft.clone();
            updated.updated_at = crate::util::now();
            app.actions.push(Action::SaveCrmDeal(updated));
        }

        ui.ctx().data_mut(|d| d.insert_temp(notes_id, notes_draft));
    });
}

fn render_followups(app: &mut App, ui: &mut egui::Ui, palette: &Palette, chat_id: &str) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            theme::text(ui, "Follow-ups e Cobranças", theme::semibold(13.0), palette.text);
        });
        ui.add_space(4.0);

        let now = crate::util::now();
        let followups: Vec<CrmFollowup> = app
            .crm_followups
            .iter()
            .filter(|f| f.chat_id == chat_id)
            .cloned()
            .collect();

        for f in &followups {
            let is_past = f.remind_at <= now;
            let (bg, text_col) = if f.done {
                (palette.surface, palette.dim)
            } else if is_past {
                (palette.danger.gamma_multiply(0.15), palette.danger)
            } else {
                (palette.surface_hover, palette.text)
            };

            egui::Frame::none()
                .fill(bg)
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::same(8))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let icon = if f.done {
                                Icon::CircleCheck
                            } else if is_past {
                                Icon::CircleAlert
                            } else {
                                Icon::Clock
                            };
                            theme::icon(ui, icon, 14.0, text_col);
                            theme::text(ui, &f.title, theme::medium(12.5), text_col);
                        });

                        ui.horizontal(|ui| {
                            let date_str = format_timestamp(f.remind_at);
                            theme::text(ui, &date_str, theme::regular(11.0), palette.dim);

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                // Delete button
                                if theme::icon_button(ui, Icon::Trash, 12.0, palette.dim, palette.danger, "Excluir").clicked() {
                                    app.actions.push(Action::DeleteCrmFollowup(f.id.clone()));
                                }

                                if !f.done {
                                    // Complete button
                                    if theme::icon_button(ui, Icon::Check, 12.0, palette.accent, palette.accent, "Concluir").clicked() {
                                        app.actions.push(Action::CompleteCrmFollowup(f.id.clone()));
                                    }
                                    // Quick Snooze +1d
                                    if theme::icon_button(ui, Icon::Clock, 12.0, palette.secondary, palette.text, "+1 dia").clicked() {
                                        let next_day = f.remind_at + 86400;
                                        app.actions.push(Action::SnoozeCrmFollowup { id: f.id.clone(), until: next_day });
                                    }
                                }
                            });
                        });
                    });
                });
        }

        // Quick add reminder section
        ui.add_space(4.0);
        let reminder_input_id = ui.id().with("crm_new_followup_title");
        let mut title = ui.ctx().data(|d| d.get_temp::<String>(reminder_input_id)).unwrap_or_default();

        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut title)
                    .hint_text("Novo lembrete (ex: Ligar amanhã)")
                    .desired_width(190.0),
            );
        });

        // Quick scheduling buttons
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let options = [
                ("+1h", now + 3600),
                ("Amanhã 09h", tomorrow_nine_am(now)),
                ("+2 dias", now + 172800),
                ("+1 semana", now + 604800),
            ];

            for (label, when) in options {
                if ui.button(label).clicked() {
                    let task_title = if title.trim().is_empty() {
                        "Retornar contato".to_owned()
                    } else {
                        title.trim().to_owned()
                    };

                    let new_id = format!("fu_{}_{}", crate::util::now(), rand_suffix());
                    let followup = CrmFollowup {
                        id: new_id,
                        chat_id: chat_id.to_owned(),
                        title: task_title,
                        remind_at: when,
                        done: false,
                        created_at: now,
                    };
                    app.actions.push(Action::SaveCrmFollowup(followup));
                    title.clear();
                }
            }
        });
        ui.ctx().data_mut(|d| d.insert_temp(reminder_input_id, title));
    });
}

pub fn format_currency(cents: i64) -> String {
    let reais = cents / 100;
    let centavos = (cents % 100).abs();
    let num_str = format!("{reais}");
    let mut with_dots = String::new();
    let len = num_str.len();
    for (i, ch) in num_str.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 && num_str.as_bytes()[0] != b'-' {
            with_dots.push('.');
        }
        with_dots.push(ch);
    }
    format!("R$ {with_dots},{centavos:02}")
}

pub fn parse_hex_color(hex: &str) -> Option<Color32> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color32::from_rgb(r, g, b))
    } else {
        None
    }
}

fn format_timestamp(timestamp: i64) -> String {
    if let Some((d, h, m)) = crate::util::local_datetime(timestamp) {
        format!("{:02}/{:02} às {:02}:{:02}", d.day(), d.month(), h, m)
    } else {
        "Data inválida".to_owned()
    }
}

fn tomorrow_nine_am(now: i64) -> i64 {
    crate::util::today()
        .tomorrow()
        .ok()
        .and_then(|d| crate::util::to_unix_seconds(d, 9, 0))
        .unwrap_or(now + 86400)
}

fn rand_suffix() -> u32 {
    (crate::util::now() % 10000) as u32
}
