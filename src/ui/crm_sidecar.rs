//! Client sidecar panel (Mini-CRM): encrypted internal notes, pipeline stage selector,
//! deal value (R$), tags, and follow-up reminders attached to the active chat.

use egui::{vec2, Align, Color32, CornerRadius, Frame, Layout, Margin, Stroke, Vec2};

use crate::app::App;
use crate::model::{Action, Chat, CrmDeal, CrmFollowup, Dialog};
use crate::theme::{self, Icon, Palette};

/// Renders the CRM sidecar as a native right-docked panel at the root UI level,
/// preventing overlap with the conversation messages and ensuring full interactivity.
pub fn show_panel(app: &mut App, ui: &mut egui::Ui, chat: &Chat) {
    let palette = app.palette;
    let width = 320.0;
    let panel = egui::Panel::right("crm_sidecar_panel")
        .resizable(false)
        .default_size(width)
        .size_range(width..=width)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.panel).inner_margin(Margin::ZERO));

    let response = panel.show(ui, |ui| {
        show(app, ui, chat);
    });

    // Draw subtle vertical separator line on the left edge of the sidecar
    let rect = response.response.rect;
    ui.painter().vline(
        rect.left(),
        rect.y_range(),
        Stroke::new(1.0, palette.outline),
    );
}

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

    ui.allocate_ui_with_layout(
        vec2(ui.available_width().max(0.0), ui.available_height().max(0.0)),
        Layout::top_down(Align::Min),
        |ui| {
            // Header with Contact info, Kanban shortcut and close button
            render_header(app, ui, &palette, chat);

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            // Scrollable CRM body
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 12.0;

                    // 1. Etapa do Funil (Pipeline Stage)
                    render_stage_selector(app, ui, &palette, &deal);

                    // 2. Valor do Negócio (R$)
                    render_deal_value(app, ui, &palette, &deal);

                    // 3. Etiquetas / Tags
                    render_tags(app, ui, &palette, &deal);

                    // 4. Notas Internas (Criptografadas)
                    render_internal_notes(app, ui, &palette, &deal);

                    // 5. Lembretes de Follow-up
                    render_followups(app, ui, &palette, &chat.id);

                    ui.add_space(20.0);
                });
        },
    );
}

fn render_header(app: &mut App, ui: &mut egui::Ui, palette: &Palette, chat: &Chat) {
    let title = app.chat_title(chat);
    let subtitle = if chat.is_group() {
        "Grupo".to_owned()
    } else {
        chat.id.trim_end_matches("@s.whatsapp.net").to_owned()
    };

    ui.horizontal(|ui| {
        // Contact icon / avatar badge
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 16.0, palette.surface_hover);
        let icon = if chat.is_group() { Icon::Users } else { Icon::User };
        theme::paint_icon(ui, icon, rect, 18.0, palette.accent);

        ui.add_space(4.0);

        ui.vertical(|ui| {
            theme::text(ui, &title, theme::semibold(14.0), palette.text);
            theme::text(ui, &subtitle, theme::regular(11.0), palette.dim);
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Close button
            if theme::icon_button(ui, Icon::X, 16.0, palette.dim, palette.text, "Fechar painel").clicked() {
                app.actions.push(Action::ToggleCrmSidecar);
            }

            // Quick shortcut to open Kanban
            if theme::icon_button(ui, Icon::ListChecks, 16.0, palette.secondary, palette.accent, "Ver no Funil").clicked() {
                app.actions.push(Action::ShowDialog(Dialog::Kanban));
            }
        });
    });
}

fn render_stage_selector(app: &mut App, ui: &mut egui::Ui, palette: &Palette, deal: &CrmDeal) {
    let columns = app.crm_columns.clone();
    let current_col = columns.iter().find(|c| c.id == deal.column_id);
    let current_title = current_col.map(|c| c.title.as_str()).unwrap_or("Selecione");
    let current_color = current_col
        .and_then(|c| parse_hex_color(&c.color))
        .unwrap_or(palette.accent);

    Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.surface_hover))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    theme::text(ui, "Etapa do Funil", theme::semibold(12.5), palette.text);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Current stage dot
                        let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot_rect.center(), 4.0, current_color);
                    });
                });

                ui.add_space(6.0);

                // ComboBox Dropdown
                egui::ComboBox::from_id_salt("sidecar_stage_selector")
                    .selected_text(current_title)
                    .width(ui.available_width() - 8.0)
                    .show_ui(ui, |ui| {
                        for col in &columns {
                            let dot_color = parse_hex_color(&col.color).unwrap_or(palette.accent);
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                                ui.painter().circle_filled(r.center(), 4.0, dot_color);
                                if ui.selectable_label(deal.column_id == col.id, &col.title).clicked() {
                                    let mut updated = deal.clone();
                                    updated.column_id = col.id.clone();
                                    updated.updated_at = crate::util::now();
                                    app.actions.push(Action::SaveCrmDeal(updated));
                                }
                            });
                        }
                    });

                ui.add_space(6.0);

                // Visual Pipeline Step Progress Bar
                if !columns.is_empty() {
                    let total_stages = columns.len();
                    let current_idx = columns.iter().position(|c| c.id == deal.column_id).unwrap_or(0);
                    let spacing = 3.0;
                    let bar_w = ((ui.available_width() - (total_stages as f32 - 1.0) * spacing) / total_stages as f32).max(4.0);

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = spacing;
                        for i in 0..total_stages {
                            let (seg_rect, _) = ui.allocate_exact_size(vec2(bar_w, 4.0), egui::Sense::hover());
                            let seg_color = if i <= current_idx {
                                current_color
                            } else {
                                palette.surface_hover
                            };
                            ui.painter().rect_filled(seg_rect, 2.0, seg_color);
                        }
                    });
                }
            });
        });
}

fn render_deal_value(app: &mut App, ui: &mut egui::Ui, palette: &Palette, deal: &CrmDeal) {
    Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.surface_hover))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    theme::text(ui, "Valor da Negociação", theme::semibold(12.5), palette.text);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let formatted = format_currency(deal.value_cents);
                        theme::text(ui, &formatted, theme::bold(13.5), palette.accent);
                    });
                });

                ui.add_space(6.0);

                let value_input_id = ui.id().with("sidecar_deal_value_input");
                let mut val_str = ui.ctx().data(|d| d.get_temp::<String>(value_input_id)).unwrap_or_else(|| {
                    if deal.value_cents > 0 {
                        format!("{:.2}", deal.value_cents as f64 / 100.0)
                    } else {
                        String::new()
                    }
                });

                ui.horizontal(|ui| {
                    theme::text(ui, "R$", theme::semibold(13.0), palette.dim);
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut val_str)
                            .hint_text("0,00")
                            .desired_width(ui.available_width() - 70.0),
                    );

                    let save_clicked = ui.button("Salvar").clicked();
                    let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                    if save_clicked || enter_pressed {
                        let sanitized = val_str.replace(',', ".").replace(' ', "");
                        let cents = if let Ok(num) = sanitized.parse::<f64>() {
                            (num * 100.0).round().max(0.0) as i64
                        } else {
                            0
                        };
                        if cents != deal.value_cents {
                            let mut updated = deal.clone();
                            updated.value_cents = cents;
                            updated.updated_at = crate::util::now();
                            app.actions.push(Action::SaveCrmDeal(updated));
                        }
                    }
                });
                ui.ctx().data_mut(|d| d.insert_temp(value_input_id, val_str));
            });
        });
}

fn render_tags(app: &mut App, ui: &mut egui::Ui, palette: &Palette, deal: &CrmDeal) {
    Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.surface_hover))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                theme::text(ui, "Etiquetas do Contato", theme::semibold(12.5), palette.text);
                ui.add_space(6.0);

                // Existing tags
                if !deal.tags.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                        for (idx, tag) in deal.tags.iter().enumerate() {
                            Frame::new()
                                .fill(palette.surface_hover)
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(6, 3))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        theme::text(ui, tag, theme::medium(11.5), palette.text);
                                        if theme::icon_button(ui, Icon::X, 10.0, palette.dim, palette.danger, "Remover").clicked() {
                                            let mut updated = deal.clone();
                                            updated.tags.remove(idx);
                                            updated.updated_at = crate::util::now();
                                            app.actions.push(Action::SaveCrmDeal(updated));
                                        }
                                    });
                                });
                        }
                    });
                    ui.add_space(6.0);
                }

                // Add tag row
                let tag_input_id = ui.id().with("sidecar_new_tag_input");
                let mut new_tag = ui.ctx().data(|d| d.get_temp::<String>(tag_input_id)).unwrap_or_default();

                ui.horizontal(|ui| {
                    let text_resp = ui.add(
                        egui::TextEdit::singleline(&mut new_tag)
                            .hint_text("+ Nova tag...")
                            .desired_width(ui.available_width() - 80.0),
                    );

                    let add_clicked = ui.button("+ Adicionar").clicked();
                    let enter_pressed = text_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                    if (add_clicked || enter_pressed) && !new_tag.trim().is_empty() {
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
        });
}

fn render_internal_notes(app: &mut App, ui: &mut egui::Ui, palette: &Palette, deal: &CrmDeal) {
    Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.surface_hover))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::Lock, 13.0, palette.secondary);
                    theme::text(ui, "Notas Internas (Criptografadas)", theme::semibold(12.5), palette.text);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        theme::text(ui, "✓ Salvo", theme::regular(10.5), palette.dim);
                    });
                });

                ui.add_space(4.0);

                let notes_id = ui.id().with("sidecar_internal_notes");
                let mut notes_draft = ui
                    .ctx()
                    .data(|d| d.get_temp::<String>(notes_id))
                    .unwrap_or_else(|| deal.notes.clone());

                let res = ui.add(
                    egui::TextEdit::multiline(&mut notes_draft)
                        .desired_rows(4)
                        .desired_width(ui.available_width())
                        .hint_text("Adicione anotações particulares sobre este cliente, acordos, reuniões..."),
                );

                if res.lost_focus() && notes_draft != deal.notes {
                    let mut updated = deal.clone();
                    updated.notes = notes_draft.clone();
                    updated.updated_at = crate::util::now();
                    app.actions.push(Action::SaveCrmDeal(updated));
                }

                ui.ctx().data_mut(|d| d.insert_temp(notes_id, notes_draft));
            });
        });
}

fn render_followups(app: &mut App, ui: &mut egui::Ui, palette: &Palette, chat_id: &str) {
    Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.surface_hover))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::Clock, 14.0, palette.accent);
                    theme::text(ui, "Follow-ups e Cobranças", theme::semibold(12.5), palette.text);
                });

                ui.add_space(6.0);

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
                        (palette.surface_hover, palette.dim)
                    } else if is_past {
                        (palette.danger.gamma_multiply(0.18), palette.danger)
                    } else {
                        (palette.panel, palette.text)
                    };

                    Frame::new()
                        .fill(bg)
                        .corner_radius(CornerRadius::same(6))
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
                                    theme::icon(ui, icon, 13.0, text_col);
                                    theme::text(ui, &f.title, theme::medium(12.0), text_col);
                                });

                                ui.horizontal(|ui| {
                                    let date_str = format_timestamp(f.remind_at);
                                    theme::text(ui, &date_str, theme::regular(10.5), palette.dim);

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if theme::icon_button(ui, Icon::Trash, 12.0, palette.dim, palette.danger, "Excluir").clicked() {
                                            app.actions.push(Action::DeleteCrmFollowup(f.id.clone()));
                                        }

                                        if !f.done {
                                            if theme::icon_button(ui, Icon::Check, 12.0, palette.accent, palette.accent, "Concluir").clicked() {
                                                app.actions.push(Action::CompleteCrmFollowup(f.id.clone()));
                                            }
                                            if ui.small_button("+1d").on_hover_text("Adiar 1 dia").clicked() {
                                                let next_day = f.remind_at + 86400;
                                                app.actions.push(Action::SnoozeCrmFollowup { id: f.id.clone(), until: next_day });
                                            }
                                        }
                                    });
                                });
                            });
                        });
                    ui.add_space(4.0);
                }

                // Add reminder section
                ui.add_space(4.0);
                let reminder_input_id = ui.id().with("sidecar_new_followup_title");
                let mut title = ui.ctx().data(|d| d.get_temp::<String>(reminder_input_id)).unwrap_or_default();

                ui.add(
                    egui::TextEdit::singleline(&mut title)
                        .hint_text("Lembrete (ex: Ligar para fechar)")
                        .desired_width(ui.available_width()),
                );

                ui.add_space(4.0);

                // 2x2 grid of quick schedule buttons
                let half_w = (ui.available_width() - 6.0) / 2.0;
                ui.horizontal(|ui| {
                    if ui.add_sized(vec2(half_w, 24.0), egui::Button::new("⏱ +1 hora")).clicked() {
                        create_followup(app, chat_id, &title, now + 3600);
                        title.clear();
                    }
                    if ui.add_sized(vec2(half_w, 24.0), egui::Button::new("🌅 Amanhã 09h")).clicked() {
                        create_followup(app, chat_id, &title, tomorrow_nine_am(now));
                        title.clear();
                    }
                });
                ui.horizontal(|ui| {
                    if ui.add_sized(vec2(half_w, 24.0), egui::Button::new("📅 +2 dias")).clicked() {
                        create_followup(app, chat_id, &title, now + 172800);
                        title.clear();
                    }
                    if ui.add_sized(vec2(half_w, 24.0), egui::Button::new("🗓 +1 semana")).clicked() {
                        create_followup(app, chat_id, &title, now + 604800);
                        title.clear();
                    }
                });

                ui.ctx().data_mut(|d| d.insert_temp(reminder_input_id, title));
            });
        });
}

fn create_followup(app: &mut App, chat_id: &str, title: &str, when: i64) {
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
        created_at: crate::util::now(),
    };
    app.actions.push(Action::SaveCrmFollowup(followup));
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
